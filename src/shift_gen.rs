pub struct ShiftGen {
    pub shifts: Vec<u32>,
    max_shift: u32,
}

impl ShiftGen {
    pub fn new(num_shifts: u32, num_nonzero_shifts: u32, max_shift: u32) -> ShiftGen {
        assert!(num_nonzero_shifts <= num_shifts);
        assert!(max_shift > 0);

        let mut shifts = vec![0; num_shifts as usize];
        for i in 0..(num_nonzero_shifts as usize) {
            let index = shifts.len() - 1 - i;
            shifts[index] = 1;
        }
        ShiftGen { shifts, max_shift }
    }

    pub fn next(&mut self) -> bool {
        for i in (0..self.shifts.len()).rev() {
            if self.shifts[i] == 0 {
                continue;
            }
            if self.shifts[i] == self.max_shift {
                if i > 0 && self.shifts[i - 1] == 0 {
                    self.shifts[i - 1] = 1;
                    self.shifts[i] = 0;
                } else {
                    self.shifts[i] = 1;
                    continue;
                }
            } else {
                self.shifts[i] += 1;
            }
            self.shifts[i + 1..].reverse();
            return true;
        }
        false
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashSet;

    use super::*;
    use crate::comb::Comb;

    fn all_shifts(mut gen: ShiftGen) -> Vec<Vec<u32>> {
        let mut shifts = Vec::new();
        loop {
            shifts.push(gen.shifts.clone());
            if !gen.next() {
                break;
            }
        }
        shifts
    }

    #[test]
    fn test_no_shifts() {
        assert_eq!(all_shifts(ShiftGen::new(0, 0, 1)), vec![vec![]]);
        assert_eq!(all_shifts(ShiftGen::new(0, 0, 3)), vec![vec![]]);
    }

    #[test]
    fn test_all_zero() {
        assert_eq!(all_shifts(ShiftGen::new(3, 0, 1)), vec![vec![0, 0, 0]]);
        assert_eq!(all_shifts(ShiftGen::new(3, 0, 3)), vec![vec![0, 0, 0]]);
    }

    #[test]
    fn test_one_nonzero() {
        assert_eq!(
            all_shifts(ShiftGen::new(3, 1, 1)),
            vec![vec![0, 0, 1], vec![0, 1, 0], vec![1, 0, 0]]
        );
        assert_eq!(
            all_shifts(ShiftGen::new(3, 1, 3)),
            vec![
                vec![0, 0, 1],
                vec![0, 0, 2],
                vec![0, 0, 3],
                vec![0, 1, 0],
                vec![0, 2, 0],
                vec![0, 3, 0],
                vec![1, 0, 0],
                vec![2, 0, 0],
                vec![3, 0, 0],
            ]
        );
    }

    #[test]
    fn test_two_nonzero() {
        assert_eq!(
            all_shifts(ShiftGen::new(3, 2, 1)),
            vec![vec![0, 1, 1], vec![1, 0, 1], vec![1, 1, 0]]
        );
        assert_eq!(
            all_shifts(ShiftGen::new(3, 2, 3)),
            vec![
                vec![0, 1, 1],
                vec![0, 1, 2],
                vec![0, 1, 3],
                vec![0, 2, 1],
                vec![0, 2, 2],
                vec![0, 2, 3],
                vec![0, 3, 1],
                vec![0, 3, 2],
                vec![0, 3, 3],
                vec![1, 0, 1],
                vec![1, 0, 2],
                vec![1, 0, 3],
                vec![1, 1, 0],
                vec![1, 2, 0],
                vec![1, 3, 0],
                vec![2, 0, 1],
                vec![2, 0, 2],
                vec![2, 0, 3],
                vec![2, 1, 0],
                vec![2, 2, 0],
                vec![2, 3, 0],
                vec![3, 0, 1],
                vec![3, 0, 2],
                vec![3, 0, 3],
                vec![3, 1, 0],
                vec![3, 2, 0],
                vec![3, 3, 0],
            ]
        );
    }

    #[test]
    fn test_all_nonzero() {
        assert_eq!(all_shifts(ShiftGen::new(3, 3, 1)), vec![vec![1, 1, 1]]);
        assert_eq!(
            all_shifts(ShiftGen::new(3, 3, 3)),
            vec![
                vec![1, 1, 1],
                vec![1, 1, 2],
                vec![1, 1, 3],
                vec![1, 2, 1],
                vec![1, 2, 2],
                vec![1, 2, 3],
                vec![1, 3, 1],
                vec![1, 3, 2],
                vec![1, 3, 3],
                vec![2, 1, 1],
                vec![2, 1, 2],
                vec![2, 1, 3],
                vec![2, 2, 1],
                vec![2, 2, 2],
                vec![2, 2, 3],
                vec![2, 3, 1],
                vec![2, 3, 2],
                vec![2, 3, 3],
                vec![3, 1, 1],
                vec![3, 1, 2],
                vec![3, 1, 3],
                vec![3, 2, 1],
                vec![3, 2, 2],
                vec![3, 2, 3],
                vec![3, 3, 1],
                vec![3, 3, 2],
                vec![3, 3, 3],
            ]
        );
    }

    #[test]
    fn test_big() {
        assert_eq!(
            all_shifts(ShiftGen::new(4, 2, 2)),
            vec![
                vec![0, 0, 1, 1],
                vec![0, 0, 1, 2],
                vec![0, 0, 2, 1],
                vec![0, 0, 2, 2],
                vec![0, 1, 0, 1],
                vec![0, 1, 0, 2],
                vec![0, 1, 1, 0],
                vec![0, 1, 2, 0],
                vec![0, 2, 0, 1],
                vec![0, 2, 0, 2],
                vec![0, 2, 1, 0],
                vec![0, 2, 2, 0],
                vec![1, 0, 0, 1],
                vec![1, 0, 0, 2],
                vec![1, 0, 1, 0],
                vec![1, 0, 2, 0],
                vec![1, 1, 0, 0],
                vec![1, 2, 0, 0],
                vec![2, 0, 0, 1],
                vec![2, 0, 0, 2],
                vec![2, 0, 1, 0],
                vec![2, 0, 2, 0],
                vec![2, 1, 0, 0],
                vec![2, 2, 0, 0],
            ]
        )
    }

    fn validate(comb: &mut Comb, num_shifts: u32, num_nonzero_shifts: u32, max_shift: u32) {
        let all = all_shifts(ShiftGen::new(num_shifts, num_nonzero_shifts, max_shift));

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

        for shifts in &all {
            for &shift in shifts {
                assert!(shift <= max_shift);
            }
            assert_eq!(
                shifts.iter().filter(|&&s| s > 0).count(),
                num_nonzero_shifts as usize
            );
        }

        let expected_len = (max_shift as usize).pow(num_nonzero_shifts)
            * comb.comb(num_shifts as usize, num_nonzero_shifts as usize);
        assert_eq!(all.len(), expected_len);
    }

    #[test]
    fn test_many() {
        let mut comb = Comb::new();
        for num_shifts in 0u32..6 {
            for num_nonzero_shifts in 0..=num_shifts {
                for max_shift in 1..=4 {
                    validate(&mut comb, num_shifts, num_nonzero_shifts, max_shift);
                }
            }
        }
    }
}
