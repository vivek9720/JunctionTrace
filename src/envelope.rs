use crate::checksum;
use crate::cursor::Cursor;
use crate::dictionary::Dictionary;
use crate::error::{JunctionError, Result};
use crate::frame;
use crate::journal;
use crate::model::{Bundle, Envelope, Section, SectionKind};
use crate::occupancy;
use crate::reassembler::Reassembler;
use crate::route;
use crate::script;

pub struct BundleDecoder {
    max_sections: usize,
    max_section_len: usize,
}

impl BundleDecoder {
    pub fn new() -> Self {
        Self {
            max_sections: 64,
            max_section_len: 16 * 1024,
        }
    }

    pub fn parse(&self, data: &[u8]) -> Result<Bundle> {
        let envelope = self.parse_envelope(data)?;
        let mut dictionary = Dictionary::new();
        for section in &envelope.sections {
            if section.kind == SectionKind::Dictionary {
                dictionary.parse_page(&section.payload)?;
            }
        }

        let mut routes = Vec::new();
        let mut frames = Vec::new();
        let mut occupancy_batches = Vec::new();
        let mut journals = Vec::new();
        let mut scripts = Vec::new();
        let mut diagnostics = envelope.warnings.clone();

        for section in &envelope.sections {
            match section.kind {
                SectionKind::RoutePlan => {
                    match route::parse_route_plans(&section.payload, &dictionary) {
                        Ok(mut parsed) => routes.append(&mut parsed),
                        Err(e) => diagnostics.push(format!("route section {}: {e}", section.id)),
                    }
                }
                SectionKind::RadioFrames | SectionKind::Transcript => {
                    match frame::parse_frames(&section.payload) {
                        Ok(mut parsed) => frames.append(&mut parsed),
                        Err(e) => diagnostics.push(format!("frame section {}: {e}", section.id)),
                    }
                }
                SectionKind::Occupancy => {
                    match occupancy::parse_occupancy_batch(&section.payload, &dictionary) {
                        Ok(batch) => occupancy_batches.push(batch),
                        Err(e) => {
                            diagnostics.push(format!("occupancy section {}: {e}", section.id))
                        }
                    }
                }
                SectionKind::Journal => {
                    match journal::parse_journal(&section.payload, &dictionary) {
                        Ok(j) => journals.push(j),
                        Err(e) => diagnostics.push(format!("journal section {}: {e}", section.id)),
                    }
                }
                SectionKind::Script => {
                    match script::parse_script_program(&section.payload, &dictionary) {
                        Ok(p) => scripts.push(p),
                        Err(e) => diagnostics.push(format!("script section {}: {e}", section.id)),
                    }
                }
                _ => {}
            }
        }

        let mut reassembler = Reassembler::new();
        let messages = reassembler.reassemble(&frames, &routes, &dictionary);

        Ok(Bundle {
            envelope,
            dictionary,
            routes,
            frames,
            messages,
            occupancy: occupancy_batches,
            journals,
            scripts,
            diagnostics,
        })
    }

    pub fn parse_envelope(&self, data: &[u8]) -> Result<Envelope> {
        let mut c = Cursor::new(data);
        if c.remaining() < 12 {
            return Err(JunctionError::Truncated {
                needed: 12,
                remaining: c.remaining(),
            });
        }
        let magic = c.read_bytes(4)?;
        if magic != b"JTRC" {
            return Err(JunctionError::BadMagic);
        }
        let version = c.read_u8()?;
        if version == 0 || version > 3 {
            return Err(JunctionError::UnsupportedVersion(version));
        }
        let flags = c.read_u8()?;
        let station_id = c.read_u16()?;
        let epoch = c.read_u32()?;
        let section_count = c.read_u8()?;
        let _header_flags = c.read_u8()?;
        let _reserved = c.read_u16()?;
        let mut sections = Vec::new();
        let mut warnings = Vec::new();

        let wanted = (section_count as usize).min(self.max_sections);
        for _ in 0..wanted {
            if c.remaining() < 8 {
                break;
            }
            let raw_kind = c.read_u8()?;
            let id = c.read_u8()?;
            let flags = c.read_u16()?;
            let len = c.read_u16()? as usize;
            let checksum = c.read_u16()?;
            if len > self.max_section_len {
                return Err(JunctionError::LimitExceeded("section payload"));
            }
            if c.remaining() < len {
                break;
            }
            let payload = c.read_vec(len)?;
            let calc = checksum::crc16(&payload);
            if calc != checksum && flags & 0x0001 != 0 {
                warnings.push(format!(
                    "section {id} checksum mismatch {checksum:04x}/{calc:04x}"
                ));
            }
            let score = checksum::section_score(raw_kind, id, flags, &payload);
            sections.push(Section {
                kind: SectionKind::from_byte(raw_kind),
                id,
                flags,
                checksum,
                payload,
                score,
            });
        }

        Ok(Envelope {
            version,
            flags,
            station_id,
            epoch,
            section_count,
            sections,
            warnings,
        })
    }
}

impl Default for BundleDecoder {
    fn default() -> Self {
        Self::new()
    }
}
