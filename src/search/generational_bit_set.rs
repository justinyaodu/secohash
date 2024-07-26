type Word = u8;

pub struct GenerationalBitSet {
    generation: Word,
    values: Vec<Word>,
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

    pub fn clear_all(&mut self) {
        if self.generation == Word::MAX {
            self.generation = 1;
            self.values.fill(0);
        } else {
            self.generation += 1;
        }
    }
}
