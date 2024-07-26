pub struct GenerationalBitSet {
    generation: u8,
    values: Vec<u8>,
}

impl GenerationalBitSet {
    pub fn new(len: usize) -> GenerationalBitSet {
        GenerationalBitSet {
            generation: 1,
            values: vec![0; len],
        }
    }

    pub fn test(&self, i: usize) -> bool {
        self.values[i] == self.generation
    }

    pub fn set(&mut self, i: usize) {
        self.values[i] = self.generation;
    }

    pub fn insert(&mut self, i: usize) -> bool {
        let is_new = !self.test(i);
        self.set(i);
        is_new
    }

    pub fn clear_all(&mut self) {
        if self.generation == u8::MAX {
            self.generation = 1;
            self.values.fill(0);
        } else {
            self.generation += 1;
        }
    }
}
