use super::LendingIterator;

pub struct ChooseGen {
    n: usize,
    k: usize,
    choices: Vec<usize>,
}

impl ChooseGen {
    pub fn new(n: usize, k: usize) -> ChooseGen {
        assert!(k <= n);
        let mut choices: Vec<usize> = (0..k).collect();
        choices.push(usize::MAX);
        ChooseGen { n, k, choices }
    }
}

impl LendingIterator for ChooseGen {
    type Item<'a> = &'a [usize];

    fn next(&mut self) -> Option<Self::Item<'_>> {
        let Self {
            n,
            k,
            ref mut choices,
        } = *self;

        if choices.len() != k {
            choices.pop();
            return Some(&self.choices);
        }

        let mut i = k;
        while i > 0 {
            choices[i - 1] += 1;
            if choices[i - 1] + k - i < n {
                for i in i..k {
                    choices[i] = choices[i - 1] + 1;
                }
                return Some(&self.choices);
            }
            i -= 1;
        }
        None
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::combinatorics::comb::Comb;
    use std::collections::HashSet;

    fn all_choices(mut gen: ChooseGen) -> Vec<Vec<usize>> {
        let mut all = Vec::new();
        while let Some(choices) = gen.next() {
            all.push(choices.to_vec());
        }
        all
    }

    #[test]
    fn test_0_0() {
        assert_eq!(all_choices(ChooseGen::new(0, 0)), vec![vec![]]);
    }

    #[test]
    fn test_1_0() {
        assert_eq!(all_choices(ChooseGen::new(1, 0)), vec![vec![]]);
    }

    #[test]
    fn test_1_1() {
        assert_eq!(all_choices(ChooseGen::new(1, 1)), vec![vec![0]]);
    }

    #[test]
    fn test_2_1() {
        assert_eq!(all_choices(ChooseGen::new(2, 1)), vec![vec![0], vec![1]]);
    }

    #[test]
    fn test_3_2() {
        assert_eq!(
            all_choices(ChooseGen::new(3, 2)),
            vec![vec![0, 1], vec![0, 2], vec![1, 2]]
        );
    }

    #[test]
    fn test_3_3() {
        assert_eq!(all_choices(ChooseGen::new(3, 3)), vec![vec![0, 1, 2]]);
    }

    fn validate(comb: &mut Comb, n: usize, k: usize) {
        let all = all_choices(ChooseGen::new(n, k));

        assert!(all.len() == comb.comb(n, k));

        assert_eq!(
            all,
            {
                let mut sorted = all.clone();
                sorted.sort();
                sorted
            },
            "not in lexicographical order"
        );

        assert_eq!(
            all.len(),
            all.iter().cloned().collect::<HashSet<_>>().len(),
            "not distinct"
        );

        for choices in &all {
            for &choice in choices {
                assert!(choice < n);
            }
        }
    }

    #[test]
    fn test_many() {
        let mut comb = Comb::new();
        for n in 0..10 {
            for k in 0..=n {
                validate(&mut comb, n, k);
            }
        }
    }
}
