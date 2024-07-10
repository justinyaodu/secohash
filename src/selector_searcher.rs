use std::collections::{HashMap, HashSet};

use crate::{
    choose_gen::ChooseGen,
    phf::{Phf, Reg},
    selector::Selector,
};

pub fn selector_search_2(phf: &Phf) -> Option<(Phf, Vec<Reg>)> {
    let mut simple_selectors = Vec::new();
    for i in 0..phf.min_nonzero_key_len {
        simple_selectors.push(Selector::Index(i));
    }
    for i in 1..=phf.min_nonzero_key_len {
        simple_selectors.push(Selector::Sub(i));
    }
    for i in 0..phf.min_nonzero_key_len {
        simple_selectors.push(Selector::And(i));
    }
    for i in 1..32 {
        if (phf.max_key_len >> i) == 0 {
            break;
        }
        simple_selectors.push(Selector::Shrl(i));
    }

    let mut keys_by_len: HashMap<usize, Vec<Vec<u32>>> = HashMap::new();
    for key in &phf.interpreted_keys {
        keys_by_len.entry(key.len()).or_default().push(key.clone())
    }

    for num_selectors in 1..=4 {
        for num_non_simple_selectors in 0..=num_selectors {
            let num_simple_selectors = num_selectors - num_non_simple_selectors;
            if num_simple_selectors > simple_selectors.len() {
                break;
            }

            let has_len_selector = num_non_simple_selectors > 0;
            let num_table_selectors = if has_len_selector {
                num_non_simple_selectors - 1
            } else {
                0
            };

            let mut simple_choose_gen =
                ChooseGen::new(simple_selectors.len(), num_simple_selectors);
            loop {
                'outer: {
                    if num_table_selectors > 0 {
                        let mut tables = vec![vec![0u8; phf.max_key_len + 1]; num_table_selectors];
                        'inner: for (&len, keys_with_len) in keys_by_len.iter() {
                            let mut table_choose_gen =
                                ChooseGen::new(len, usize::min(len, num_table_selectors));
                            loop {
                                let mut choices: Vec<Selector> = simple_choose_gen
                                    .choices
                                    .iter()
                                    .map(|&i| simple_selectors[i].clone())
                                    .collect();
                                if has_len_selector {
                                    choices.push(Selector::Len);
                                }
                                table_choose_gen
                                    .choices
                                    .iter()
                                    .for_each(|&i| choices.push(Selector::Index(i)));

                                if is_solution(keys_with_len, &choices) {
                                    table_choose_gen
                                        .choices
                                        .iter()
                                        .enumerate()
                                        .for_each(|(i, &x)| tables[i][len] = x.try_into().unwrap());
                                    continue 'inner;
                                }

                                if !table_choose_gen.next() {
                                    break 'outer;
                                }
                            }
                        }

                        let mut choices: Vec<Selector> = simple_choose_gen
                            .choices
                            .iter()
                            .map(|&i| simple_selectors[i].clone())
                            .collect();
                        if has_len_selector {
                            choices.push(Selector::Len);
                        }
                        tables
                            .into_iter()
                            .for_each(|t| choices.push(Selector::Table(t)));
                        return Some(compile_selectors(phf, &choices));
                    } else {
                        let mut choices: Vec<Selector> = simple_choose_gen
                            .choices
                            .iter()
                            .map(|&i| simple_selectors[i].clone())
                            .collect();
                        if has_len_selector {
                            choices.push(Selector::Len);
                        }
                        if is_solution(&phf.interpreted_keys, &choices) {
                            return Some(compile_selectors(phf, &choices));
                        }
                    }
                }

                if !simple_choose_gen.next() {
                    break;
                }
            }
        }
    }

    None
}

fn compile_selectors(phf: &Phf, selectors: &[Selector]) -> (Phf, Vec<Reg>) {
    let mut phf = phf.clone();
    let mut sel_regs = Vec::new();
    for selector in selectors {
        sel_regs.push(selector.compile(&mut phf));
    }
    (phf, sel_regs)
}

pub fn selector_search(phf: &Phf) -> Option<Vec<Selector>> {
    {
        let mut selectors = Vec::new();
        for i in 0..phf.min_nonzero_key_len {
            selectors.push(Selector::Index(i));
        }
        for i in 1..=phf.min_nonzero_key_len {
            selectors.push(Selector::Sub(i));
        }
        for i in 0..phf.min_nonzero_key_len {
            selectors.push(Selector::And(i));
        }
        for i in 1..32 {
            if (phf.max_key_len >> i) == 0 {
                break;
            }
            selectors.push(Selector::Shrl(i));
        }

        for choices in 1..4 {
            let opt = search_rec(&phf.interpreted_keys, &selectors, Vec::new(), choices);
            if opt.is_some() {
                return opt;
            }
        }
    }

    let mut keys_by_len: HashMap<usize, Vec<Vec<u32>>> = HashMap::new();
    for key in &phf.interpreted_keys {
        keys_by_len.entry(key.len()).or_default().push(key.clone())
    }
    for choices in 1..4 {
        let mut tables = vec![vec![0u8; phf.max_key_len + 1]; choices];
        let mut solved = true;
        for (&len, keys_with_len) in keys_by_len.iter() {
            let mut selectors = Vec::new();
            for i in 0..len {
                selectors.push(Selector::Index(i));
            }
            let opt = search_rec(keys_with_len, &selectors, vec![Selector::Len], choices);
            match opt {
                Some(sels) => {
                    for (i, sel) in sels.iter().skip(1).enumerate() {
                        let Selector::Index(index) = *sel else {
                            panic!();
                        };
                        tables[i][len] = index.try_into().unwrap();
                    }
                }
                None => {
                    solved = false;
                    break;
                }
            }
        }
        if solved {
            let mut selectors = vec![Selector::Len];
            selectors.extend(tables.into_iter().map(Selector::Table));
            return Some(selectors);
        }
    }

    None
}

fn search_rec(
    keys: &[Vec<u32>],
    selectors: &[Selector],
    chosen: Vec<Selector>,
    choices: usize,
) -> Option<Vec<Selector>> {
    if choices == 0 || selectors.is_empty() {
        return if is_solution(keys, &chosen) {
            Some(chosen)
        } else {
            None
        };
    }

    for (i, sel) in selectors.iter().enumerate() {
        let mut new_chosen = chosen.clone();
        new_chosen.push(sel.clone());
        let opt = search_rec(keys, &selectors[i + 1..], new_chosen, choices - 1);
        if opt.is_some() {
            return opt;
        }
    }

    None
}

fn is_solution(keys: &[Vec<u32>], selectors: &[Selector]) -> bool {
    let mut set = HashSet::new();
    for key in keys {
        let selected: Vec<u32> = selectors.iter().map(|s| s.eval(key)).collect();
        if !set.insert(selected) {
            return false;
        }
    }
    true
}
