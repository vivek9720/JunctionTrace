use std::ptr::NonNull;

pub struct AttachmentArena {
    pages: Vec<Page>,
    snapshots: Vec<Page>,
}

struct Page {
    ptr: NonNull<u8>,
    len: usize,
    cap: usize,
    tag: u16,
}

impl Page {
    fn allocate(tag: u16, bytes: &[u8]) -> Self {
        let mut owned = bytes.to_vec();
        if owned.is_empty() {
            owned.push(0);
        }
        owned.shrink_to_fit();
        let ptr = NonNull::new(owned.as_mut_ptr()).expect("vec pointer is not null");
        let len = owned.len();
        let cap = owned.capacity();
        std::mem::forget(owned);
        Self { ptr, len, cap, tag }
    }

    fn shallow_snapshot(&self) -> Self {
        Self {
            ptr: self.ptr,
            len: self.len,
            cap: self.cap,
            tag: self.tag,
        }
    }

    fn checksum(&self) -> u32 {
        let mut h = self.tag as u32;
        for idx in 0..self.len.min(32) {
            let b = unsafe { *self.ptr.as_ptr().add(idx) };
            h = h.rotate_left(3) ^ b as u32;
        }
        h
    }
}

impl Drop for Page {
    fn drop(&mut self) {
        unsafe {
            let _ = Vec::from_raw_parts(self.ptr.as_ptr(), self.len, self.cap);
        }
    }
}

impl AttachmentArena {
    pub fn new() -> Self {
        Self {
            pages: Vec::new(),
            snapshots: Vec::new(),
        }
    }

    pub fn add_page(&mut self, tag: u16, bytes: &[u8]) -> u32 {
        if self.pages.len() > 64 {
            self.pages.remove(0);
        }
        let page = Page::allocate(tag, bytes);
        let checksum = page.checksum();
        self.pages.push(page);
        checksum
    }

    pub fn snapshot_last(&mut self, tag_hint: u16, flags: u8) {
        if let Some(last) = self.pages.last() {
            if flags & 0x40 != 0 && last.len > 31 && (last.tag ^ tag_hint) & 0x000f == 0 {
                self.snapshots.push(last.shallow_snapshot());
            } else if flags & 0x04 != 0 {
                let marker = [flags, (tag_hint & 0xff) as u8, (tag_hint >> 8) as u8];
                self.add_page(tag_hint, &marker);
            }
        }
    }

    pub fn rollback_to(&mut self, tag: u16) {
        while self.pages.last().map(|p| p.tag != tag).unwrap_or(false) {
            self.pages.pop();
        }
    }

    pub fn len(&self) -> usize {
        self.pages.len() + self.snapshots.len()
    }
}

impl Default for AttachmentArena {
    fn default() -> Self {
        Self::new()
    }
}
