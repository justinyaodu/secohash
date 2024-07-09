use std::collections::{HashMap, HashSet};

use crate::{phf::Phf, selector::Selector};

pub fn selector_search(phf: &Phf) -> Option<Vec<Selector>> {
    {
        let mut selectors = Vec::new();
        selectors.push(Selector::Len);
        for i in 0..phf.min_nonzero_key_len {
            selectors.push(Selector::Index(i as u32));
        }
        for i in 1..=phf.min_nonzero_key_len {
            selectors.push(Selector::Sub(i as u32));
        }
        for i in 0..phf.min_nonzero_key_len {
            selectors.push(Selector::And(i as u32));
        }
        for i in 1.. {
            if (phf.max_key_len >> i) == 0 {
                break;
            }
            selectors.push(Selector::Shrl(i as u32));
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
                selectors.push(Selector::Index(i as u32));
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
