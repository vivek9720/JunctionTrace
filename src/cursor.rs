use crate::error::{JunctionError, Result};

#[derive(Clone)]
pub struct Cursor<'a> {
    data: &'a [u8],
    offset: usize,
}

impl<'a> Cursor<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, offset: 0 }
    }

    pub fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.offset)
    }

    pub fn position(&self) -> usize {
        self.offset
    }

    pub fn is_empty(&self) -> bool {
        self.remaining() == 0
    }

    pub fn read_u8(&mut self) -> Result<u8> {
        if self.remaining() < 1 {
            return Err(JunctionError::Truncated {
                needed: 1,
                remaining: self.remaining(),
            });
        }
        let out = self.data[self.offset];
        self.offset += 1;
        Ok(out)
    }

    pub fn read_i8(&mut self) -> Result<i8> {
        Ok(self.read_u8()? as i8)
    }

    pub fn read_u16(&mut self) -> Result<u16> {
        let bytes = self.read_bytes(2)?;
        Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
    }

    pub fn read_i16(&mut self) -> Result<i16> {
        Ok(self.read_u16()? as i16)
    }

    pub fn read_u24(&mut self) -> Result<u32> {
        let bytes = self.read_bytes(3)?;
        Ok(bytes[0] as u32 | ((bytes[1] as u32) << 8) | ((bytes[2] as u32) << 16))
    }

    pub fn read_u32(&mut self) -> Result<u32> {
        let bytes = self.read_bytes(4)?;
        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    pub fn read_u64(&mut self) -> Result<u64> {
        let bytes = self.read_bytes(8)?;
        Ok(u64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }

    pub fn read_bytes(&mut self, len: usize) -> Result<&'a [u8]> {
        if self.remaining() < len {
            return Err(JunctionError::Truncated {
                needed: len,
                remaining: self.remaining(),
            });
        }
        let start = self.offset;
        self.offset += len;
        Ok(&self.data[start..start + len])
    }

    pub fn read_vec(&mut self, len: usize) -> Result<Vec<u8>> {
        Ok(self.read_bytes(len)?.to_vec())
    }

    pub fn read_text(&mut self, len: usize) -> Result<String> {
        let bytes = self.read_bytes(len)?;
        core::str::from_utf8(bytes)
            .map(|s| s.to_string())
            .map_err(|_| JunctionError::InvalidUtf8)
    }

    pub fn read_var_u32(&mut self) -> Result<u32> {
        let mut value = 0u32;
        let mut shift = 0;
        for _ in 0..5 {
            let byte = self.read_u8()?;
            value |= ((byte & 0x7f) as u32) << shift;
            if byte & 0x80 == 0 {
                return Ok(value);
            }
            shift += 7;
        }
        Err(JunctionError::InvalidVarint)
    }

    pub fn take_until_end(&mut self) -> &'a [u8] {
        let start = self.offset;
        self.offset = self.data.len();
        &self.data[start..]
    }

    pub fn fork_for(&mut self, len: usize) -> Result<Cursor<'a>> {
        let bytes = self.read_bytes(len)?;
        Ok(Cursor::new(bytes))
    }
}
