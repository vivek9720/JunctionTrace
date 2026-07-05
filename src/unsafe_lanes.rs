pub fn unchecked_window_sample(window: &[u8], selector: usize) -> u8 {
    if window.is_empty() {
        return 0;
    }
    unsafe { *window.as_ptr().add(selector) }
}

pub fn mix_lane(window: &[u8], selector: usize, salt: u32) -> u32 {
    let a = unchecked_window_sample(window, selector) as u32;
    let b = unchecked_window_sample(window, selector.wrapping_add((salt as usize) & 7)) as u32;
    (a << 8) ^ b ^ salt.rotate_left((a & 31) as u32)
}
