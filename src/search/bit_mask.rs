pub struct BitMask {
    len: usize,
    words: Vec<u64>,
}

impl BitMask {
    pub fn new(len: usize) -> BitMask {
        BitMask {
            len,
            words: vec![0; (len + 63) / 64],
        }
    }

    fn word_index_and_mask(i: usize) -> (usize, u64) {
        (i / 64, 1 << (i % 64))
    }

    pub fn test(&self, i: usize) -> bool {
        let (word_index, mask) = Self::word_index_and_mask(i);
        (self.words[word_index] & mask) != 0
    }

    pub fn set(&mut self, i: usize) {
        let (word_index, mask) = Self::word_index_and_mask(i);
        self.words[word_index] |= mask;
    }

    pub fn clear_all(&mut self) {
        self.words.fill(0);
    }

    pub fn should_use_disjoint(&self, other_popcnt: usize) -> bool {
        self.len % 64 == 0 && 8 * self.words.len() < other_popcnt
    }

    pub fn disjoint(&self, other: &BitMask, rotation: usize) -> bool {
        // eprintln!();
        // eprintln!("self  = {}", self.words.iter().rev().map(|w| format!("{w:064b}")).collect::<Vec<_>>().join(" "));
        // eprintln!("other = {}", other.words.iter().rev().map(|w| format!("{w:064b}")).collect::<Vec<_>>().join(" "));
        let ret = self.disjoint_inner(other, rotation);
        // eprintln!("disjoint for rotation={rotation}: {ret}");
        let self_bools = self.to_bools();
        let mut other_bools = other.to_bools();
        other_bools.rotate_right(rotation);
        let expected_ret = !self_bools
            .iter()
            .copied()
            .zip(other_bools.iter().copied())
            .any(|(a, b)| a && b);
        if ret != expected_ret {
            eprintln!(
                "self_bools  = {}",
                self_bools
                    .iter()
                    .rev()
                    .map(|&b| ["0", "1"][b as usize])
                    .collect::<Vec<_>>()
                    .join("")
            );
            eprintln!(
                "other_bools = {}",
                other_bools
                    .iter()
                    .rev()
                    .map(|&b| ["0", "1"][b as usize])
                    .collect::<Vec<_>>()
                    .join("")
            );
            panic!()
        }
        ret
    }

    fn to_bools(&self) -> Vec<bool> {
        (0..self.len).map(|i| self.test(i)).collect()
    }

    fn disjoint_inner(&self, other: &BitMask, rotation: usize) -> bool {
        assert!(self.len % 64 == 0);
        assert!(self.len == other.len);

        // It's OK if this underflows when len == 0: it won't be used because
        // the loop body will never run.
        let word_index_mask = self.words.len().wrapping_sub(1);

        if rotation % 64 == 0 {
            let word_offset = rotation / 64;
            for (i, &word) in self.words.iter().enumerate() {
                let other_word = other.words[(i.wrapping_sub(word_offset)) & word_index_mask];
                if word & other_word != 0 {
                    return false;
                }
            }
            true
        } else {
            let left_word_offset = rotation / 64;
            let left_word_shift = rotation % 64;
            let right_word_offset = left_word_offset + 1;
            let right_word_shift = 64 - left_word_shift;
            for (i, &word) in self.words.iter().enumerate() {
                let left_part = other.words[i.wrapping_sub(left_word_offset) & word_index_mask]
                    << left_word_shift;
                let right_part = other.words[i.wrapping_sub(right_word_offset) & word_index_mask]
                    >> right_word_shift;
                // eprintln!("word  = {word:064b}");
                // eprintln!("left  = {left_part:064b}");
                // eprintln!("right = {right_part:064b}");
                if word & (left_part | right_part) != 0 {
                    return false;
                }
            }
            true
        }
    }
}
