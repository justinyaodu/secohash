use std::collections::HashSet;

use crate::{
    ir::{Tables, Tac, Trace},
    spec::Spec,
    util::to_usize,
};

use super::compressor_searcher::CompressorSearchSolution;

pub struct Phf {
    pub tac: Tac,
    pub tables: Tables,
    pub key_table: Vec<Vec<u32>>,
}

impl Phf {
    pub fn new(
        spec: &Spec,
        CompressorSearchSolution {
            tac,
            tables,
            hash_bits,
        }: CompressorSearchSolution,
    ) -> Phf {
        let mut key_table = vec![Vec::new(); 1 << hash_bits];

        let mut has_empty_key = false;
        let mut non_empty_keys: Vec<Vec<u32>> = Vec::new();
        for key in &spec.keys {
            if key.is_empty() {
                has_empty_key = true;
            } else {
                non_empty_keys.push(key.clone());
            }
        }

        let trace = Trace::new(&non_empty_keys, &tac, &tables, Some(key_table.len()));
        let hash_reg = tac.last_reg();

        for (lane, key) in non_empty_keys.into_iter().enumerate() {
            let hash = trace[hash_reg][lane];
            key_table[to_usize(hash)] = key;
        }

        if !has_empty_key {
            let mut fake_key = vec!['!' as u32];
            for key in &key_table {
                if !key.is_empty() {
                    fake_key = key.clone();
                    break;
                }
            }
            key_table[0] = fake_key;
        }

        let mut keys = Vec::new();
        for (i, key) in key_table.iter().enumerate() {
            let hash = if key.len() < spec.min_interpreted_key_len
                || key.len() > spec.max_interpreted_key_len
            {
                0
            } else {
                Trace::new(&[key.clone()], &tac, &tables, Some(key_table.len()))[hash_reg][0]
            };
            if hash == i.try_into().unwrap() {
                keys.push(key.clone());
            }
        }

        assert!(
            keys.into_iter().collect::<HashSet<_>>()
                == spec.keys.iter().cloned().collect::<HashSet<_>>()
        );

        Phf {
            tac,
            tables,
            key_table,
        }
    }
}
