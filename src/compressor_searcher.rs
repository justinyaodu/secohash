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
    let max_table_size = {
        let mut tmp = 1;
        while tmp * 2 <= max_table_size {
            tmp *= 2;
        }
        tmp
    };

    let mut selections: Vec<Vec<u32>> = Vec::new();
    for key in &keys.non_empty_keys {
        let mut interpreter = Interpreter::new(ir);
        interpreter.run(key);
        selections.push(regs.iter().map(|r| interpreter.reg(*r)).collect())
    }

    let num_selectors = regs.len();
    let bound = 1u32 << (5 * num_selectors);
    let mut best_mask = (max_table_size as u32) - 1;
    let mut best_shifts: Option<Vec<u32>> = None;
    for bits in 0..bound {
        let shifts = (0..num_selectors)
            .map(|i| (bits >> (5 * i)) & 31)
            .collect::<Vec<_>>();

        let mut has_collision = false;
        let mut seen = HashSet::new();
        seen.insert(0);
        for selection in &selections {
            let mut sum = 0u32;
            for i in 0..num_selectors {
                sum = sum.wrapping_add(selection[i] << shifts[i]);
            }
            if !seen.insert(sum & best_mask) {
                has_collision = true;
                break;
            }
        }

        if has_collision {
            continue;
        }

        while best_mask > 0 {
            let next_mask = best_mask >> 1;
            let next_seen = seen
                .iter()
                .map(|sum| sum & next_mask)
                .collect::<HashSet<_>>();
            if next_seen.len() < seen.len() {
                break;
            }
            best_mask = next_mask;
            seen = next_seen;
        }

        best_shifts = Some(shifts);
    }

    let best_shifts = best_shifts?;

    let mut ir = ir.clone();
    let shifted = regs
        .iter()
        .zip(best_shifts)
        .map(|(r, s)| {
            let s = ir.instr(Instr::Imm(s));
            ir.instr(Instr::Shll(*r, s))
        })
        .collect::<Vec<_>>();
    let sum = shifted
        .into_iter()
        .reduce(|a, b| ir.instr(Instr::Add(a, b)))
        .unwrap();
    let mask = ir.instr(Instr::Imm(best_mask));
    ir.instr(Instr::And(sum, mask));

    let table_size = (best_mask as usize) + 1;
    let table = build_table(keys, &ir, table_size);
    Some((ir, table))
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
