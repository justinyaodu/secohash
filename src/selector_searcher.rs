use std::collections::{HashMap, HashSet};

use crate::{keys::Keys, selector::Selector};

pub fn selector_search(keys: &Keys) -> Option<Vec<Selector>> {
    {
        let mut selectors = Vec::new();
        selectors.push(Selector::Len);
        for i in 0..keys.start_len {
            selectors.push(Selector::Index(i as u32));
        }

        for choices in 1..4 {
            let opt = search_rec(&keys.non_empty_keys, &selectors, Vec::new(), choices);
            if opt.is_some() {
                return opt;
            }
        }
    }

    let mut keys_by_len: HashMap<usize, Vec<Vec<u32>>> = HashMap::new();
    for key in &keys.non_empty_keys {
        keys_by_len.entry(key.len()).or_default().push(key.clone())
    }
    for choices in 1..4 {
        let mut tables = vec![vec![0u8; keys.end_len]; choices];
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
