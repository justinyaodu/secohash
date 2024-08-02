use std::{collections::HashSet, iter};

use crate::{
    combinatorics::{ChooseGen, LendingIterator},
    ir::{ExprBuilder, Reg, Tables, Tac},
    spec::Spec,
    util::{to_u32, to_usize},
};

#[derive(Clone, Debug)]
pub enum Selector {
    Len,
    Index(u32),
    Sub(u32),
    And(u32),
    Shrl(u32),
    StrSum(u32),
    Table(Vec<u32>),
}

impl Selector {
    fn eval(&self, keys: &[Vec<u32>]) -> Vec<u32> {
        let mut buf = vec![0u32; keys.len()];
        match *self {
            Selector::Len => {
                for (i, key) in keys.iter().enumerate() {
                    buf[i] = to_u32(key.len());
                }
            }
            Selector::Index(k) => {
                let k = to_usize(k);
                for (i, key) in keys.iter().enumerate() {
                    if k < key.len() {
                        buf[i] = key[k];
                    }
                }
            }
            Selector::Sub(k) => {
                let k = to_usize(k);
                for (i, key) in keys.iter().enumerate() {
                    buf[i] = key[key.len() - k];
                }
            }
            Selector::And(k) => {
                let k = to_usize(k);
                for (i, key) in keys.iter().enumerate() {
                    buf[i] = key[key.len() & k];
                }
            }
            Selector::Shrl(k) => {
                for (i, key) in keys.iter().enumerate() {
                    buf[i] = key[key.len() >> k];
                }
            }
            Selector::StrSum(mask) => {
                let k = to_usize(mask);
                for (i, key) in keys.iter().enumerate() {
                    let mut sum = 0u32;
                    for (j, char) in key.iter().enumerate() {
                        sum = sum.wrapping_add(char << (j & k));
                    }
                    buf[i] = sum;
                }
            }
            Selector::Table(_) => panic!(),
        }
        buf
    }

    pub fn compile(self, tac: &mut Tac, tables: &mut Tables) -> Reg {
        let x = ExprBuilder();
        let expr = match self {
            Selector::Len => x.str_len(),
            Selector::Index(i) => x.str_get(x.imm(i)),
            Selector::Sub(k) => x.str_get(x.sub(x.str_len(), x.imm(k))),
            Selector::And(k) => x.str_get(x.and(x.str_len(), x.imm(k))),
            Selector::Shrl(k) => x.str_get(x.shrl(x.str_len(), x.imm(k))),
            Selector::StrSum(k) => x.str_sum(k),
            Selector::Table(t) => {
                let t = tables.push(t);
                x.str_get(x.table_get(t, x.str_len()))
            }
        };
        tac.push_expr(expr)
    }

    pub fn search(spec: &Spec) -> Option<Vec<Selector>> {
        let pos_limit = 32;

        let mut searcher = SelectorSearcher::new(&spec.interpreted_keys);

        let mut len_sels = Vec::new();
        let len_not_constant = spec.min_interpreted_key_len < spec.max_interpreted_key_len;
        if len_not_constant {
            len_sels.push(searcher.add_selector(Selector::Len));
        }

        let mut safe_index_sels = Vec::new();
        let mut all_index_sels = Vec::new();
        for k in 0..usize::min(spec.max_interpreted_key_len, pos_limit) {
            let sel = searcher.add_selector(Selector::Index(to_u32(k)));
            if k < spec.min_interpreted_key_len {
                safe_index_sels.push(sel);
            }
            all_index_sels.push(sel);
        }

        let mut arith_sels = Vec::new();
        if len_not_constant {
            for k in 1..=usize::min(spec.min_interpreted_key_len, pos_limit) {
                arith_sels.push(searcher.add_selector(Selector::Sub(to_u32(k))));
            }
            for k in 1..usize::min(spec.min_interpreted_key_len, pos_limit) {
                arith_sels.push(searcher.add_selector(Selector::And(to_u32(k))));
            }
            for k in 1..32 {
                if (spec.max_interpreted_key_len >> k) == 0 {
                    break;
                }
                arith_sels.push(searcher.add_selector(Selector::Shrl(k)));
            }
        }

        let choices = 'choices: {
            let search_exponent = 3;

            let index_arith_sels: Vec<_> = safe_index_sels
                .iter()
                .chain(arith_sels.iter())
                .copied()
                .collect();

            for num_choices in 0..=search_exponent {
                if let Some(choices) =
                    searcher.find_distinguishing(&[], &safe_index_sels, num_choices, None)
                {
                    break 'choices choices;
                }

                if len_not_constant {
                    if let Some(choices) =
                        searcher.find_distinguishing(&[], &index_arith_sels, num_choices, None)
                    {
                        break 'choices choices;
                    }

                    if let Some(choices) =
                        searcher.find_distinguishing(&len_sels, &safe_index_sels, num_choices, None)
                    {
                        break 'choices choices;
                    }

                    if let Some(choices) = searcher.find_distinguishing(
                        &len_sels,
                        &index_arith_sels,
                        num_choices,
                        None,
                    ) {
                        break 'choices choices;
                    }
                }
            }

            let mut sum_sels = Vec::new();
            for mask in 0..32 {
                sum_sels.push(searcher.add_selector(Selector::StrSum(mask)));
            }

            for &sum_sel in &sum_sels {
                for num_choices in 0..=2 {
                    if let Some(choices) = searcher.find_distinguishing(
                        &[sum_sel],
                        &index_arith_sels,
                        num_choices,
                        None,
                    ) {
                        break 'choices choices;
                    }
                }
            }

            return None;
        };

