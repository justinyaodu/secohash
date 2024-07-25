use super::LendingIterator;

pub struct PermGen {
    n: usize,
    perm: Vec<usize>,
}

impl PermGen {
    pub fn new(n: usize) -> PermGen {
        let mut perm: Vec<usize> = (0..n).collect();
        perm.push(usize::MAX);
        PermGen { n, perm }
    }
}

impl LendingIterator for PermGen {
    type Item<'a> = &'a [usize];

    fn next(&mut self) -> Option<Self::Item<'_>> {
        let PermGen { n, ref mut perm } = *self;

        if perm.len() != n {
            perm.pop();
            return Some(&self.perm);
        }

        let mut tail_len = 1;
        while tail_len < n {
            if perm[n - tail_len - 1] < perm[n - tail_len] {
                break;
            }
            tail_len += 1;
        }
        if tail_len >= n {
            return None;
        }

        let swap_left = n - 1 - tail_len;
        let mut swap_right = n - 1;
        while perm[swap_right] < perm[swap_left] {
            swap_right -= 1;
        }

        perm.swap(swap_left, swap_right);
        perm[swap_left + 1..].reverse();
        Some(&self.perm)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::collections::HashSet;

    fn all_perms(mut gen: PermGen) -> Vec<Vec<usize>> {
        let mut all = Vec::new();
        while let Some(perm) = gen.next() {
            all.push(perm.to_vec());
        }
        all
    }

    #[test]
    fn test_0() {
        assert_eq!(all_perms(PermGen::new(0)), vec![Vec::new()])
    }

    #[test]
    fn test_1() {
        assert_eq!(all_perms(PermGen::new(1)), vec![vec![0]])
    }

    #[test]
    fn test_2() {
        assert_eq!(all_perms(PermGen::new(2)), vec![vec![0, 1], vec![1, 0]])
    }

    #[test]
    fn test_3() {
        assert_eq!(
            all_perms(PermGen::new(3)),
            vec![
                vec![0, 1, 2],
                vec![0, 2, 1],
                vec![1, 0, 2],
                vec![1, 2, 0],
                vec![2, 0, 1],
                vec![2, 1, 0],
            ]
        )
    }

    #[test]
    fn test_4() {
        assert_eq!(
            all_perms(PermGen::new(4)),
            vec![
                vec![0, 1, 2, 3],
                vec![0, 1, 3, 2],
                vec![0, 2, 1, 3],
                vec![0, 2, 3, 1],
                vec![0, 3, 1, 2],
                vec![0, 3, 2, 1],
                vec![1, 0, 2, 3],
                vec![1, 0, 3, 2],
                vec![1, 2, 0, 3],
                vec![1, 2, 3, 0],
                vec![1, 3, 0, 2],
                vec![1, 3, 2, 0],
                vec![2, 0, 1, 3],
                vec![2, 0, 3, 1],
                vec![2, 1, 0, 3],
                vec![2, 1, 3, 0],
                vec![2, 3, 0, 1],
                vec![2, 3, 1, 0],
                vec![3, 0, 1, 2],
                vec![3, 0, 2, 1],
                vec![3, 1, 0, 2],
                vec![3, 1, 2, 0],
                vec![3, 2, 0, 1],
                vec![3, 2, 1, 0],
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
        let all = all_perms(PermGen::new(n));

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

        for perm in &all {
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
