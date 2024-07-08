use std::collections::HashSet;

pub struct Keys {
    pub non_empty_keys: Vec<Vec<u32>>,
    pub empty_key_ordinal: Option<usize>,
    pub start_len: usize,
    pub end_len: usize,
}

impl Keys {
    pub fn new(keys: &[Vec<u32>]) -> Keys {
        let mut empty_key_ordinal = None;
        let mut non_empty_keys = Vec::new();
        let mut seen = HashSet::new();
        for (i, key) in keys.iter().enumerate() {
            if !seen.insert(key) {
                panic!("already seen {key:?}")
            }

            if key.is_empty() {
                empty_key_ordinal = Some(i);
            } else {
                non_empty_keys.push(key.clone())
            }
        }

        let (start_len, end_len) = if non_empty_keys.is_empty() {
            (1, 1)
        } else {
            (
                non_empty_keys.iter().map(Vec::len).min().unwrap(),
                non_empty_keys.iter().map(Vec::len).max().unwrap() + 1,
            )
        };

        Keys {
            non_empty_keys,
            empty_key_ordinal,
            start_len,
            end_len,
        }
    }

    pub fn all_keys(&self) -> Vec<Vec<u32>> {
        let mut keys = self.non_empty_keys.clone();
        if let Some(i) = self.empty_key_ordinal {
            keys.insert(i, Vec::new())
        }
        keys
    }

    pub fn num_keys(&self) -> usize {
        self.all_keys().len()
    }
}