        Some(
            choices
                .iter()
                .map(|&choice| searcher.selectors[choice].clone())
                .collect(),
        )
    }

    /*
    fn find_distinguishing(
        keys: &[Vec<u32>],
        sels: &[Selector],
        cols: &mut [Vec<u32>],
        already_chosen: &[usize],
        choosable: &[usize],
        num_choices: usize,
        row_range: Option<(usize, usize)>,
        seen: &mut HashSet<Vec<u32>>,
    ) -> Option<Vec<usize>> {
        if num_choices > choosable.len() {
            return None;
        }

        let row_range = row_range.unwrap_or((0, keys.len()));

        let mut choices: Vec<_> = iter::repeat(0)
            .take(num_choices)
            .chain(already_chosen.iter().copied())
            .collect();
        let mut choose_gen = ChooseGen::new(choosable.len(), num_choices);
        while let Some(chosen_indices) = choose_gen.next() {
            for (i, &chosen_index) in chosen_indices.iter().enumerate() {
                let choice = choosable[chosen_index];
                choices[i] = choice;
            }
            // for &choice in &choices {
            //     if cols[choice].is_empty() {
            //         cols[choice] = sels[choice].eval(keys);
            //         eprintln!("evaluated {:?}", sels[choice]);
            //     }
            // }
            if Self::distinguishes(cols, &choices, row_range, seen) {
                return Some(choices);
            }
        }
        None
    }

    fn distinguishes(
        cols: &[Vec<u32>],
        choices: &[usize],
        row_range: (usize, usize),
        seen: &mut HashSet<Vec<u32>>,
    ) -> bool {
        seen.clear();
        let (start_row, end_row) = row_range;
        for row in start_row..end_row {
            let vec = choices.iter().map(|&choice| cols[choice][row]).collect();
            if !seen.insert(vec) {
                return false;
            }
        }
        true
    }
    */
}

struct SelectorSearcher<'a> {
    keys: &'a [Vec<u32>],
    selectors: Vec<Selector>,
    cols: Vec<Vec<u32>>,
    seen: HashSet<Vec<u32>>,
}

impl SelectorSearcher<'_> {
    fn new(keys: &[Vec<u32>]) -> SelectorSearcher {
        SelectorSearcher {
            keys,
            selectors: Vec::new(),
            cols: Vec::new(),
            seen: HashSet::with_capacity(keys.len()),
        }
    }

    fn add_selector(&mut self, selector: Selector) -> usize {
        let i = self.selectors.len();
        self.cols.push(selector.eval(self.keys));
        self.selectors.push(selector);
        i
    }

    fn find_distinguishing(
        &mut self,
        already_chosen: &[usize],
        choosable: &[usize],
        num_choices: usize,
        row_range: Option<(usize, usize)>,
    ) -> Option<Vec<usize>> {
        if num_choices > choosable.len() {
            return None;
        }

        let row_range = row_range.unwrap_or((0, self.keys.len()));

        let mut choices: Vec<_> = iter::repeat(0)
            .take(num_choices)
            .chain(already_chosen.iter().copied())
            .collect();
        let mut choose_gen = ChooseGen::new(choosable.len(), num_choices);
        while let Some(chosen_indices) = choose_gen.next() {
            for (i, &chosen_index) in chosen_indices.iter().enumerate() {
                choices[i] = choosable[chosen_index];
            }
            if self.distinguishes(&choices, row_range) {
                return Some(choices);
            }
        }
        None
    }

    fn distinguishes(&mut self, choices: &[usize], row_range: (usize, usize)) -> bool {
        self.seen.clear();
        let (start_row, end_row) = row_range;
        for row in start_row..end_row {
            let vec = choices
                .iter()
                .map(|&choice| self.cols[choice][row])
                .collect();
            if !self.seen.insert(vec) {
                return false;
            }
        }
        true
    }
}
