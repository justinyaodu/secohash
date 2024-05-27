#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Reg(pub usize);

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Table(pub usize);

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum Instr {
    Imm(u32),
    Table(Table, Reg),
    StrGet(Reg),
    StrLen,
    Add(Reg, Reg),
    And(Reg, Reg),
    Sub(Reg, Reg),
    Shll(Reg, Reg),
    Shrl(Reg, Reg),
}

#[derive(Clone, PartialEq, Eq, Hash)]
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

    // TODO: common subexpression elimination
    // TODO: dead code elimination
}

pub struct Interpreter<'a> {
    ir: &'a Ir,
    regs: Vec<u32>,
}

impl Interpreter<'_> {
    pub fn new(ir: &Ir) -> Interpreter<'_> {
        Interpreter {
            ir,
            regs: Vec::new(),
        }
    }

    pub fn reg(&self, Reg(reg): Reg) -> u32 {
        self.regs[reg]
    }

    fn table(&self, Table(table): Table, reg: Reg) -> u32 {
        let index = self.reg(reg) as usize;
        self.ir.tables[table][index].into()
    }

    pub fn run(&mut self, key: &[u32]) -> u32 {
        for instr in &self.ir.instrs {
            self.regs.push(match *instr {
                Instr::Imm(n) => n,
                Instr::Table(t, i) => self.table(t, i),
                Instr::StrGet(i) => key[self.reg(i) as usize],
                Instr::StrLen => key.len() as u32,
                Instr::Add(a, b) => self.reg(a).wrapping_add(self.reg(b)),
                Instr::Sub(a, b) => self.reg(a).wrapping_sub(self.reg(b)),
                Instr::And(a, b) => self.reg(a) & self.reg(b),
                Instr::Shll(a, b) => self.reg(a) << self.reg(b),
                Instr::Shrl(a, b) => self.reg(a) >> self.reg(b),
            });
        }
        self.regs.last().copied().unwrap()
    }
}
