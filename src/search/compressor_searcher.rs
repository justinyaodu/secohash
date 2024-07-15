use std::collections::HashSet;

use crate::{
    combinatorics::ShiftGen,
    phf::{ExprBuilder, Interpreter, Phf, Reg},
};

fn table_size(index_bits: u32) -> usize {
    1 << index_bits
}

fn table_index_mask(index_bits: u32) -> u32 {
    (table_size(index_bits) - 1) as u32
}

fn mix_search(phf: &Phf, interpreter: &Interpreter, sel_regs: &[Reg]) -> Option<Phf> {
    let mut mask = !0;
    let mut sol_shifts = None;

    // Ensure that we always have at least one shift equal to 0. Otherwise the
    // least significant bit of the mix is wasted, because it's always zero.
    let max_nonzero_shifts = (sel_regs.len() - 1) as u32;
    let mut mixes = HashSet::new();
    'sol: for num_nonzero_shifts in 0..=max_nonzero_shifts {
        let mut shift_gen = ShiftGen::new(sel_regs.len() as u32, num_nonzero_shifts, 31);
        loop {
            'shifts: {
                loop {
                    mixes.clear();
                    for lane in 0..interpreter.width() {
                        let mut mix = 0u32;
                        for (&sel_reg, &shift) in sel_regs.iter().zip(shift_gen.shifts.iter()) {
                            mix = mix.wrapping_add(interpreter.reg_values(sel_reg)[lane] << shift);
                        }
                        mix &= mask;

                        if !mixes.insert(mix) {
                            break 'shifts;
                        }
                    }

                    sol_shifts = Some(shift_gen.shifts.clone());
                    mask >>= 1;
                    if mask == 0 {
                        break 'sol;
                    }
                }
            }

            if !shift_gen.next() {
                break;
            }
        }
    }

    let sol_shifts = sol_shifts?;

    let mut phf = phf.clone();
    let e = ExprBuilder();
    phf.push_expr(
        e.sum(
            sel_regs
                .iter()
                .zip(sol_shifts)
                .map(|(&sel, left_shift)| e.shll(e.reg(sel), e.imm(left_shift)))
                .collect(),
        ),
    );
    Some(phf)
}

fn mixed_offset_search(
    phf: &Phf,
    interpreter: &Interpreter,
    sel_regs: &[Reg],
    hash_bits: u32,
) -> Option<Phf> {
    let phf = mix_search(phf, interpreter, sel_regs);
    let phf = phf?;

    let mix_reg = phf.last_reg();
    let interpreter = Interpreter::new(&phf, &phf.interpreted_keys);

    for offset_index_bits in 1..=hash_bits {
        let offset_index_mask = table_index_mask(offset_index_bits);
        let offset_indices: Vec<u32> = interpreter
            .reg_values(mix_reg)
            .iter()
            .map(|mix| mix & offset_index_mask)
            .collect();

        for base_shift in 0..32 {
            let bases: Vec<u32> = interpreter
                .reg_values(mix_reg)
                .iter()
                .map(|mix| mix >> base_shift)
                .collect();

            let Some(offset_table) =
                offset_table_search(&bases, &offset_indices, offset_index_bits, hash_bits)
            else {
                continue;
            };

            let mut phf = phf.clone();
            let offset_table = phf.push_data_table(offset_table);
            let e = ExprBuilder();
            phf.push_expr(e.and(
                e.add(
                    e.shrl(e.reg(mix_reg), e.imm(base_shift)),
                    e.table_get(
                        offset_table,
                        e.and(e.reg(mix_reg), e.table_index_mask(offset_table)),
                    ),
                ),
                e.hash_mask(),
            ));
            return Some(phf);
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
    let offset_size = usize::min(hash_table_size, 128) as u8;
    'group: for (group, index) in groups_and_indices {
        'offset: for offset in 0..offset_size {
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

pub fn compressor_search(phf: &Phf, sel_regs: &[Reg], max_table_size: usize) -> Option<Phf> {
    let mut start_hash_bits: u32 = 1;
    while (1 << start_hash_bits) < phf.keys.len() {
        start_hash_bits += 1;
    }

    let mut end_hash_bits: u32 = start_hash_bits;
    while (1 << end_hash_bits) <= max_table_size {
        end_hash_bits += 1;
    }

    let interpreter = Interpreter::new(phf, &phf.interpreted_keys);

    for hash_bits in start_hash_bits..end_hash_bits {
        if let Some(mut phf) = mixed_offset_search(phf, &interpreter, sel_regs, hash_bits) {
            phf.build_hash_table(hash_bits);
            return Some(phf);
        }
    }

    None
}
