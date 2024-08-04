#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BinOp {
    Add,
    Sub,
    And,
    Shll,
    Shrl,
}

impl BinOp {
    pub fn eval(&self, a: u32, b: u32) -> u32 {
        match self {
            BinOp::Add => a.wrapping_add(b),
            BinOp::Sub => a.wrapping_sub(b),
            BinOp::And => a & b,
            BinOp::Shll => a << b,
            BinOp::Shrl => a >> b,
        }
    }
}
