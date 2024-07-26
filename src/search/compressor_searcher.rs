use std::{collections::HashSet, time::Instant};

use crate::{
    ir::{ExprBuilder, Tables, Tac, Trace},
    search::generational_bit_set::GenerationalBitSet,
    spec::Spec,
    util::{table_index_mask, table_size, to_u32, to_usize},
};

use super::selector_searcher::SelectorSearchSolution;

pub struct CompressorSearchSolution {
    pub tac: Tac,
    pub tables: Tables,
    pub hash_bits: u32,
}

struct Compressor {
    hash_bits: u32,
    mix_shifts: Vec<u32>,
    base_shift_and_offset_table: Option<(u32, Vec<u32>)>,
}

pub fn compressor_search(
    spec: &Spec,
    SelectorSearchSolution {
        mut tac,
        mut tables,
        sel_regs,
    }: SelectorSearchSolution,
) -> Option<CompressorSearchSolution> {
    eprintln!("min_hash_bits={}", spec.min_hash_bits);

    let trace = Trace::new(&spec.interpreted_keys, &tac, &tables, None);

    let mut mix_shifts = vec![0];
    let mut mixes = trace[sel_regs[0]].to_vec();
    'sel: for i in 1..sel_regs.len() {
        let mut shift = *mix_shifts.last().unwrap();
        let mut seen = HashSet::with_capacity(trace.width());
        'shift: while shift < 32 {
            seen.clear();
            let mut new_mixes = Vec::with_capacity(trace.width());
            for (lane, mix) in mixes.iter().copied().enumerate() {
                let sel_value = trace[sel_regs[i]][lane];
                let new_mix = mix + (sel_value << shift);
                new_mixes.push(new_mix);

                let mut mix_and_unmixed = vec![new_mix];
                for j in i + 1..sel_regs.len() {
                    mix_and_unmixed.push(trace[sel_regs[j]][lane]);
                }
                if !seen.insert(mix_and_unmixed) {
                    shift += 1;
                    continue 'shift;
                }
            }

            mix_shifts.push(shift);
            mixes = new_mixes;
            continue 'sel;
        }

        return None;
    }

    let direct_hash_bits = direct_compressor_search(spec.min_hash_bits, &mixes);

    let mut compressor = Compressor {
        hash_bits: direct_hash_bits,
        mix_shifts: mix_shifts.clone(),
        base_shift_and_offset_table: None,
    };

    let start = Instant::now();
    eprintln!(
        "found direct compressor with hash_bits={}",
        compressor.hash_bits
    );
    eprintln!(
        "direct compressor search took {} us",
        start.elapsed().as_micros()
    );

    if compressor.hash_bits > spec.min_hash_bits {
        'compressor: for hash_bits in spec.min_hash_bits..=spec.min_hash_bits {
            let hash_table_size = table_size(hash_bits);
            let mut seen = GenerationalBitSet::new(hash_table_size);

            for offset_index_bits in 1..=hash_bits {
                let offset_table_size = table_size(offset_index_bits);
                let offset_table_index_mask = table_index_mask(offset_index_bits);

                let mut groups = vec![Vec::new(); offset_table_size];
                for &mix in &mixes {
                    let offset_index = mix & offset_table_index_mask;
                    groups[to_usize(offset_index)].push(mix);
                }

                groups.retain(|group| !group.is_empty());
                groups.sort_by_key(Vec::len);

                for base_shift in (direct_hash_bits - hash_bits)..=offset_index_bits {
                    let start = Instant::now();
                    let opt = offset_table_search(
                        &groups,
                        hash_bits,
                        offset_index_bits,
                        base_shift,
                        &mut seen,
                    );
                    eprintln!("offset table search for offset_index_bits={offset_index_bits} base_shift={base_shift} took {} us", start.elapsed().as_micros());
                    if let Some(offset_table) = opt {
                        compressor = Compressor {
                            hash_bits,
                            mix_shifts,
                            base_shift_and_offset_table: Some((base_shift, offset_table)),
                        };
                        break 'compressor;
                    }
                }
            }
        }
    }

    let x = ExprBuilder();
    let mix_reg = tac.push_expr(
        x.sum(
            sel_regs
                .iter()
                .zip(compressor.mix_shifts)
                .map(|(&sel, left_shift)| x.shll(x.reg(sel), x.imm(left_shift)))
                .collect(),
        ),
    );
    let unmasked_hash_reg = match compressor.base_shift_and_offset_table {
        Some((base_shift, offset_table)) => {
            let offset_table = tables.push(offset_table);
            tac.push_expr(x.add(
                x.shrl(x.reg(mix_reg), x.imm(base_shift)),
                x.table_get(
                    offset_table,
                    x.and(x.reg(mix_reg), x.table_index_mask(offset_table)),
                ),
            ))
        }
        None => mix_reg,
    };
    tac.push_expr(x.and(x.reg(unmasked_hash_reg), x.hash_mask()));
    Some(CompressorSearchSolution {
        tac,
        tables,
        hash_bits: compressor.hash_bits,
    })
}

fn direct_compressor_search(min_hash_bits: u32, mixes: &[u32]) -> u32 {
    let mut seen = HashSet::with_capacity(mixes.len());
    'hash_bits: for hash_bits in min_hash_bits..32 {
        let mask = table_index_mask(hash_bits);
        seen.clear();
        for mix in mixes {
            if !seen.insert(mix & mask) {
                continue 'hash_bits;
            }
        }
        return hash_bits;
    }
    32
}

fn offset_table_search(
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

    for group in groups.iter().rev() {
        let start = Instant::now();

        let mut good_offset = None;
        if group.len() == 1 {
            while seen.test(full_before) {
                full_before += 1;
            }
            good_offset =
                Some(to_u32(full_before).wrapping_sub(group[0] >> base_shift) & hash_mask);
            seen.set(full_before);
            full_before += 1;
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
            // No offset can resolve the conflicts for this group.
            eprintln!(
                "\tfailed to fit group of size {} in {} us",
                group.len(),
                start.elapsed().as_micros()
            );
            return None;
        }
    }

    Some(offset_table)
}
