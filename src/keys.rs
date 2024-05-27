use std::{cmp::Ordering, collections::HashSet};

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

        let start_len = non_empty_keys.iter().map(Vec::len).min().unwrap_or(0);
        let end_len = non_empty_keys
            .iter()
            .map(Vec::len)
            .max()
            .map(|n| n + 1)
            .unwrap_or(0);

        Keys {
            non_empty_keys,
            empty_key_ordinal,
            start_len,
            end_len,
        }
    }

    pub fn num_keys(&self) -> usize {
        self.non_empty_keys.len() + self.empty_key_ordinal.iter().len()
    }

    pub fn all_keys(&self) -> Vec<(usize, Vec<u32>)> {
        let empty_key_ordinal = self.empty_key_ordinal.unwrap_or(usize::MAX);
        (0..self.num_keys())
            .map(|i| {
                let key = match i.cmp(&empty_key_ordinal) {
                    Ordering::Less => self.non_empty_keys[i].clone(),
                    Ordering::Equal => Vec::new(),
                    Ordering::Greater => self.non_empty_keys[i - 1].clone()
                };
                (i, key)
            })
            .collect()
    }
}
