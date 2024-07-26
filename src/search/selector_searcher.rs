use std::collections::{HashMap, HashSet};

use crate::{
    combinatorics::{ChooseGen, LendingIterator},
    ir::{Expr, ExprBuilder, Instr, Reg, Tables, Tac, Trace},
    search::selector::Selector,
    spec::Spec,
    util::to_u32,
};

pub struct SelectorSearchSolution {
    pub tac: Tac,
    pub tables: Tables,
    pub sel_regs: Vec<Reg>,
}

pub fn selector_search(spec: &Spec) -> Option<SelectorSearchSolution> {
    let sol = basic_search(spec);
    if sol.is_some() {
        return sol;
    }

    table_search(spec)
}

fn basic_search(spec: &Spec) -> Option<SelectorSearchSolution> {
    let index_bound = usize::min(spec.min_interpreted_key_len, 32);

    let mut tac = Tac::new();
    let mut tables = Tables::new();

    let mut sels_1 = Vec::new();
    sels_1.push(Selector::Len);
    for i in 0..index_bound {
        sels_1.push(Selector::Index(i));
    }

    let mut sels_2 = Vec::new();
    for i in 1..=index_bound {
        sels_2.push(Selector::Sub(i));
    }
    for i in 0..index_bound {
        sels_2.push(Selector::And(i));
    }
    for i in 1..32 {
        if (spec.max_interpreted_key_len >> i) == 0 {
            break;
        }
        sels_2.push(Selector::Shrl(i));
    }

    let regs_1: Vec<Reg> = sels_1
        .into_iter()
        .map(|sel| sel.compile(&mut tac, &mut tables))
        .collect();

    let regs_2: Vec<Reg> = regs_1
        .iter()
        .copied()
        .chain(
            sels_2
                .into_iter()
                .map(|sel| sel.compile(&mut tac, &mut tables)),
        )
        .collect();

    let trace = Trace::new(&spec.interpreted_keys, &tac, &tables, None);

    for num_choices in 1..=4 {
        for all_regs in [&regs_1, &regs_2] {
            if num_choices > all_regs.len() {
                continue;
            }
            let sol = find_distinguishing_regs(&trace, all_regs, num_choices, &[]);
            if let Some(sel_regs) = sol {
                return Some(SelectorSearchSolution {
                    tac,
                    tables,
                    sel_regs,
                });
            }
        }
    }

    None
}

fn table_search(spec: &Spec) -> Option<SelectorSearchSolution> {
    let mut keys_by_len: HashMap<usize, Vec<Vec<u32>>> = HashMap::new();
    for key in &spec.interpreted_keys {
        keys_by_len.entry(key.len()).or_default().push(key.clone())
    }

    let traces_by_len: HashMap<usize, (Trace, HashMap<Reg, usize>, Reg)> = keys_by_len
        .into_iter()
        .map(|(len, keys)| {
            let mut tac = Tac::new();
            let mut tables = Tables::new();

            let mut reg_to_index = HashMap::new();
            for i in 0..len {
                reg_to_index.insert(Selector::Index(i).compile(&mut tac, &mut tables), i);
            }
            let sum_reg = tac.push(Instr::StrSum);
            (
                len,
                (
                    Trace::new(&keys, &tac, &tables, None),
                    reg_to_index,
                    sum_reg,
                ),
            )
        })
        .collect();

    for use_sum_reg in [false, true] {
        'num_tables: for num_tables in 1..=3 {
            let mut raw_tables = vec![vec![0u32; spec.max_interpreted_key_len + 1]; num_tables];

            for (&len, &(ref trace, ref reg_to_index, sum_reg)) in &traces_by_len {
                let mut regs = vec![Reg(0); reg_to_index.len()];
                for (&reg, &index) in reg_to_index {
                    regs[index] = reg;
                }
                let num_choices = usize::min(num_tables, regs.len());
                let extra_regs = if use_sum_reg {
                    vec![sum_reg]
                } else {
                    Vec::new()
                };
                let Some(chosen) = find_distinguishing_regs(trace, &regs, num_choices, &extra_regs)
                else {
                    continue 'num_tables;
                };
                for (i, choice) in chosen.iter().enumerate() {
                    raw_tables[i][len] = reg_to_index[choice].try_into().unwrap();
                }
            }

            let mut tac = Tac::new();
            let mut tables = Tables::new();
            let mut sel_regs = Vec::new();
            sel_regs.push(Selector::Len.compile(&mut tac, &mut tables));
            for raw_table in raw_tables {
                sel_regs.push(Selector::Table(raw_table).compile(&mut tac, &mut tables));
            }
            if use_sum_reg {
                sel_regs.push(tac.push(Instr::StrSum));
            }
            return Some(SelectorSearchSolution {
                tac,
                tables,
                sel_regs,
            });
        }
    }

    None
}

fn find_distinguishing_regs(
    trace: &Trace,
    regs: &[Reg],
    num_choices: usize,
    extra_regs: &[Reg],
) -> Option<Vec<Reg>> {
    let mut choose_gen = ChooseGen::new(regs.len(), num_choices);
    let mut seen = HashSet::new();
    'choices: while let Some(choice_indices) = choose_gen.next() {
        let choices: Vec<Reg> = choice_indices.iter().map(|&i| regs[i]).collect();
        seen.clear();
        for lane in 0..trace.width() {
            let reg_values: Vec<u32> = choices
                .iter()
                .chain(extra_regs)
                .map(|&reg| trace[reg][lane])
                .collect();
            if !seen.insert(reg_values) {
                continue 'choices;
            }
        }
        return Some(choices);
    }
    None
}
