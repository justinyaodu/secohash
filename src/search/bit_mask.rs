pub struct BitMask(Vec<u64>);

impl BitMask {
    fn new(len: usize) -> BitMask {
        assert!(len & (len - 1) == 0);
        BitMask(vec![0; len / 64])
    }

    fn disjoint(&self, other: &BitMask, rotation: usize) -> bool {
        assert!(self.0.len() == other.0.len());
        let word_index_mask = self.0.len() - 1;
        if rotation % 64 == 0 {
            let word_offset = rotation / 64;
            for (i, &word) in self.0.iter().enumerate() {
                let other_word = other.0[(i + word_offset) & word_index_mask];
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
            for (i, &word) in self.0.iter().enumerate() {
                let left_part =
                    other.0[(i + left_word_offset) & word_index_mask] >> left_word_shift;
                let right_part =
                    other.0[(i + right_word_offset) & word_index_mask] << right_word_shift;
                if word & (left_part | right_part) != 0 {
                    return false;
                }
            }
            true
        }
    }
}
