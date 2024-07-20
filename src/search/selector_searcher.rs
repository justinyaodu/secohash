use std::collections::{HashMap, HashSet};

use crate::{
    combinatorics::{ChooseGen, LendingIterator},
    phf::{Phf, Reg},
    search::selector::Selector,
};

pub fn selector_search(phf: &Phf) -> Option<(Phf, Vec<Reg>)> {
    if let Some(selectors) = combined_search(phf) {
        let mut phf = phf.clone();
        let mut sel_regs = Vec::new();
        for selector in selectors {
            sel_regs.push(selector.compile(&mut phf));
        }
        Some((phf, sel_regs))
    } else {
        None
    }
}

fn combined_search(phf: &Phf) -> Option<Vec<Selector>> {
    let sol = basic_search(phf);
    if sol.is_some() {
        return sol;
    }

    let mut keys_by_len: HashMap<usize, Vec<Vec<u32>>> = HashMap::new();
    for key in &phf.interpreted_keys {
        keys_by_len.entry(key.len()).or_default().push(key.clone())
    }

    for num_tables in 1..=3 {
        let sol = table_search(phf, &keys_by_len, num_tables);
        if sol.is_some() {
            return sol;
        }
    }

    None
}

fn basic_search(phf: &Phf) -> Option<Vec<Selector>> {
    let index_bound = usize::min(phf.min_nonzero_key_len, 32);

    let mut selectors_1 = Vec::new();
    selectors_1.push(Selector::Len);
    for i in 0..index_bound {
        selectors_1.push(Selector::Index(i));
    }

    let mut selectors_2 = selectors_1.clone();
    for i in 1..=index_bound {
        selectors_2.push(Selector::Sub(i));
    }
    for i in 0..index_bound {
        selectors_2.push(Selector::And(i));
    }
    for i in 1..32 {
        if (phf.max_key_len >> i) == 0 {
            break;
        }
        selectors_2.push(Selector::Shrl(i));
    }

    for num_choices in 1..=4 {
        for selectors in [&selectors_1, &selectors_2] {
            if num_choices > selectors.len() {
                continue;
            }
            let sol = find_distinguishing_selectors(&phf.interpreted_keys, selectors, num_choices);
            if sol.is_some() {
                return sol;
            }
        }
    }

    None
}

fn table_search(
    phf: &Phf,
    keys_by_len: &HashMap<usize, Vec<Vec<u32>>>,
    num_tables: usize,
) -> Option<Vec<Selector>> {
    let mut tables = vec![vec![0u8; phf.max_key_len + 1]; num_tables];

    for (&len, keys_with_len) in keys_by_len.iter() {
        let index_selectors: Vec<Selector> =
            (0..usize::min(len, 32)).map(Selector::Index).collect();

        let num_choices = usize::min(num_tables, index_selectors.len());

        if let Some(chosen) =
            find_distinguishing_selectors(keys_with_len, &index_selectors, num_choices)
        {
            for (i, choice) in chosen.into_iter().enumerate() {
                let index = match choice {
                    Selector::Index(index) => index,
                    _ => panic!(),
                };
                tables[i][len] = index.try_into().unwrap();
            }
        } else {
            return None;
        }
    }

    let mut chosen = Vec::new();
    chosen.push(Selector::Len);
    for table in tables {
        chosen.push(Selector::Table(table));
    }
    Some(chosen)
}

fn find_distinguishing_selectors(
    keys: &[Vec<u32>],
    selectors: &[Selector],
    k: usize,
) -> Option<Vec<Selector>> {
    let n = selectors.len();
    assert!(n >= k);
    let mut choose_gen = ChooseGen::new(n, k);
    while let Some(choices) = choose_gen.next() {
        let chosen: Vec<Selector> = choices
            .iter()
            .map(|&i| selectors[i].clone())
            .collect();
        if is_solution(keys, &chosen) {
            return Some(chosen);
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
