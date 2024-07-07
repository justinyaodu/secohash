use std::collections::HashSet;

use crate::keys::Keys;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Reg(pub usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Table(pub usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Instr {
    Imm(u32),
    Table(Table, Reg),
    StrGet(Reg),
    StrLen,
    Add(Reg, Reg),
    Sub(Reg, Reg),
    Mul(Reg, Reg),
    And(Reg, Reg),
    Xor(Reg, Reg),
    Shll(Reg, Reg),
    Shrl(Reg, Reg),
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
            interpreter.run(&self, key);
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
                interpreter.run(&self, key);
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
                Instr::StrGet(i) => key[self.reg(i) as usize],
                Instr::StrLen => key.len() as u32,
                Instr::Add(a, b) => self.reg(a).wrapping_add(self.reg(b)),
                Instr::Sub(a, b) => self.reg(a).wrapping_sub(self.reg(b)),
                Instr::Mul(a, b) => self.reg(a).wrapping_mul(self.reg(b)),
                Instr::And(a, b) => self.reg(a) & self.reg(b),
                Instr::Xor(a, b) => self.reg(a) ^ self.reg(b),
                Instr::Shll(a, b) => self.reg(a) << self.reg(b),
                Instr::Shrl(a, b) => self.reg(a) >> self.reg(b),
            });
        }
        self.regs.last().copied().unwrap()
    }
}
