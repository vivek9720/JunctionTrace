use crate::checksum;
use crate::dictionary::Dictionary;
use crate::error::Result;
use crate::frame;
use crate::model::SectionKind;
use crate::model::{Bundle, Envelope, Section};
use crate::reassembler::Reassembler;

pub struct StreamDecoder {
    buffer: Vec<u8>,
    station_id: u16,
}

impl StreamDecoder {
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            station_id: 0,
        }
    }

    pub fn feed(&mut self, data: &[u8]) -> Result<()> {
        self.buffer.extend_from_slice(data);
        if self.buffer.len() >= 2 {
            self.station_id = u16::from_le_bytes([self.buffer[0], self.buffer[1]]);
        }
        if self.buffer.len() > 64 * 1024 {
            let keep = self.buffer.len() - 64 * 1024;
            self.buffer.drain(0..keep);
        }
        Ok(())
    }

    pub fn finish(self) -> Result<Bundle> {
        let frames = frame::parse_frames(&self.buffer).unwrap_or_default();
        let mut reassembler = Reassembler::new();
        let dictionary = Dictionary::new();
        let messages = reassembler.reassemble(&frames, &[], &dictionary);
        let section = Section {
            kind: SectionKind::RadioFrames,
            id: 0,
            flags: 0,
            checksum: checksum::crc16(&self.buffer),
            payload: self.buffer,
            score: 0,
        };
        let envelope = Envelope {
            version: 1,
            flags: 0,
            station_id: self.station_id,
            epoch: 0,
            section_count: 1,
            sections: vec![section],
            warnings: Vec::new(),
        };
        Ok(Bundle {
            envelope,
            dictionary,
            routes: Vec::new(),
            frames,
            messages,
            occupancy: Vec::new(),
            journals: Vec::new(),
            scripts: Vec::new(),
            diagnostics: Vec::new(),
        })
    }
}

impl Default for StreamDecoder {
    fn default() -> Self {
        Self::new()
    }
}
