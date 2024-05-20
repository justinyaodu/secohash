pub struct Hasher();

impl Hasher {
    pub fn new() -> Self {
        Self()
    }

    pub fn lookup(&self, key: &str) -> u64 {
        (key.len() as u64) + (key.as_bytes()[0] as u64)
    }
}
