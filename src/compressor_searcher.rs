use std::collections::HashSet;

use crate::{
    ir::{Instr, Interpreter, Ir, Reg},
    keys::Keys,
    shift_gen::ShiftGen,
};

fn table_size(index_bits: u32) -> usize {
    1 << index_bits
}

fn table_index_mask(index_bits: u32) -> u32 {
    (table_size(index_bits) - 1) as u32
}

fn no_offset_search(
    ir: &Ir,
    interpreters: &[Interpreter],
    sel_regs: &[Reg],
    hash_bits: u32,
) -> Option<Ir> {
    for num_nonzero_shifts in 0..(sel_regs.len() as u32) {
        let mut shift_gen = ShiftGen::new(sel_regs.len() as u32, num_nonzero_shifts, hash_bits - 1);

        loop {
            let sol = no_offset_search_2(ir, interpreters, sel_regs, hash_bits, &shift_gen.shifts);
            if sol.is_some() {
                return sol;
            }

            if !shift_gen.next() {
                break;
            }
        }
    }

    None
}

fn no_offset_search_2(
    ir: &Ir,
    interpreters: &[Interpreter],
    sel_regs: &[Reg],
    hash_bits: u32,
    shifts: &[u32],
) -> Option<Ir> {
    assert!(sel_regs.len() == shifts.len());

    let mut seen = vec![false; table_size(hash_bits)];
    for interpreter in interpreters {
        let mut hash = 0u32;
        for i in 0..sel_regs.len() {
            hash = hash.wrapping_add(interpreter.reg(sel_regs[i]) << shifts[i]);
        }
        hash &= table_index_mask(hash_bits);
        if seen[hash as usize] {
            return None;
        }
        seen[hash as usize] = true;
    }

    let mut ir = ir.clone();
    let shifted_regs = sel_regs
        .iter()
        .zip(shifts)
        .map(|(&sel_reg, &shift)| {
            if shift == 0 {
                sel_reg
            } else {
                let shift_amount = ir.instr(Instr::Imm(shift));
                ir.instr(Instr::Shll(sel_reg, shift_amount))
            }
        })
        .collect::<Vec<_>>();
    let unmasked_hash = shifted_regs
        .into_iter()
        .reduce(|a, b| ir.instr(Instr::Add(a, b)))
        .unwrap();
    let hash_mask = ir.instr(Instr::Imm(table_index_mask(hash_bits)));
    ir.instr(Instr::And(unmasked_hash, hash_mask));
    Some(ir)
}

fn unmixed_offset_search(
    ir: &Ir,
    interpreters: &[Interpreter],
    sel_regs: &[Reg],
    hash_bits: u32,
) -> Option<Ir> {
    let num_base_sels = (sel_regs.len() - 1) as u32;
    for offset_index_bits in 1..=hash_bits {
        for index_sel_index in 0..sel_regs.len() {
            for num_nonzero_shifts in 0..num_base_sels {
                let mut base_sel_regs = sel_regs.to_vec();
                let offset_index_sel_reg = base_sel_regs.remove(index_sel_index);

                let mut shift_gen = ShiftGen::new(num_base_sels, num_nonzero_shifts, hash_bits - 1);

                loop {
                    let sol = unmixed_offset_search_2(
                        ir,
                        interpreters,
                        &base_sel_regs,
                        hash_bits,
                        &shift_gen.shifts,
                        offset_index_sel_reg,
                        offset_index_bits,
                    );
                    if sol.is_some() {
                        return sol;
                    }

                    if !shift_gen.next() {
                        break;
                    }
                }
            }
        }
    }

    None
}

fn unmixed_offset_search_2(
    ir: &Ir,
    interpreters: &[Interpreter],
    base_sel_regs: &[Reg],
    hash_bits: u32,
    shifts: &[u32],
    offset_index_sel_reg: Reg,
    offset_index_bits: u32,
) -> Option<Ir> {
    let bases: Vec<u32> = interpreters
        .iter()
        .map(|interpreter| {
            let mut direct_value = 0u32;
            for i in 0..base_sel_regs.len() {
                direct_value =
                    direct_value.wrapping_add(interpreter.reg(base_sel_regs[i]) << shifts[i]);
            }
            direct_value
        })
        .collect();

    let offset_indices: Vec<u32> = interpreters
        .iter()
        .map(|interpreter| interpreter.reg(offset_index_sel_reg))
        .collect();

    let table = offset_table_search(&bases, &offset_indices, offset_index_bits, hash_bits);
    let table = table?;

    let mut ir = ir.clone();
    let table = ir.table(table);

    let shifted_regs = base_sel_regs
        .iter()
        .zip(shifts)
        .map(|(&sel_reg, &shift)| {
            if shift == 0 {
                sel_reg
            } else {
                let shift_amount = ir.instr(Instr::Imm(shift));
                ir.instr(Instr::Shll(sel_reg, shift_amount))
            }
        })
        .collect::<Vec<_>>();
    let base_reg = shifted_regs
        .into_iter()
        .reduce(|a, b| ir.instr(Instr::Add(a, b)))
        .unwrap();

    let offset_index_mask = ir.instr(Instr::Imm(table_index_mask(offset_index_bits)));
    let offset_index = ir.instr(Instr::And(offset_index_sel_reg, offset_index_mask));
    let offset = ir.instr(Instr::Table(table, offset_index));
    let unmasked_hash = ir.instr(Instr::Add(base_reg, offset));
    let hash_mask = ir.instr(Instr::Imm(table_index_mask(hash_bits)));
    ir.instr(Instr::And(unmasked_hash, hash_mask));
    Some(ir)
}

