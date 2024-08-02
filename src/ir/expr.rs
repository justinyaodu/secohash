use std::collections::HashMap;

use super::{BinOp, Instr, Reg, Table, Tac};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Var(pub usize);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Expr {
    Var(Var),
    Reg(Reg),
    Imm(u32),
    StrGet(Box<Expr>),
    StrLen,
    StrSum(u8),
    TableGet(Table, Box<Expr>),
    TableIndexMask(Table),
    HashMask,
    BinOp(BinOp, Box<Expr>, Box<Expr>),
}

impl Expr {
    pub fn transform<F>(self, f: &F) -> Expr
    where
        F: Fn(Expr) -> Expr,
    {
        let x = ExprBuilder();
        let tmp = match self {
            Expr::Var(_)
            | Expr::Reg(_)
            | Expr::Imm(_)
            | Expr::StrLen
            | Expr::StrSum(_)
            | Expr::TableIndexMask(_)
            | Expr::HashMask => self,
            Expr::StrGet(e) => x.str_get(e.transform(f)),
            Expr::TableGet(t, e) => x.table_get(t, e.transform(f)),
            Expr::BinOp(op, a, b) => x.bin_op(op, a.transform(f), b.transform(f)),
        };
        f(tmp)
    }

    pub fn flatten(&self, tac: &mut Tac, var_to_reg: &HashMap<Var, Reg>) -> Reg {
        match *self {
            Expr::Imm(n) => tac.push(Instr::Imm(n)),
            Expr::Reg(r) => r,
            Expr::Var(s) => var_to_reg.get(&s).copied().unwrap(),
            Expr::StrGet(ref i) => {
                let i = i.flatten(tac, var_to_reg);
                tac.push(Instr::StrGet(i))
            }
            Expr::StrLen => tac.push(Instr::StrLen),
            Expr::StrSum(m) => tac.push(Instr::StrSum(m)),
            Expr::TableGet(t, ref i) => {
                let i = i.flatten(tac, var_to_reg);
                tac.push(Instr::TableGet(t, i))
            }
            Expr::TableIndexMask(t) => tac.push(Instr::TableIndexMask(t)),
            Expr::HashMask => tac.push(Instr::HashMask),
            Expr::BinOp(op, ref a, ref b) => {
                let a = a.flatten(tac, var_to_reg);
                let b = b.flatten(tac, var_to_reg);
                tac.push(Instr::BinOp(op, a, b))
            }
        }
    }
}

pub struct ExprBuilder();

impl ExprBuilder {
    pub fn reg(&self, r: Reg) -> Expr {
        Expr::Reg(r)
    }

    pub fn imm(&self, n: u32) -> Expr {
        Expr::Imm(n)
    }

    pub fn str_get(&self, i: Expr) -> Expr {
        Expr::StrGet(Box::new(i))
    }

    pub fn str_len(&self) -> Expr {
        Expr::StrLen
    }

    pub fn str_sum(&self, m: u8) -> Expr {
        Expr::StrSum(m)
    }

    pub fn table_get(&self, t: Table, i: Expr) -> Expr {
        Expr::TableGet(t, Box::new(i))
    }

    pub fn table_index_mask(&self, t: Table) -> Expr {
        Expr::TableIndexMask(t)
    }

    pub fn hash_mask(&self) -> Expr {
        Expr::HashMask
    }

    pub fn add(&self, a: Expr, b: Expr) -> Expr {
        self.bin_op(BinOp::Add, a, b)
    }

    pub fn sub(&self, a: Expr, b: Expr) -> Expr {
        self.bin_op(BinOp::Sub, a, b)
    }

    pub fn and(&self, a: Expr, b: Expr) -> Expr {
        self.bin_op(BinOp::And, a, b)
    }

    pub fn shll(&self, a: Expr, b: Expr) -> Expr {
        self.bin_op(BinOp::Shll, a, b)
    }

    pub fn shrl(&self, a: Expr, b: Expr) -> Expr {
        self.bin_op(BinOp::Shrl, a, b)
    }

    pub fn bin_op(&self, op: BinOp, a: Expr, b: Expr) -> Expr {
        Expr::BinOp(op, Box::new(a), Box::new(b))
    }

    pub fn sum(&self, exprs: Vec<Expr>) -> Expr {
        exprs
            .into_iter()
            .reduce(|a, b| self.add(a, b))
            .unwrap_or(self.imm(0))
    }
}

pub struct Exprs(pub Vec<Expr>);

impl Exprs {
    pub fn new() -> Exprs {
        Exprs(Vec::new())
    }

    pub fn push(&mut self, expr: Expr) -> Var {
        self.0.push(expr);
        Var(self.0.len() - 1)
    }
}
