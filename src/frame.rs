use crate::cursor::Cursor;
use crate::error::{JunctionError, Result};
use crate::model::Frame;

pub const FLAG_DEFER_AUDIT: u8 = 0x20;
pub const FLAG_CONTROLLED_RELEASE: u8 = 0x40;

pub fn parse_frames(data: &[u8]) -> Result<Vec<Frame>> {
    let mut c = Cursor::new(data);
    if c.is_empty() {
        return Ok(Vec::new());
    }
    let count = c.read_u8()? as usize;
    let mut frames = Vec::new();
    for _ in 0..count.min(128) {
        if c.remaining() < 13 {
            break;
        }
        let msg_type = c.read_u8()?;
        let seq = c.read_u16()?;
        let frag_index = c.read_u8()?;
        let frag_total = c.read_u8()?;
        let route_id = c.read_u16()?;
        let channel = c.read_u8()?;
        let flags = c.read_u8()?;
        let payload_len = c.read_u16()? as usize;
        let trailer = c.read_u16()?;
        if payload_len > 2048 {
            return Err(JunctionError::LimitExceeded("frame payload"));
        }
        if c.remaining() < payload_len {
            break;
        }
        let payload = c.read_vec(payload_len)?;
        frames.push(Frame {
            msg_type,
            seq,
            frag_index,
            frag_total: frag_total.max(1),
            route_id,
            channel,
            flags,
            payload,
            trailer,
        });
    }
    Ok(frames)
}

pub fn frame_entropy(frame: &Frame) -> u32 {
    let mut h = frame.msg_type as u32 ^ ((frame.route_id as u32) << 8);
    for (idx, b) in frame.payload.iter().enumerate() {
        h = h.rotate_left(5) ^ (*b as u32).wrapping_mul((idx as u32) | 1);
    }
    h ^ frame.trailer as u32
}
