use std::collections::HashSet;

use crate::{
    ir::{Instr, Interpreter, Ir, Reg},
    keys::Keys,
};

pub fn compressor_search(
    keys: &Keys,
    ir: &Ir,
    regs: &[Reg],
    max_table_size: usize,
) -> Option<(Ir, Vec<Option<(Vec<u32>, usize)>>)> {
    // Find the largest power of two <= max_table_size.
    let mut max_hash_bits: u32 = 1;
    while (1 << (max_hash_bits + 1)) <= max_table_size {
        max_hash_bits += 1;
    }

    let mut min_hash_bits: u32 = 1;
    while (1 << min_hash_bits) < keys.num_keys() {
        min_hash_bits += 1;
    }

    let mut selections: Vec<Vec<u32>> = Vec::new();
    for key in &keys.non_empty_keys {
        let mut interpreter = Interpreter::new(ir);
        interpreter.run(key);
        selections.push(regs.iter().map(|r| interpreter.reg(*r)).collect())
    }

    let (wide_hashes, shifts) = wide_hash_search(&selections, min_hash_bits)?;
    assert!(wide_hashes.contains(&0));
    assert!(wide_hashes.len() == keys.non_empty_keys.len() + 1);
    assert!(shifts.len() == regs.len());
    let wide_hashes = wide_hashes.into_iter().collect::<Vec<_>>();

    let mut best_mixer = None;
    let mut hash_bits = max_hash_bits + 1;

    let mixer_bits = shifts.iter().map(|s| 32 - s).max().unwrap();
    'outer: for mixer in 1..(1u64 << mixer_bits) {
        let mixer: u32 = mixer.try_into().unwrap();
        loop {
            let target_hash_bits = hash_bits - 1;
            let mut seen = vec![false; 1usize << target_hash_bits];

            for wide_hash in &wide_hashes {
                let narrow_hash = wide_hash.wrapping_mul(mixer) >> (32 - target_hash_bits);
                if seen[narrow_hash as usize] {
                    continue 'outer;
                }
                seen[narrow_hash as usize] = true;
            }

            best_mixer = Some(mixer);
            hash_bits = target_hash_bits;
            if hash_bits == min_hash_bits {
                break 'outer;
            }
        }
    }

    let mixer = best_mixer?;

    let mut ir = ir.clone();
    let shifted = regs
        .iter()
        .zip(shifts)
        .map(|(r, s)| {
            let s = ir.instr(Instr::Imm(s));
            ir.instr(Instr::Shll(*r, s))
        })
        .collect::<Vec<_>>();
    let sum = shifted
        .into_iter()
        .reduce(|a, b| ir.instr(Instr::Add(a, b)))
        .unwrap();

    let mut check_table = HashSet::new();
    for (_, key) in keys.all_keys() {
        let hash = if key.is_empty() {
            0
        } else {
            Interpreter::new(&ir).run(&key)
        };
        assert!(check_table.insert(hash));
    }

    let mixer_reg = ir.instr(Instr::Imm(mixer));
    let mixed = ir.instr(Instr::Mul(sum, mixer_reg));
    let final_shift_reg = ir.instr(Instr::Imm(32 - hash_bits));
    ir.instr(Instr::Shrl(mixed, final_shift_reg));

    let table_size = 1usize << hash_bits;
    let table = build_table(keys, &ir, table_size);
    Some((ir, table))
}

pub fn wide_hash_search(
    selections: &[Vec<u32>],
    min_hash_bits: u32,
) -> Option<(HashSet<u32>, Vec<u32>)> {
    let mut wide_hashes = HashSet::new();
    wide_hashes.insert(0);

    if selections.is_empty() {
        return Some((wide_hashes, Vec::new()));
    }

    let num_selectors = selections[0].len();
    'outer: for width in min_hash_bits..=32 {
        let min_shift = 32 - width;
        let mut stack: Vec<u32> = Vec::new();

        'inner: loop {
            let depth = stack.len();
            if depth == num_selectors {
                wide_hashes.clear();
                wide_hashes.insert(0);

                for selection in selections {
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
                return Some((wide_hashes, stack));
            } else if depth == num_selectors - 1 && !stack.contains(&min_shift) {
                stack.push(min_shift);
            } else {
                stack.push(31);
            }
        }
    }

    None
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
    table
}
