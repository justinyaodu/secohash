use std::fmt::Display;

#[derive(Clone)]
pub struct Pos {
    line: usize,
    col: usize,
}

impl Pos {
    pub fn new() -> Pos {
        Pos { line: 0, col: 0 }
    }

    pub fn advance(&mut self, c: char) {
        if c == '\n' {
            self.line += 1;
            self.col = 0;
        } else {
            self.col += 1;
        }
    }
}

impl Display for Pos {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "line {} col {}", self.line + 1, self.col + 1)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_pos() {
        let mut pos = Pos::new();
        assert_eq!(pos.to_string(), "line 1 col 1");
        pos.advance('a');
        assert_eq!(pos.to_string(), "line 1 col 2");
        pos.advance('\n');
        assert_eq!(pos.to_string(), "line 2 col 1");
        pos.advance('\n');
        assert_eq!(pos.to_string(), "line 3 col 1");
    }
}