fn mix_search(ir: &Ir, interpreters: &[Interpreter], sel_regs: &[Reg]) -> Option<Ir> {
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
                    for interpreter in interpreters {
                        let mut mix = 0u32;
                        for (&sel_reg, &shift) in sel_regs.iter().zip(shift_gen.shifts.iter()) {
                            mix = mix.wrapping_add(interpreter.reg(sel_reg) << shift);
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

    let mut ir = ir.clone();
    let shifted_regs = sel_regs
        .iter()
        .zip(sol_shifts)
        .map(|(&sel, left_shift)| {
            let left_shift = ir.instr(Instr::Imm(left_shift));
            ir.instr(Instr::Shll(sel, left_shift))
        })
        .collect::<Vec<_>>();
    shifted_regs
        .into_iter()
        .reduce(|a, b| ir.instr(Instr::Add(a, b)))
        .unwrap();
    Some(ir)
}

fn mixed_offset_search(
    keys: &Keys,
    ir: &Ir,
    interpreters: &[Interpreter],
    sel_regs: &[Reg],
    hash_bits: u32,
) -> Option<Ir> {
    let ir = mix_search(ir, interpreters, sel_regs);
    let ir = ir?;
    let interpreters = ir.run_all(keys);

    let mix_reg = ir.last_reg();
    for offset_index_bits in 1..=hash_bits {
        let offset_index_mask = table_index_mask(offset_index_bits);
        for base_shift in 0..32 {
            let bases: Vec<u32> = interpreters
                .iter()
                .map(|interpreter| interpreter.reg(mix_reg) >> base_shift)
                .collect();

            let offset_indices: Vec<u32> = interpreters
                .iter()
                .map(|interpreter| interpreter.reg(mix_reg) & offset_index_mask)
                .collect();

            let Some(offset_table) =
                offset_table_search(&bases, &offset_indices, offset_index_bits, hash_bits)
            else {
                continue;
            };

            let mut ir = ir.clone();
            let offset_table = ir.table(offset_table);
            let base_shift_reg = ir.instr(Instr::Imm(base_shift));
            let base_reg = ir.instr(Instr::Shrl(mix_reg, base_shift_reg));
            let offset_index_mask_reg = ir.instr(Instr::Imm(offset_index_mask));
            let offset_index_reg = ir.instr(Instr::And(mix_reg, offset_index_mask_reg));
            let offset_reg = ir.instr(Instr::Table(offset_table, offset_index_reg));
            let unmasked_hash_reg = ir.instr(Instr::Add(base_reg, offset_reg));
            let hash_mask_reg = ir.instr(Instr::Imm(table_index_mask(hash_bits)));
            ir.instr(Instr::And(unmasked_hash_reg, hash_mask_reg));
            return Some(ir);
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

pub struct Phf {
    pub ir: Ir,
    pub hash_table: Vec<Option<(Vec<u32>, usize)>>,
}

impl Phf {
    fn new(keys: &Keys, ir: Ir, hash_bits: u32) -> Phf {
        let mut hash_table: Vec<Option<(Vec<u32>, usize)>> = vec![None; table_size(hash_bits)];
        for (i, key) in keys.all_keys() {
            let index = if key.is_empty() {
                0
            } else {
                Interpreter::new().run(&ir, &key) as usize
            };
            assert!(hash_table[index].is_none());
            hash_table[index] = Some((key, i));
        }

        Phf { ir, hash_table }
    }
}

pub fn compressor_search(
    keys: &Keys,
    ir: &Ir,
    sel_regs: &[Reg],
    max_table_size: usize,
) -> Option<Phf> {
    let mut start_hash_bits: u32 = 1;
    while (1 << start_hash_bits) < keys.num_keys() {
        start_hash_bits += 1;
    }

    let mut end_hash_bits: u32 = start_hash_bits;
    while (1 << end_hash_bits) <= max_table_size {
        end_hash_bits += 1;
    }

    let interpreters = ir.run_all(keys);

    for hash_bits in start_hash_bits..end_hash_bits {
        if let Some(ir) = no_offset_search(ir, &interpreters, sel_regs, hash_bits) {
            return Some(Phf::new(keys, ir, hash_bits));
        }
        if let Some(ir) = unmixed_offset_search(ir, &interpreters, sel_regs, hash_bits) {
            return Some(Phf::new(keys, ir, hash_bits));
        }
        if let Some(ir) = mixed_offset_search(keys, ir, &interpreters, sel_regs, hash_bits) {
            return Some(Phf::new(keys, ir, hash_bits));
        }
    }

    None
}
