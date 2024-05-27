pub struct Hasher();

impl Hasher {
    pub fn new() -> Self {
        Self()
    }

    pub fn lookup(&self, key: &str) -> u32 {
        (key.len() as u32).wrapping_add(key.as_bytes()[0] as u32)
    }
}
