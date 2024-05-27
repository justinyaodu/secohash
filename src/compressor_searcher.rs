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

    let num_selectors = regs.len();

    // for hash_bits in min_hash_bits..=max_hash_bits {
    let hash_bits = max_hash_bits;
    {
        let final_shift = 32 - hash_bits;
        let bound = 1u32 << (5 * num_selectors);
        for bits in 0..bound {
            let shifts = (0..num_selectors)
                .map(|i| (bits >> (5 * i)) & 31)
                .collect::<Vec<_>>();

            let mut has_collision = false;
            let mut wide_hashes = HashSet::new();
            wide_hashes.insert(0);
            for selection in &selections {
                let mut sum = 0u32;
                for i in 0..num_selectors {
                    sum = sum.wrapping_add(selection[i] << shifts[i]);
                }
                if !wide_hashes.insert(sum) {
                    has_collision = true;
                    break;
                }
            }

            if has_collision {
                continue;
            }

            let wide_hashes = wide_hashes.into_iter().collect::<Vec<_>>();
            for mixer in 0..(1u32 << 28) {
                let mixer = (mixer << 1) + 1;
                let mut narrow_hashes = HashSet::new();
                let mut has_collision = false;
                for wide_hash in &wide_hashes {
                    if !narrow_hashes.insert(wide_hash.wrapping_mul(mixer) >> final_shift) {
                        has_collision = true;
                        break;
                    }
                }
                if has_collision {
                    continue;
                }

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
                let mixer_reg = ir.instr(Instr::Imm(mixer));
                let mixed = ir.instr(Instr::Mul(sum, mixer_reg));
                let final_shift_reg = ir.instr(Instr::Imm(final_shift));
                ir.instr(Instr::Shrl(mixed, final_shift_reg));

                let table_size = ((1 << hash_bits) as usize) + 1;
                let table = build_table(keys, &ir, table_size);
                return Some((ir, table));
            }

            return None;
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
