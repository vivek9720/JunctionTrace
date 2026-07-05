use crate::attachment::AttachmentArena;
use crate::cursor::Cursor;
use crate::dictionary::{Dictionary, DictionaryClass};
use crate::error::{JunctionError, Result};
use crate::model::{Journal, JournalRecord};

pub fn parse_journal(data: &[u8], dict: &Dictionary) -> Result<Journal> {
    let mut c = Cursor::new(data);
    if c.remaining() < 3 {
        return Err(JunctionError::Truncated {
            needed: 3,
            remaining: c.remaining(),
        });
    }
    let source_id = c.read_u16()?;
    let count = c.read_u8()? as usize;
    if count > 128 {
        return Err(JunctionError::LimitExceeded("journal records"));
    }
    let mut arena = AttachmentArena::new();
    let mut records = Vec::new();
    let mut committed = false;
    for _ in 0..count {
        if c.remaining() < 6 {
            break;
        }
        let opcode = c.read_u8()?;
        let tag = c.read_u16()?;
        let flags = c.read_u8()?;
        let payload_len = c.read_u16()? as usize;
        if payload_len > 4096 {
            return Err(JunctionError::LimitExceeded("journal payload"));
        }
        if c.remaining() < payload_len {
            break;
        }
        let payload = c.read_bytes(payload_len)?;
        let note = note_for(opcode, tag, payload, dict);
        match opcode {
            0x01 => {
                arena.add_page(tag, payload);
            }
            0x02 => {
                arena.snapshot_last(tag, flags);
            }
            0x03 => {
                arena.rollback_to(tag);
            }
            0x7f => committed = true,
            _ => {
                if flags & 0x20 != 0 {
                    arena.add_page(tag ^ source_id, payload);
                }
            }
        }
        records.push(JournalRecord {
            opcode,
            tag,
            flags,
            payload_len,
            note,
        });
    }
    let attachments_seen = arena.len();
    Ok(Journal {
        source_id,
        records,
        attachments_seen,
        committed,
    })
}

fn note_for(opcode: u8, tag: u16, payload: &[u8], dict: &Dictionary) -> String {
    if let Some(entry) = dict.lookup_class(tag, DictionaryClass::Maintainer) {
        return format!("{}:{opcode:02x}:{}", entry.name, payload.len());
    }
    if payload.windows(4).any(|w| w == b"hold") {
        format!("hold request {tag:04x}")
    } else if opcode == 0x7f {
        "commit".to_string()
    } else {
        format!("record {opcode:02x}/{tag:04x}")
    }
}
