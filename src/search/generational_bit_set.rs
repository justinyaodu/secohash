use std::collections::HashSet;

use crate::util::to_usize;

pub trait BitSet {
    fn test(&self, i: u32) -> bool;
    fn set(&mut self, i: u32);
    fn clear(&mut self);
}

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
}

impl BitSet for GenerationalBitSet {
    fn test(&self, i: u32) -> bool {
        self.values[to_usize(i)] == self.generation
    }

    fn set(&mut self, i: u32) {
        self.values[to_usize(i)] = self.generation;
    }

    fn clear(&mut self) {
        if self.generation == Word::MAX {
            self.generation = 1;
            self.values.fill(0);
        } else {
            self.generation += 1;
        }
    }
}

impl BitSet for HashSet<u32> {
    fn test(&self, i: u32) -> bool {
        self.contains(&i)
    }

    fn set(&mut self, i: u32) {
        self.insert(i);
    }

    fn clear(&mut self) {
        self.clear()
    }
}
