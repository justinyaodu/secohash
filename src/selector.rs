use crate::ir::{Instr, Ir, Reg};

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
        match *self {
            Selector::Len => ir.instr(Instr::StrLen),
            Selector::Index(i) => {
                let i = ir.instr(Instr::Imm(i));
                ir.instr(Instr::StrGet(i))
            }
            Selector::Table(ref t) => {
                let t = ir.table(t.to_vec());
                let len = ir.instr(Instr::StrLen);
                let index = ir.instr(Instr::Table(t, len));
                ir.instr(Instr::StrGet(index))
            }
            Selector::Sub(k) => {
                let len = ir.instr(Instr::StrLen);
                let k = ir.instr(Instr::Imm(k));
                let index = ir.instr(Instr::Sub(len, k));
                ir.instr(Instr::StrGet(index))
            }
            Selector::And(k) => {
                let len = ir.instr(Instr::StrLen);
                let k = ir.instr(Instr::Imm(k));
                let index = ir.instr(Instr::And(len, k));
                ir.instr(Instr::StrGet(index))
            }
            Selector::Shrl(k) => {
                let len = ir.instr(Instr::StrLen);
                let k = ir.instr(Instr::Imm(k));
                let index = ir.instr(Instr::Shrl(len, k));
                ir.instr(Instr::StrGet(index))
            }
        }
    }
}
