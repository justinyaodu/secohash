pub struct ChooseGen {
    pub n: usize,
    pub choices: Vec<usize>,
}

impl ChooseGen {
    pub fn new(n: usize, k: usize) -> ChooseGen {
        assert!(k <= n);
        ChooseGen {
            n,
            choices: (0..k).collect(),
        }
    }

    pub fn next(&mut self) -> bool {
        let mut end = self.choices.len();
        while end > 0 {
            self.choices[end - 1] += 1;
            if self.choices[end - 1] < self.n - (self.choices.len() - end) {
                for i in end..self.choices.len() {
                    self.choices[i] = self.choices[i - 1] + 1;
                }
                return true;
            }
            end -= 1;
        }
        false
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::comb::Comb;
    use std::collections::HashSet;

    fn all_choices(mut gen: ChooseGen) -> Vec<Vec<usize>> {
        let mut choices = Vec::new();
        loop {
            choices.push(gen.choices.clone());
            if !gen.next() {
                break;
            }
        }
        choices
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
        assert_eq!(
            all_choices(ChooseGen::new(3, 3)),
            vec![vec![0, 1, 2]]
        );
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
