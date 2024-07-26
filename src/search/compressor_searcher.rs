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

    let mut compressor: Option<Compressor> = None;

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

    let start = Instant::now();
    if let Some(hash_bits) = direct_compressor_search(spec.min_hash_bits, &compressor, &mixes) {
        compressor = Some(Compressor {
            hash_bits,
            mix_shifts: mix_shifts.clone(),
            base_shift_and_offset_table: None,
        });
        eprintln!("found direct compressor with hash_bits={hash_bits}");
    }
    eprintln!(
        "direct compressor search took {} us",
        start.elapsed().as_micros()
    );

    let mut interesting_mask = 0;
    for i in 1..mixes.len() {
        interesting_mask |= mixes[i - 1] ^ mixes[i];
    }

    for base_shift in 1..32 {
        if interesting_mask >> base_shift == 0 {
            break;
        }

        let start = Instant::now();
        if let Some((hash_bits, pair)) =
            offset_compressor_search(spec.min_hash_bits, &compressor, &mixes, base_shift)
        {
            compressor = Some(Compressor {
                hash_bits,
                mix_shifts: mix_shifts.clone(),
                base_shift_and_offset_table: Some(pair),
            });
            eprintln!("found offset compressor with hash_bits={hash_bits}");
        }
        eprintln!(
            "offset compressor search for base_shift={base_shift} took {} us",
            start.elapsed().as_micros()
        );
    }

    let compressor = compressor?;

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

fn direct_compressor_search(
    min_hash_bits: u32,
    compressor: &Option<Compressor>,
    mixes: &[u32],
) -> Option<u32> {
    let mut seen = HashSet::new();
    'hash_bits: for hash_bits in min_hash_bits..=32 {
        if let Some(c) = compressor {
            if c.hash_bits < hash_bits {
                break;
            }
        }

        let mask = table_index_mask(hash_bits);
        seen.clear();
        for mix in mixes {
            if !seen.insert(mix & mask) {
                continue 'hash_bits;
            }
        }
        return Some(hash_bits);
    }
    None
}

fn offset_compressor_search(
    min_hash_bits: u32,
    compressor: &Option<Compressor>,
    mixes: &[u32],
    base_shift: u32,
) -> Option<(u32, (u32, Vec<u32>))> {
    let bases: Vec<u32> = mixes.iter().copied().map(|m| m >> base_shift).collect();

    for hash_bits in min_hash_bits..=(min_hash_bits + 1) {
        if let Some(c) = compressor {
            if c.hash_bits < hash_bits {
                break;
            }
        }

        for offset_index_bits in 1..=hash_bits {
            if let Some(c) = compressor {
                if c.hash_bits == hash_bits {
                    if let Some((_, offset_table)) = c.base_shift_and_offset_table.as_ref() {
                        if offset_table.len() <= 1usize << offset_index_bits {
                            break;
                        }
                    }
                }
            }

            if let Some(offset_table) =
                offset_table_search(&bases, mixes, offset_index_bits, hash_bits)
            {
                return Some((hash_bits, (base_shift, offset_table)));
            }
        }
    }
    None
}

fn offset_table_search(
    bases: &[u32],
    offset_indices: &[u32],
    offset_index_bits: u32,
    hash_bits: u32,
) -> Option<Vec<u32>> {
    assert!(bases.len() == offset_indices.len());

    let hash_table_size = table_size(hash_bits);
    let hash_mask = table_index_mask(hash_bits);

    let offset_table_size = table_size(offset_index_bits);
    let offset_table_index_mask = table_index_mask(offset_index_bits);

    // Group all the bases for each offset index.
    let mut groups = vec![Vec::new(); offset_table_size];
    for (&base, &index) in bases.iter().zip(offset_indices) {
        groups[(index & offset_table_index_mask) as usize].push(base);
    }

    let mut seen = GenerationalBitSet::new(hash_table_size);

    // Sort the non-empty groups in descending order by size.
    let mut groups_and_indices = Vec::new();
    for (index, group) in groups.into_iter().enumerate() {
        if group.len() >= 2 {
            seen.clear_all();
            for &base in &group {
                if !seen.insert(to_usize(base & hash_mask)) {
                    // eprintln!("pruned because of group conflict");
                    return None;
                }
            }
        }

        if !group.is_empty() {
            groups_and_indices.push((group, index));
        }
    }
    groups_and_indices.sort_by_key(|p| p.0.len());
    groups_and_indices.reverse();
    // eprintln!("group sizes = {:?}", groups_and_indices.iter().map(|x| x.0.len()).collect::<Vec<_>>());

    // Assign offsets to indices using a first-fit algorithm.

    seen.clear_all();
    seen.set(0);

    let mut offset_table = vec![0; offset_table_size];
    let offset_size = to_u32(hash_table_size);
    'group: for (group, index) in groups_and_indices {
        'offset: for offset in 0..offset_size {
            for &base in &group {
                let hash = (base.wrapping_add(offset) & hash_mask) as usize;
                if seen.test(hash) {
                    continue 'offset;
                }
            }

            for &base in &group {
                let hash = (base.wrapping_add(offset) & hash_mask) as usize;
                seen.set(hash);
            }
            offset_table[index] = offset;
            continue 'group;
        }

        // No table value will resolve the conflicts for this group.
        return None;
    }

    Some(offset_table)
}
