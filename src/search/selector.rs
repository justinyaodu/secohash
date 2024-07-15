use crate::phf::{ExprBuilder, Phf, Reg};

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
    pub fn eval(&self, key: &[u32]) -> u32 {
        match *self {
            Selector::Index(i) => key[i],
            Selector::Sub(k) => key[key.len() - k],
            Selector::And(k) => key[key.len() & k],
            Selector::Shrl(k) => key[key.len() >> k],
            Selector::Len => key.len() as u32,
            Selector::Table(ref t) => key[usize::from(t[key.len()])],
        }
    }

    pub fn compile(&self, phf: &mut Phf) -> Reg {
        let e = ExprBuilder();
        let expr = match *self {
            Selector::Index(i) => e.str_get(e.imm(i.try_into().unwrap())),
            Selector::Sub(k) => e.str_get(e.sub(e.str_len(), e.imm(k.try_into().unwrap()))),
            Selector::And(k) => e.str_get(e.and(e.str_len(), e.imm(k.try_into().unwrap()))),
            Selector::Shrl(k) => e.str_get(e.shrl(e.str_len(), e.imm(k))),
            Selector::Len => e.str_len(),
            Selector::Table(ref t) => {
                let t = phf.push_data_table(t.to_vec());
                e.str_get(e.table_get(t, e.str_len()))
            }
        };
        phf.push_expr(expr)
    }
}
