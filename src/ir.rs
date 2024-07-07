use std::collections::HashSet;

use crate::keys::Keys;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Reg(pub usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Table(pub usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    And,
    Xor,
    Shll,
    Shrl,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Instr {
    Imm(u32),
    Table(Table, Reg),
    TableIndexMask(Table),
    StrGet(Reg),
    StrLen,
    BinOp(BinOp, Reg, Reg),
}

pub enum Expr {
    Reg(Reg),
    Imm(u32),
    Table(Table, Box<Expr>),
    TableIndexMask(Table),
    StrGet(Box<Expr>),
    StrLen,
    BinOp(BinOp, Box<Expr>, Box<Expr>),
}

impl Expr {
    fn flatten(self, ir: &mut Ir) -> Reg {
        match self {
            Expr::Imm(n) => ir.instr(Instr::Imm(n)),
            Expr::Reg(r) => r,
            Expr::Table(t, i) => {
                let i = i.flatten(ir);
                ir.instr(Instr::Table(t, i))
            }
            Expr::TableIndexMask(t) => ir.instr(Instr::TableIndexMask(t)),
            Expr::StrGet(i) => {
                let i = i.flatten(ir);
                ir.instr(Instr::StrGet(i))
            }
            Expr::StrLen => ir.instr(Instr::StrLen),
            Expr::BinOp(op, a, b) => {
                let a = a.flatten(ir);
                let b = b.flatten(ir);
                ir.instr(Instr::BinOp(op, a, b))
            }
        }
    }
}

pub struct ExprBuilder();

impl ExprBuilder {
    pub fn imm(&self, n: u32) -> Expr {
        Expr::Imm(n)
    }

    pub fn reg(&self, r: Reg) -> Expr {
        Expr::Reg(r)
    }

    pub fn table(&self, t: Table, i: Expr) -> Expr {
        Expr::Table(t, Box::new(i))
    }

    pub fn table_index_mask(&self, t: Table) -> Expr {
        Expr::TableIndexMask(t)
    }

    pub fn str_get(&self, i: Expr) -> Expr {
        Expr::StrGet(Box::new(i))
    }

    pub fn str_len(&self, ) -> Expr {
        Expr::StrLen
    }

    pub fn add(&self, a: Expr, b: Expr) -> Expr {
        Expr::BinOp(BinOp::Add, Box::new(a), Box::new(b))
    }

    pub fn sub(&self, a: Expr, b: Expr) -> Expr {
        Expr::BinOp(BinOp::Sub, Box::new(a), Box::new(b))
    }

    pub fn and(&self, a: Expr, b: Expr) -> Expr {
        Expr::BinOp(BinOp::And, Box::new(a), Box::new(b))
    }

    pub fn shll(&self, a: Expr, b: Expr) -> Expr {
        Expr::BinOp(BinOp::Shll, Box::new(a), Box::new(b))
    }

    pub fn shrl(&self, a: Expr, b: Expr) -> Expr {
        Expr::BinOp(BinOp::Shrl, Box::new(a), Box::new(b))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Ir {
    pub instrs: Vec<Instr>,
    pub tables: Vec<Vec<u8>>,
}

impl Ir {
    pub fn new() -> Self {
        Ir {
            instrs: Vec::new(),
            tables: Vec::new(),
        }
    }

    pub fn instr(&mut self, instr: Instr) -> Reg {
        self.instrs.push(instr);
        Reg(self.instrs.len() - 1)
    }

    pub fn expr(&mut self, expr: Expr) -> Reg {
        expr.flatten(self)
    }

    pub fn table(&mut self, table: Vec<u8>) -> Table {
        self.tables.push(table);
        Table(self.tables.len() - 1)
    }

    pub fn last_reg(&self) -> Reg {
        Reg(self.instrs.len() - 1)
    }

    pub fn distinguishes(&self, keys: &Keys, regs: &[Reg]) -> bool {
        let mut hashes = HashSet::new();
        for key in &keys.non_empty_keys {
            let mut interpreter = Interpreter::new();
            interpreter.run(self, key);
            if !hashes.insert(regs.iter().map(|&r| interpreter.reg(r)).collect::<Vec<_>>()) {
                return false;
            }
        }
        true
    }

    pub fn run_all(&self, keys: &Keys) -> Vec<Interpreter> {
        keys.non_empty_keys
            .iter()
            .map(|key| {
                let mut interpreter = Interpreter::new();
                interpreter.run(self, key);
                interpreter
            })
            .collect()
    }

    // TODO: common subexpression elimination
    // TODO: dead code elimination
}

#[derive(Clone, Debug)]
pub struct Interpreter {
    regs: Vec<u32>,
}

impl Interpreter {
    pub fn new() -> Interpreter {
        Interpreter { regs: Vec::new() }
    }

    pub fn reg(&self, Reg(reg): Reg) -> u32 {
        self.regs[reg]
    }

    fn table(&self, ir: &Ir, Table(table): Table, reg: Reg) -> u32 {
        let index = self.reg(reg) as usize;
        let table = &ir.tables[table];
        table[index].into()
    }

    pub fn run(&mut self, ir: &Ir, key: &[u32]) -> u32 {
        for instr in &ir.instrs {
            self.regs.push(match *instr {
                Instr::Imm(n) => n,
                Instr::Table(t, i) => self.table(ir, t, i),
                Instr::TableIndexMask(t) => (ir.tables[t.0].len() - 1) as u32,
                Instr::StrGet(i) => key[self.reg(i) as usize],
                Instr::StrLen => key.len() as u32,
                Instr::BinOp(op, a, b) => {
                    let a = self.reg(a);
                    let b = self.reg(b);
                    match op {
                        BinOp::Add => a.wrapping_add(b),
                        BinOp::Sub => a.wrapping_sub(b),
                        BinOp::Mul => a.wrapping_mul(b),
                        BinOp::And => a & b,
                        BinOp::Xor => a ^ b,
                        BinOp::Shll => a << b,
                        BinOp::Shrl => a >> b,
                    }
                }
            });
        }
        self.regs.last().copied().unwrap()
    }
}
