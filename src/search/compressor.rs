use std::{cmp, time::Instant};

use crate::{
    ir::{ExprBuilder, Reg, Tables, Tac},
    spec::Spec,
    util::{table_index_mask, table_size, to_u32, to_usize},
};

use super::{generational_bit_set::GenerationalBitSet, mixer::Mixer};

pub struct Compressor {
    hash_bits: u32,
    base_shift: u32,
    offset_table: Vec<u32>,
}

impl Compressor {
    pub fn search(spec: &Spec, mixer: &Mixer) -> Option<Compressor> {
        let hash_bits = spec.min_hash_bits;
        let hash_table_size = table_size(hash_bits);
        let mut seen = GenerationalBitSet::new(hash_table_size);

        for offset_index_bits in 1..=hash_bits {
            let offset_table_size = table_size(offset_index_bits);
            let offset_table_index_mask = table_index_mask(offset_index_bits);

            let mut groups = vec![Vec::new(); offset_table_size];
            for &mix in &mixer.mixes {
                let offset_index = mix & offset_table_index_mask;
                groups[to_usize(offset_index)].push(mix);
            }

            groups.retain(|group| !group.is_empty());
            groups.sort_by_key(|group| cmp::Reverse(group.len()));

            for base_shift in (mixer.mix_bits - hash_bits)..=offset_index_bits {
                let start = Instant::now();
                let opt = Self::find_offset_table(
                    &groups,
                    hash_bits,
                    offset_index_bits,
                    base_shift,
                    &mut seen,
                );
                eprintln!("offset table search for offset_index_bits={offset_index_bits} base_shift={base_shift} took {} us", start.elapsed().as_micros());
                if let Some(offset_table) = opt {
                    return Some(Compressor {
                        hash_bits,
                        base_shift,
                        offset_table,
                    });
                }
            }
        }

        None
    }

    fn find_offset_table(
        groups: &[Vec<u32>],
        hash_bits: u32,
        offset_index_bits: u32,
        base_shift: u32,
        seen: &mut GenerationalBitSet,
    ) -> Option<Vec<u32>> {
        let hash_table_size = table_size(hash_bits);
        let hash_mask = table_index_mask(hash_bits);
        let offset_table_size = table_size(offset_index_bits);
        let offset_table_index_mask = table_index_mask(offset_index_bits);

        seen.clear_all();
        seen.set(0);
        let mut full_before = 1;

        let mut offset_table = vec![0; offset_table_size];
        let offset_size = to_u32(hash_table_size);

        for group in groups {
            let mut good_offset = None;
            if group.len() == 1 {
                while seen.test(full_before) {
                    full_before += 1;
                }
                good_offset =
                    Some(to_u32(full_before).wrapping_sub(group[0] >> base_shift) & hash_mask);
            } else {
                'offset: for offset in 0..offset_size {
                    for &mix in group {
                        let hash = (mix >> base_shift).wrapping_add(offset) & hash_mask;
                        if seen.test(to_usize(hash)) {
                            continue 'offset;
                        }
                    }

                    good_offset = Some(offset);
                    break;
                }
            }

            if let Some(offset) = good_offset {
                for &mix in group {
                    let hash = (mix >> base_shift).wrapping_add(offset) & hash_mask;
                    seen.set(to_usize(hash));
                }
                let offset_table_index = group[0] & offset_table_index_mask;
                offset_table[to_usize(offset_table_index)] = offset;
            } else {
                return None;
            }
        }

        Some(offset_table)
    }

    pub fn compile(self, tac: &mut Tac, tables: &mut Tables, mix_reg: Reg) -> Reg {
        let x = ExprBuilder();
        let offset_table = tables.push(self.offset_table);
        tac.push_expr(x.add(
            x.shrl(x.reg(mix_reg), x.imm(self.base_shift)),
            x.table_get(
                offset_table,
                x.and(x.reg(mix_reg), x.table_index_mask(offset_table)),
            ),
        ))
    }
}
