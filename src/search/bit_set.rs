pub struct BitSet(Vec<u32>);

impl BitSet {
    pub fn new(len: usize) -> BitSet {
        BitSet(vec![0; (len + 31) / 32])
    }

    pub fn test(&self, i: usize) -> bool {
        (self.0[i >> 5] & (1 << (i & 31))) != 0
    }

    pub fn set(&mut self, i: usize) {
        self.0[i >> 5] |= 1 << (i & 31);
    }

    pub fn insert(&mut self, i: usize) -> bool {
        let new = !self.test(i);
        self.set(i);
        new
    }
}
