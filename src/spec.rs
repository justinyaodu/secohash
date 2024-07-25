pub struct Spec {
    pub keys: Vec<Vec<u32>>,
    pub interpreted_keys: Vec<Vec<u32>>,
    pub min_interpreted_key_len: usize,
    pub max_interpreted_key_len: usize,
    pub min_hash_bits: u32,
}

impl Spec {
    pub fn new(keys: Vec<Vec<u32>>) -> Spec {
        let mut interpreted_keys = Vec::new();
        for key in &keys {
            if !key.is_empty() {
                interpreted_keys.push(key.clone());
            }
        }
        if interpreted_keys.is_empty() {
            interpreted_keys.push(vec!['!' as u32]);
        }

        let min_hash_table_size = interpreted_keys.len() + 1;
        let mut min_hash_bits = 1;
        while 1usize << min_hash_bits < min_hash_table_size {
            min_hash_bits += 1;
        }

        let min_interpreted_key_len = interpreted_keys.iter().map(Vec::len).min().unwrap();
        let max_interpreted_key_len = interpreted_keys.iter().map(Vec::len).max().unwrap();

        Spec {
            keys,
            interpreted_keys,
            min_interpreted_key_len,
            max_interpreted_key_len,
            min_hash_bits,
        }
    }
}
