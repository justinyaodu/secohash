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
        let x = ExprBuilder();
        let expr = match *self {
            Selector::Index(i) => x.str_get(x.imm(i.try_into().unwrap())),
            Selector::Sub(k) => x.str_get(x.sub(x.str_len(), x.imm(k.try_into().unwrap()))),
            Selector::And(k) => x.str_get(x.and(x.str_len(), x.imm(k.try_into().unwrap()))),
            Selector::Shrl(k) => x.str_get(x.shrl(x.str_len(), x.imm(k))),
            Selector::Len => x.str_len(),
            Selector::Table(ref t) => {
                let t = phf.push_data_table(t.to_vec());
                x.str_get(x.table_get(t, x.str_len()))
            }
        };
        phf.push_expr(expr)
    }
}
