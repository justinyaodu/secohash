use std::collections::HashSet;

use crate::{
    ir::{Instr, Interpreter, Ir, Reg},
    keys::Keys,
};

pub fn compressor_search(
    keys: &Keys,
    ir: &Ir,
    sel_regs: &[Reg],
    max_table_size: usize,
) -> Option<(Ir, Vec<Option<(Vec<u32>, usize)>>)> {
    let mut start_hash_bits: u32 = 1;
    while (1 << start_hash_bits) < keys.num_keys() {
        start_hash_bits += 1;
    }

    let mut end_hash_bits: u32 = start_hash_bits;
    while (1 << end_hash_bits) <= max_table_size {
        end_hash_bits += 1;
    }

    let (ir, hash_bits) = wide_hash_search(keys, ir, sel_regs, start_hash_bits)?;

    let (ir, hash_bits) = if hash_bits < end_hash_bits {
        (ir, hash_bits)
    } else {
        xor_table_search(keys, &ir, hash_bits, start_hash_bits)?
    };

    let table = build_table(keys, &ir, 1 << hash_bits);
    Some((ir, table))
}

pub fn wide_hash_search(
    keys: &Keys,
    ir: &Ir,
    sel_regs: &[Reg],
    start_hash_bits: u32,
) -> Option<(Ir, u32)> {
    let mut selections: Vec<Vec<u32>> = Vec::new();
    for key in &keys.non_empty_keys {
        let mut interpreter = Interpreter::new(ir);
        interpreter.run(key);
        selections.push(sel_regs.iter().map(|&r| interpreter.reg(r)).collect())
    }

    let mut wide_hashes = HashSet::new();

    // TODO: handle no non-empty keys

    let num_selectors = selections[0].len();
    'outer: for wide_hash_bits in start_hash_bits..=32 {
        let min_shift = 32 - wide_hash_bits;
        let mut stack: Vec<u32> = Vec::new();

        'inner: loop {
            let depth = stack.len();
            if depth == num_selectors {
                wide_hashes.clear();

                for selection in &selections {
                    let mut sum: u32 = 0;
                    for i in 0..num_selectors {
                        sum = sum.wrapping_add(selection[i] << stack[i]);
                    }
                    if !wide_hashes.insert(sum) {
                        while stack.last() == Some(&min_shift) {
                            stack.pop();
                        }
                        if stack.is_empty() {
                            continue 'outer;
                        }
                        *stack.last_mut().unwrap() -= 1;
                        continue 'inner;
                    }
                }

                let ir = compile_wide_hash(keys, ir, sel_regs, &stack, wide_hash_bits);
                return Some((ir, wide_hash_bits));
            } else if depth == num_selectors - 1 && !stack.contains(&min_shift) {
                stack.push(min_shift);
            } else {
                stack.push(31);
            }
        }
    }

    None
}

fn compile_wide_hash(
    keys: &Keys,
    ir: &Ir,
    sel_regs: &[Reg],
    left_shifts: &[u32],
    wide_hash_bits: u32,
) -> Ir {
    let mut ir = ir.clone();
    let shifted = sel_regs
        .iter()
        .zip(left_shifts)
        .map(|(&r, &s)| {
            let s = ir.instr(Instr::Imm(s));
            ir.instr(Instr::Shll(r, s))
        })
        .collect::<Vec<_>>();
    let sum = shifted
        .into_iter()
        .reduce(|a, b| ir.instr(Instr::Add(a, b)))
        .unwrap();
    let right_shift_amount = ir.instr(Instr::Imm(32 - wide_hash_bits));
    let wide_hash = ir.instr(Instr::Shrl(sum, right_shift_amount));

    ir.assert_distinguishes(keys, &[wide_hash]);
    ir
}

pub fn xor_table_search(
    keys: &Keys,
    ir: &Ir,
    cur_hash_bits: u32,
    start_hash_bits: u32,
) -> Option<(Ir, u32)> {
    let hashes = keys
        .non_empty_keys
        .iter()
        .map(|key| Interpreter::new(ir).run(key))
        .collect::<Vec<_>>();

    for hash_bits in start_hash_bits..cur_hash_bits {
        let hash_shift = cur_hash_bits - hash_bits;
        'outer: for table_index_bits in 0..start_hash_bits {
            let table_size = 1usize << table_index_bits;
            let index_mask = (1 << table_index_bits) - 1;

            let mut groups = vec![HashSet::new(); table_size];
            for hash in &hashes {
                if !groups[(hash & index_mask) as usize].insert(hash >> hash_shift) {
                    continue 'outer;
                }
            }

            let mut groups = groups
                .into_iter()
                .map(|set| set.into_iter().collect::<Vec<_>>())
                .enumerate()
                .collect::<Vec<_>>();
            groups.sort_by_key(|p| p.1.len());
            groups.reverse();

            let mut seen = vec![false; 1 << hash_bits];
            seen[0] = true;
            let mut xor_table = vec![0u8; table_size];

            for (i, group) in groups {
                let mut good_xor: Option<u32> = None;
                'inner: for xor in 0..u32::min(256, 1 << hash_bits) {
                    for index in &group {
                        if seen[(index ^ xor) as usize] {
                            continue 'inner;
                        }
                    }
                    good_xor = Some(xor);
                    break;
                }

                let Some(xor) = good_xor else {
                    continue 'outer;
                };

                for index in &group {
                    seen[(index ^ xor) as usize] = true;
                }
                xor_table[i] = xor.try_into().unwrap();
            }

            return Some((
                compile_xor_table(keys, ir, xor_table, hash_shift),
                hash_bits,
            ));
        }
    }

    None
}

fn compile_xor_table(keys: &Keys, ir: &Ir, xor_table: Vec<u8>, hash_shift: u32) -> Ir {
    let index_mask = (xor_table.len() - 1).try_into().unwrap();
    let hash = ir.last_reg();

    let mut ir = ir.clone();
    let xor_table = ir.table(xor_table);
    let index_mask = ir.instr(Instr::Imm(index_mask));
    let masked_index = ir.instr(Instr::And(hash, index_mask));
    let xor_table_value = ir.instr(Instr::Table(xor_table, masked_index));
    let hash_shift = ir.instr(Instr::Imm(hash_shift));
    let shifted_hash = ir.instr(Instr::Shrl(hash, hash_shift));
    let hash = ir.instr(Instr::Xor(shifted_hash, xor_table_value));
    ir.assert_distinguishes(keys, &[hash]);
    ir
}

fn build_table(keys: &Keys, ir: &Ir, table_size: usize) -> Vec<Option<(Vec<u32>, usize)>> {
    let mut table: Vec<Option<(Vec<u32>, usize)>> = vec![None; table_size];
    for (i, key) in keys.all_keys() {
        let index = if key.is_empty() {
            0
        } else {
            Interpreter::new(ir).run(&key) as usize
        };
        assert!(table[index].is_none());
        table[index] = Some((key, i));
    }
    assert!(table[0].is_none());
    table
}
