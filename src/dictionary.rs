use crate::cursor::Cursor;
use crate::error::{JunctionError, Result};

#[derive(Clone, Debug, Default)]
pub struct Dictionary {
    entries: Vec<DictionaryEntry>,
}

#[derive(Clone, Debug)]
pub struct DictionaryEntry {
    pub id: u16,
    pub class: DictionaryClass,
    pub flags: u8,
    pub name: String,
    pub value: Vec<u8>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DictionaryClass {
    Circuit,
    Signal,
    Route,
    Maintainer,
    Phrase,
    Vendor(u8),
}

impl DictionaryClass {
    pub fn from_byte(byte: u8) -> Self {
        match byte {
            1 => DictionaryClass::Circuit,
            2 => DictionaryClass::Signal,
            3 => DictionaryClass::Route,
            4 => DictionaryClass::Maintainer,
            5 => DictionaryClass::Phrase,
            other => DictionaryClass::Vendor(other),
        }
    }
}

impl Dictionary {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn parse_page(&mut self, data: &[u8]) -> Result<()> {
        let mut c = Cursor::new(data);
        if c.is_empty() {
            return Ok(());
        }
        let count = c.read_u8()? as usize;
        for _ in 0..count.min(96) {
            if c.remaining() < 7 {
                break;
            }
            let id = c.read_u16()?;
            let class = DictionaryClass::from_byte(c.read_u8()?);
            let flags = c.read_u8()?;
            let name_len = c.read_u8()? as usize;
            let value_len = c.read_u16()? as usize;
            if name_len > 96 || value_len > 512 {
                return Err(JunctionError::LimitExceeded("dictionary entry"));
            }
            if c.remaining() < name_len + value_len {
                break;
            }
            let name = c.read_text(name_len)?;
            let value = c.read_vec(value_len)?;
            self.upsert(DictionaryEntry {
                id,
                class,
                flags,
                name,
                value,
            });
        }
        Ok(())
    }

    fn upsert(&mut self, entry: DictionaryEntry) {
        if let Some(existing) = self
            .entries
            .iter_mut()
            .find(|e| e.id == entry.id && e.class == entry.class)
        {
            *existing = entry;
        } else {
            self.entries.push(entry);
        }
    }

    pub fn lookup(&self, id: u16) -> Option<&DictionaryEntry> {
        self.entries.iter().find(|e| e.id == id)
    }

    pub fn lookup_class(&self, id: u16, class: DictionaryClass) -> Option<&DictionaryEntry> {
        self.entries.iter().find(|e| e.id == id && e.class == class)
    }

    pub fn phrase_contains(&self, needle: &[u8]) -> bool {
        self.entries.iter().any(|entry| {
            entry.class == DictionaryClass::Phrase
                && entry.value.windows(needle.len()).any(|w| w == needle)
        })
    }

    pub fn route_bias(&self, route_id: u16) -> u32 {
        let mut bias = route_id as u32;
        for entry in &self.entries {
            if entry.id == route_id
                || entry
                    .value
                    .windows(2)
                    .any(|w| u16::from_le_bytes([w[0], w[1]]) == route_id)
            {
                bias = bias.rotate_left((entry.flags & 15) as u32) ^ entry.value.len() as u32;
            }
        }
        bias
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn entries(&self) -> &[DictionaryEntry] {
        &self.entries
    }
}
