use std::ops::Index;

use crate::util::{to_u32, to_usize};

use super::{Instr, Reg, Tables, Tac};

pub struct Trace(Vec<Vec<u32>>);

impl Trace {
    pub fn new(
        keys: &[Vec<u32>],
        tac: &Tac,
        tables: &Tables,
        hash_table_len: Option<usize>,
    ) -> Trace {
        let mut regs: Vec<Vec<u32>> = Vec::new();
        let width = keys.len();
        for instr in tac.instrs() {
            let row = match *instr {
                Instr::Imm(n) => vec![n; width],
                Instr::StrGet(r) => (0..width)
                    .map(|lane| keys[lane][to_usize(regs[r.0][lane])])
                    .collect(),
                Instr::StrLen => keys.iter().map(|key| to_u32(key.len())).collect(),
                Instr::TableGet(t, r) => regs[r.0]
                    .iter()
                    .map(|&i| tables[t][to_usize(i)].into())
                    .collect(),
                Instr::TableIndexMask(t) => vec![to_u32(tables[t].len() - 1); width],
                Instr::HashMask => vec![to_u32(hash_table_len.unwrap() - 1); width],
                Instr::BinOp(op, a, b) => {
                    let a_values = &regs[a.0];
                    let b_values = &regs[b.0];
                    (0..width)
                        .map(|lane| op.eval(a_values[lane], b_values[lane]))
                        .collect()
                }
            };
            assert!(row.len() == width);
            regs.push(row);
        }
        Trace(regs)
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn width(&self) -> usize {
        match self.0.first() {
            Some(vec) => vec.len(),
            None => 0,
        }
    }
}

impl Index<Reg> for Trace {
    type Output = [u32];

    fn index(&self, index: Reg) -> &Self::Output {
        &self.0[index.0]
    }
}
