use crate::ir::{Tables, Tac};

pub struct CompressorSearchSolution {
    pub tac: Tac,
    pub tables: Tables,
    pub hash_bits: u32,
}
