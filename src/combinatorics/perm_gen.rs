pub struct PermGen {
    pub n: usize,
    pub perm: Vec<usize>,
}

impl PermGen {
    pub fn new(n: usize) -> PermGen {
        let mut perm: Vec<usize> = (0..n).collect();
        perm.push(usize::MAX);
        PermGen { n, perm }
    }

    pub fn next(&mut self) -> usize {
        let PermGen { n, ref mut perm } = *self;

        if perm.len() != n {
            perm.pop();
            return n;
        }

        let mut tail_len = 1;
        while tail_len < n {
            if perm[n - tail_len - 1] < perm[n - tail_len] {
                break;
            }
            tail_len += 1;
        }
        if tail_len >= n {
            return n + 1;
        }

        let swap_left = n - 1 - tail_len;
        let mut swap_right = n - 1;
        while perm[swap_right] < perm[swap_left] {
            swap_right -= 1;
        }

        perm.swap(swap_left, swap_right);
        perm[swap_left + 1..].reverse();
        tail_len + 1
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::collections::HashSet;

    fn all_perms_and_change_counts(mut gen: PermGen) -> Vec<(Vec<usize>, usize)> {
        let mut all = Vec::new();
        loop {
            let change_count = gen.next();
            if change_count > gen.n {
                break;
            }
            all.push((gen.perm.clone(), change_count));
        }
        all
    }

    #[test]
    fn test_0() {
        assert_eq!(
            all_perms_and_change_counts(PermGen::new(0)),
            vec![(Vec::new(), 0)]
        )
    }

    #[test]
    fn test_1() {
        assert_eq!(
            all_perms_and_change_counts(PermGen::new(1)),
            vec![(vec![0], 1)]
        )
    }

    #[test]
    fn test_2() {
        assert_eq!(
            all_perms_and_change_counts(PermGen::new(2)),
            vec![(vec![0, 1], 2), (vec![1, 0], 2)]
        )
    }

    #[test]
    fn test_3() {
        assert_eq!(
            all_perms_and_change_counts(PermGen::new(3)),
            vec![
                (vec![0, 1, 2], 3),
                (vec![0, 2, 1], 2),
                (vec![1, 0, 2], 3),
                (vec![1, 2, 0], 2),
                (vec![2, 0, 1], 3),
                (vec![2, 1, 0], 2),
            ]
        )
    }

    #[test]
    fn test_4() {
        assert_eq!(
            all_perms_and_change_counts(PermGen::new(4)),
            vec![
                (vec![0, 1, 2, 3], 4),
                (vec![0, 1, 3, 2], 2),
                (vec![0, 2, 1, 3], 3),
                (vec![0, 2, 3, 1], 2),
                (vec![0, 3, 1, 2], 3),
                (vec![0, 3, 2, 1], 2),
                (vec![1, 0, 2, 3], 4),
                (vec![1, 0, 3, 2], 2),
                (vec![1, 2, 0, 3], 3),
                (vec![1, 2, 3, 0], 2),
                (vec![1, 3, 0, 2], 3),
                (vec![1, 3, 2, 0], 2),
                (vec![2, 0, 1, 3], 4),
                (vec![2, 0, 3, 1], 2),
                (vec![2, 1, 0, 3], 3),
                (vec![2, 1, 3, 0], 2),
                (vec![2, 3, 0, 1], 3),
                (vec![2, 3, 1, 0], 2),
                (vec![3, 0, 1, 2], 4),
                (vec![3, 0, 2, 1], 2),
                (vec![3, 1, 0, 2], 3),
                (vec![3, 1, 2, 0], 2),
                (vec![3, 2, 0, 1], 3),
                (vec![3, 2, 1, 0], 2),
            ]
        )
    }

    fn fact(n: usize) -> usize {
        if n == 0 {
            1
        } else {
            n * fact(n - 1)
        }
    }

    fn validate(n: usize) {
        let all = all_perms_and_change_counts(PermGen::new(n));

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

        for (i, &(ref perm, change_count)) in all.iter().enumerate() {
            if i == 0 {
                assert!(change_count == n);
            } else {
                let split = n - change_count;
                let prev_perm = &all[i - 1].0;
                assert_eq!(perm[..split], prev_perm[..split]);
                assert_ne!(perm[split], prev_perm[split]);
            }

            assert_eq!(
                {
                    let mut sorted = perm.clone();
                    sorted.sort();
                    sorted
                },
                (0..n).collect::<Vec<_>>()
            );
        }

        assert_eq!(all.len(), fact(n));
    }

    #[test]
    fn test_many() {
        for n in 1..=8 {
            validate(n);
        }
    }
}
