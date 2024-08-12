use std::{cmp, collections::HashSet, time::Instant};

use crate::{
    ir::{ExprBuilder, Reg, Tables, Tac},
    util::{table_index_mask, table_size, to_u32, to_usize},
};

use super::generational_bit_set::{BitSet, GenerationalBitSet};

pub struct Compressor {
    pub bitwidth: u32,
    base_shift: u32,
    offset_table: Vec<u32>,
}

impl Compressor {
    pub fn search(
        values: &[u32],
        in_bitwidth: u32,
        out_bitwidth: u32,
        max_table_bits: u32,
    ) -> Option<(Compressor, Vec<u32>)> {
        let mut seen = GenerationalBitSet::new(table_size(max_table_bits));

        let mut last_groups = None;
        for offset_index_bits in 1..=max_table_bits {
            let groups = Self::group_values(values, offset_index_bits);

            for base_shift in (in_bitwidth - out_bitwidth)..=offset_index_bits {
                let start = Instant::now();
                let opt = Self::find_offset_table(
                    &groups,
                    out_bitwidth,
                    offset_index_bits,
                    base_shift,
                    &mut seen,
                );
                eprintln!("offset table search for offset_index_bits={offset_index_bits} base_shift={base_shift} took {} us", start.elapsed().as_micros());
                if let Some((offset_table, values)) = opt {
                    return Some((
                        Compressor {
                            bitwidth: out_bitwidth,
                            base_shift,
                            offset_table,
                        },
                        values,
                    ));
                }
            }

            last_groups = Some(groups);
        }

        let mut seen = HashSet::with_capacity(values.len());
        for target_bitwidth in out_bitwidth + 1..in_bitwidth {
            let groups = last_groups.as_ref().unwrap();

            let base_shift = in_bitwidth - target_bitwidth;
            let start = Instant::now();
            let opt = Self::find_offset_table(
                groups,
                target_bitwidth,
                max_table_bits,
                base_shift,
                &mut seen,
            );
            eprintln!("offset table search for offset_index_bits={max_table_bits} base_shift={base_shift} took {} us", start.elapsed().as_micros());
            if let Some((offset_table, values)) = opt {
                return Some((
                    Compressor {
                        bitwidth: target_bitwidth,
                        base_shift,
                        offset_table,
                    },
                    values,
                ));
            }
        }
        None
    }

    fn group_values(values: &[u32], offset_index_bits: u32) -> Vec<Vec<u32>> {
        let offset_table_size = table_size(offset_index_bits);
        let offset_table_index_mask = table_index_mask(offset_index_bits);

        let mut groups = vec![Vec::new(); offset_table_size];
        for &value in values {
            let offset_index = value & offset_table_index_mask;
            groups[to_usize(offset_index)].push(value);
        }

        groups.retain(|group| !group.is_empty());
        groups.sort_by_key(|group| cmp::Reverse(group.len()));
        groups
    }

    fn find_offset_table<B>(
        groups: &[Vec<u32>],
        out_bitwidth: u32,
        offset_index_bits: u32,
        base_shift: u32,
        seen: &mut B,
    ) -> Option<(Vec<u32>, Vec<u32>)>
    where
        B: BitSet,
    {
        let hash_table_size = table_size(out_bitwidth);
        let hash_mask = table_index_mask(out_bitwidth);
        let offset_table_size = table_size(offset_index_bits);
        let offset_table_index_mask = table_index_mask(offset_index_bits);

        seen.clear();
        seen.set(0);
        let mut full_before = 1;

        let mut offset_table = vec![0; offset_table_size];
        let offset_size = to_u32(hash_table_size);

        let mut unmasked_hashes = Vec::new();

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
                    for &value in group {
                        let hash = (value >> base_shift).wrapping_add(offset) & hash_mask;
                        if seen.test(hash) {
                            continue 'offset;
                        }
                    }

                    good_offset = Some(offset);
                    break;
                }
            }

            if let Some(offset) = good_offset {
                for &mix in group {
                    let unmasked_hash = (mix >> base_shift).wrapping_add(offset);
                    seen.set(unmasked_hash & hash_mask);
                    unmasked_hashes.push(unmasked_hash);
                }
                let offset_table_index = group[0] & offset_table_index_mask;
                offset_table[to_usize(offset_table_index)] = offset;
            } else {
                return None;
            }
        }

        Some((offset_table, unmasked_hashes))
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
