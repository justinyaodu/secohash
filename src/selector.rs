use crate::ir::{Instr, Ir, Reg};

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum Selector {
    Len,
    Index(u32),
    Table(Vec<u8>),
}

impl Selector {
    pub fn eval(&self, key: &[u32]) -> u32 {
        match *self {
            Selector::Len => key.len() as u32,
            Selector::Index(i) => key[i as usize],
            Selector::Table(ref t) => key[t[key.len()] as usize],
        }
    }

    pub fn compile(&self, ir: &mut Ir) -> Reg {
        match *self {
            Selector::Len => ir.instr(Instr::StrLen),
            Selector::Index(i) => {
                let i = ir.instr(Instr::Imm(i));
                ir.instr(Instr::StrGet(i))
            }
            Selector::Table(ref t) => {
                let t = ir.table(t.to_vec());
                let len = ir.instr(Instr::StrLen);
                ir.instr(Instr::Table(t, len))
            }
        }
    }
}
