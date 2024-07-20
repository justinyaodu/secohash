use std::collections::HashSet;

use crate::{
    combinatorics::{LendingIterator, PermGen},
    phf::{ExprBuilder, Interpreter, Phf, Reg},
};

fn table_size(index_bits: u32) -> usize {
    1 << index_bits
}

fn table_index_mask(index_bits: u32) -> u32 {
    (table_size(index_bits) - 1) as u32
}

struct Compressor {
    hash_bits: u32,
    mix_shifts: Vec<u32>,
    base_shift_and_offset_table: Option<(u32, Vec<u8>)>,
}

pub fn compressor_search(phf: &Phf, sel_regs: &[Reg]) -> Option<Phf> {
    let interpreter = Interpreter::new(phf, &phf.interpreted_keys);

    let mut compressor: Option<Compressor> = None;

    let n = sel_regs.len();
    let mut perm_gen = PermGen::new(n);
    'perm: while let Some(perm) = perm_gen.next() {
        let mut shifts = vec![0];
        let mut mixes = interpreter.reg_values(sel_regs[perm[0]]).to_vec();
        'sel: for i in 1..n {
            let mut shift = *shifts.last().unwrap();
            'shift: while shift < 32 {
                let mut seen = HashSet::new();
                let mut new_mixes = Vec::new();
                for (lane, mix) in mixes.iter().copied().enumerate() {
                    let sel_value = interpreter.reg_values(sel_regs[perm[i]])[lane];
                    let new_mix = mix + (sel_value << shift);
                    new_mixes.push(new_mix);

                    let mut mix_and_unmixed = vec![new_mix];
                    for j in i + 1..n {
                        mix_and_unmixed
                            .push(interpreter.reg_values(sel_regs[perm[j]])[lane]);
                    }
                    if !seen.insert(mix_and_unmixed) {
                        shift += 1;
                        continue 'shift;
                    }
                }

                shifts.push(shift);
                mixes = new_mixes;
                continue 'sel;
            }

            continue 'perm;
        }

        let mut mix_shifts = vec![0; n];
        for (i, index) in perm.iter().copied().enumerate() {
            mix_shifts[index] = shifts[i];
        }

        if let Some(hash_bits) = direct_compressor_search(phf.min_hash_bits, &compressor, &mixes) {
            compressor = Some(Compressor {
                hash_bits,
                mix_shifts: mix_shifts.clone(),
                base_shift_and_offset_table: None,
            });
        }

        let or_of_all_mixes = mixes.iter().copied().fold(0, |a, b| a | b);
        for base_shift in 0..32 {
            if or_of_all_mixes >> base_shift == 0 {
                break;
            }

            if let Some((hash_bits, pair)) =
                offset_compressor_search(phf.min_hash_bits, &compressor, &mixes, base_shift)
            {
                compressor = Some(Compressor {
                    hash_bits,
                    mix_shifts: mix_shifts.clone(),
                    base_shift_and_offset_table: Some(pair),
                });
            }
        }

        // TODO: this makes the search a lot faster and is usually just as good,
        // but it should be configurable.
        break;
    }

    let compressor = compressor?;

    let mut phf = phf.clone();
    let x = ExprBuilder();
    let mix_reg = phf.push_expr(
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
            let offset_table = phf.push_data_table(offset_table);
            phf.push_expr(x.add(
                x.shrl(x.reg(mix_reg), x.imm(base_shift)),
                x.table_get(
                    offset_table,
                    x.and(x.reg(mix_reg), x.table_index_mask(offset_table)),
                ),
            ))
        }
        None => mix_reg,
    };
    phf.push_expr(x.and(x.reg(unmasked_hash_reg), x.hash_mask()));
    phf.build_hash_table(compressor.hash_bits);
    Some(phf)
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
) -> Option<(u32, (u32, Vec<u8>))> {
    let bases: Vec<u32> = mixes.iter().copied().map(|m| m >> base_shift).collect();

    for hash_bits in min_hash_bits..=32 {
        if let Some(c) = compressor {
            if c.hash_bits <= hash_bits {
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
) -> Option<Vec<u8>> {
    assert!(bases.len() == offset_indices.len());

    let offset_table_size = table_size(offset_index_bits);
    let offset_table_index_mask = table_index_mask(offset_index_bits);

    // Group all the bases for each offset index.
    let mut groups = vec![Vec::new(); offset_table_size];
    for (&base, &index) in bases.iter().zip(offset_indices) {
        groups[(index & offset_table_index_mask) as usize].push(base);
    }

    // Sort the non-empty groups in descending order by size.
    let mut groups_and_indices = Vec::new();
    for (index, group) in groups.into_iter().enumerate() {
        if !group.is_empty() {
            groups_and_indices.push((group, index));
        }
    }
    groups_and_indices.sort_by_key(|p| p.0.len());
    groups_and_indices.reverse();

    // Assign offsets to indices using a first-fit algorithm.

    let hash_table_size = table_size(hash_bits);
    let hash_mask = table_index_mask(hash_bits);
    let mut seen = vec![false; hash_table_size];
    seen[0] = true;

    let mut offset_table: Vec<u8> = vec![0; offset_table_size];
    let offset_size = usize::min(hash_table_size, 256);
    'group: for (group, index) in groups_and_indices {
        'offset: for offset in 0..offset_size {
            let offset = u8::try_from(offset).unwrap();
            for &base in &group {
                let hash = (base.wrapping_add(offset.into()) & hash_mask) as usize;
                if seen[hash] {
                    continue 'offset;
                }
            }

            for &base in &group {
                let hash = (base.wrapping_add(offset.into()) & hash_mask) as usize;
                if seen[hash] {
                    // Keys cannot be distinguished from base and masked index.
                    return None;
                }
                seen[hash] = true;
            }
            offset_table[index] = offset;
            continue 'group;
        }

        // No table value will resolve the conflicts for this group.
        return None;
    }

    Some(offset_table)
}
