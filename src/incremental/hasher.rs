use std::hash::Hasher;

const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;

pub struct FxHasher {
    state: u64,
}

impl FxHasher {
    pub fn new() -> Self {
        FxHasher {
            state: FNV_OFFSET_BASIS,
        }
    }
}

impl Default for FxHasher {
    fn default() -> Self {
        Self::new()
    }
}

impl Hasher for FxHasher {
    fn finish(&self) -> u64 {
        self.state
    }

    fn write(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.state = self.state.wrapping_mul(FNV_PRIME) ^ (byte as u64);
        }
    }

    fn write_u8(&mut self, i: u8) {
        self.state = self.state.wrapping_mul(FNV_PRIME) ^ (i as u64);
    }

    fn write_u16(&mut self, i: u16) {
        self.write(&i.to_le_bytes());
    }

    fn write_u32(&mut self, i: u32) {
        self.write(&i.to_le_bytes());
    }

    fn write_u64(&mut self, i: u64) {
        self.write(&i.to_le_bytes());
    }

    fn write_usize(&mut self, i: usize) {
        self.write(&i.to_le_bytes());
    }
}
