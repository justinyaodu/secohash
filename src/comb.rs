#[cfg(test)]
pub struct Comb(Vec<Vec<usize>>);

#[cfg(test)]
impl Comb {
    pub fn new() -> Comb {
        Comb(vec![vec![1]])
    }

    pub fn comb(&mut self, n: usize, k: usize) -> usize {
        assert!(k <= n);

        for next_n in self.0.len()..=n {
            let mut next_row = Vec::new();
            next_row.push(1);
            for next_k in 1..next_n {
                next_row.push(self.0[next_n - 1][next_k - 1] + self.0[next_n - 1][next_k])
            }
            next_row.push(1);
            self.0.push(next_row);
        }
        self.0[n][k]
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_comb() {
        let mut comb = Comb::new();
        assert_eq!(comb.comb(0, 0), 1);
        assert_eq!(comb.comb(1, 0), 1);
        assert_eq!(comb.comb(1, 1), 1);
        assert_eq!(comb.comb(2, 0), 1);
        assert_eq!(comb.comb(2, 1), 2);
        assert_eq!(comb.comb(2, 2), 1);
        assert_eq!(comb.comb(3, 0), 1);
        assert_eq!(comb.comb(3, 1), 3);
        assert_eq!(comb.comb(3, 2), 3);
        assert_eq!(comb.comb(3, 3), 1);
        assert_eq!(comb.comb(4, 0), 1);
        assert_eq!(comb.comb(4, 1), 4);
        assert_eq!(comb.comb(4, 2), 6);
        assert_eq!(comb.comb(4, 3), 4);
        assert_eq!(comb.comb(4, 4), 1);
    }
}
