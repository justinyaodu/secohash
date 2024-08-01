use std::{collections::HashSet, iter, mem};

use crate::ir::{ExprBuilder, Reg, Tac};

pub struct Mixer {
    pub shifts: Vec<u32>,
    pub mix_bits: u32,
    pub mixes: Vec<u32>,
    pub uses_index_zero: bool,
}

impl Mixer {
    pub fn search(cols: &[&[u32]]) -> Option<Mixer> {
        assert!(!cols.is_empty());
        let width = cols[0].len();
        let mut shifts = vec![0];
        let mut mixes = cols[0].to_vec();
        let mut new_mixes = vec![0; width];
        let mut seen = HashSet::with_capacity(width);
        'col: for i in 1..cols.len() {
            'shift: for shift in *shifts.last().unwrap()..32 {
                seen.clear();
                for (lane, mix) in mixes.iter().copied().enumerate() {
                    let new_mix = mix + (cols[i][lane] << shift);
                    new_mixes[lane] = new_mix;

                    let vec: Vec<u32> = iter::once(new_mix)
                        .chain(cols[i + 1..].iter().map(|col| col[lane]))
                        .collect();
                    if !seen.insert(vec) {
                        continue 'shift;
                    }
                }

                shifts.push(shift);
                mem::swap(&mut mixes, &mut new_mixes);
                continue 'col;
            }

            return None;
        }

        let mut mix_bits = 32;
        let mut masked_mixes = HashSet::with_capacity(width);
        'bits: for bits in *shifts.last().unwrap()..32 {
            let mask = (1 << bits) - 1;

            masked_mixes.clear();
            for &mix in &mixes {
                if !masked_mixes.insert(mix & mask) {
                    continue 'bits;
                }
            }

            mix_bits = bits;
            break;
        }

        Some(Mixer {
            shifts,
            mix_bits,
            mixes,
            uses_index_zero: masked_mixes.contains(&0),
        })
    }

    pub fn compile(&self, tac: &mut Tac, regs: &[Reg]) -> Reg {
        assert!(regs.len() == self.shifts.len());
        let x = ExprBuilder();
        tac.push_expr(
            x.sum(
                regs.iter()
                    .zip(&self.shifts)
                    .map(|(&reg, &shift)| x.shll(x.reg(reg), x.imm(shift)))
                    .collect(),
            ),
        )
    }
}
