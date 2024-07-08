use crate::ir::{ExprBuilder, Ir, Reg};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Selector {
    Len,
    Index(u32),
    Table(Vec<u8>),
    Sub(u32),
    And(u32),
    Shrl(u32),
}

impl Selector {
    pub fn eval(&self, key: &[u32]) -> u32 {
        match *self {
            Selector::Len => key.len() as u32,
            Selector::Index(i) => key[i as usize],
            Selector::Table(ref t) => key[t[key.len()] as usize],
            Selector::Sub(k) => key[key.len() - (k as usize)],
            Selector::And(k) => key[key.len() & (k as usize)],
            Selector::Shrl(k) => key[key.len() >> k],
        }
    }

    pub fn compile(&self, ir: &mut Ir) -> Reg {
        let e = ExprBuilder();
        let expr = match *self {
            Selector::Len => e.str_len(),
            Selector::Index(i) => e.str_get(e.imm(i)),
            Selector::Table(ref t) => {
                let t = ir.table(t.to_vec());
                e.str_get(e.table_get(t, e.str_len()))
            }
            Selector::Sub(k) => e.str_get(e.sub(e.str_len(), e.imm(k))),
            Selector::And(k) => e.str_get(e.and(e.str_len(), e.imm(k))),
            Selector::Shrl(k) => e.str_get(e.shrl(e.str_len(), e.imm(k))),
        };
        ir.expr(expr)
    }
}
