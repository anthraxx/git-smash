use std::hash::{BuildHasher, Hasher};
use std::io::Write;

#[derive(Default)]
pub struct IdentityHasher {
    off: u8,
    buf: [u8; 8],
}

impl Hasher for IdentityHasher {
    fn finish(&self) -> u64 {
        u64::from_ne_bytes(self.buf)
    }

    fn write(&mut self, bytes: &[u8]) {
        self.off += (&mut self.buf[self.off as usize..])
            .write(bytes)
            .unwrap_or(0) as u8;
    }
}

pub fn hash<T: BuildHasher, U: std::hash::Hash + ?Sized>(build: &T, v: &U) -> u64 {
    let mut s = build.build_hasher();
    v.hash(&mut s);
    s.finish()
}
