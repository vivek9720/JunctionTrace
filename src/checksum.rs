pub fn crc16(data: &[u8]) -> u16 {
    let mut crc = 0x5a5au16;
    for &b in data {
        crc ^= b as u16;
        for _ in 0..8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ 0xa001;
            } else {
                crc >>= 1;
            }
        }
    }
    crc
}

pub fn rolling32(seed: u32, data: &[u8]) -> u32 {
    let mut h = seed ^ 0x9e37_79b9;
    for (i, &b) in data.iter().enumerate() {
        h = h.rotate_left(5) ^ (b as u32).wrapping_mul(0x45d9_f3b);
        h = h.wrapping_add((i as u32).rotate_left((b & 15) as u32));
    }
    h
}

pub fn section_score(kind: u8, id: u8, flags: u16, payload: &[u8]) -> u32 {
    let mut h = rolling32(
        kind as u32 | ((id as u32) << 8) | ((flags as u32) << 16),
        payload,
    );
    h ^= (payload.len() as u32).rotate_left((kind & 31) as u32);
    h
}
