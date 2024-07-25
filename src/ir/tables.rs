use std::ops::Index;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Table(pub usize);

pub struct Tables(Vec<Vec<u8>>);

impl Tables {
    pub fn new() -> Tables {
        Tables(Vec::new())
    }

    pub fn push(&mut self, table: Vec<u8>) -> Table {
        self.0.push(table);
        Table(self.0.len() - 1)
    }

    pub fn tables(&self) -> &[Vec<u8>] {
        &self.0
    }
}

impl Index<Table> for Tables {
    type Output = Vec<u8>;

    fn index(&self, index: Table) -> &Self::Output {
        &self.0[index.0]
    }
}
