use crate::{
    ir::{ExprBuilder, Reg, Tables, Tac},
    util::to_u32,
};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Selector {
    Index(usize),
    Sub(usize),
    And(usize),
    Shrl(u32),
    Len,
    Table(Vec<u8>),
}

impl Selector {
    pub fn compile(self, tac: &mut Tac, tables: &mut Tables) -> Reg {
        let x = ExprBuilder();
        let expr = match self {
            Selector::Index(i) => x.str_get(x.imm(to_u32(i))),
            Selector::Sub(k) => x.str_get(x.sub(x.str_len(), x.imm(to_u32(k)))),
            Selector::And(k) => x.str_get(x.and(x.str_len(), x.imm(to_u32(k)))),
            Selector::Shrl(k) => x.str_get(x.shrl(x.str_len(), x.imm(k))),
            Selector::Len => x.str_len(),
            Selector::Table(t) => {
                let t = tables.push(t);
                x.str_get(x.table_get(t, x.str_len()))
            }
        };
        tac.push_expr(expr)
    }
}
