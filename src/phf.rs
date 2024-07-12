use std::collections::HashSet;

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

impl BinOp {
    pub fn commutative(&self) -> bool {
        match self {
            BinOp::Add => true,
            BinOp::Sub => false,
            BinOp::Mul => true,
            BinOp::And => true,
            BinOp::Xor => true,
            BinOp::Shll => false,
            BinOp::Shrl => false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Instr {
    Imm(u32),
    StrGet(Reg),
    StrLen,
    TableGet(Table, Reg),
    TableIndexMask(Table),
    HashMask,
    BinOp(BinOp, Reg, Reg),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Expr {
    Reg(Reg),
    Imm(u32),
    StrGet(Box<Expr>),
    StrLen,
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
            Expr::Reg(_)
            | Expr::Imm(_)
            | Expr::StrLen
            | Expr::TableIndexMask(_)
            | Expr::HashMask => self,
            Expr::StrGet(e) => x.str_get(e.transform(f)),
            Expr::TableGet(t, e) => x.table_get(t, e.transform(f)),
            Expr::BinOp(op, a, b) => x.bin_op(op, a.transform(f), b.transform(f)),
        };
        f(tmp)
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

    pub fn str_get(&self, i: Expr) -> Expr {
        Expr::StrGet(Box::new(i))
    }

    pub fn str_len(&self) -> Expr {
        Expr::StrLen
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
        exprs.into_iter().reduce(|a, b| self.add(a, b)).unwrap()
    }
}

#[derive(Clone)]
pub struct Phf {
    pub keys: Vec<Vec<u32>>,
    pub interpreted_keys: Vec<Vec<u32>>,
    pub min_nonzero_key_len: usize,
    pub max_key_len: usize,
    pub instrs: Vec<Instr>,
    pub data_tables: Vec<Vec<u8>>,
    pub hash_table: Option<Vec<Vec<u32>>>,
}

impl Phf {
    pub fn new(keys: &[Vec<u32>]) -> Phf {
        let mut interpreted_keys: Vec<Vec<u32>> =
            keys.iter().filter(|key| !key.is_empty()).cloned().collect();
        if interpreted_keys.is_empty() {
            interpreted_keys.push(vec!['!' as u32]);
        }

        let min_nonzero_key_len = interpreted_keys
            .iter()
            .map(Vec::len)
            .filter(|&n| n > 0)
            .min()
            .unwrap();
        let max_key_len = interpreted_keys.iter().map(Vec::len).max().unwrap();

        Phf {
            keys: keys.to_vec(),
            interpreted_keys,
            min_nonzero_key_len,
            max_key_len,
            instrs: Vec::new(),
            data_tables: Vec::new(),
            hash_table: None,
        }
    }

    pub fn last_reg(&self) -> Reg {
        Reg(self.instrs.len() - 1)
    }

    fn push_instr(&mut self, instr: Instr) -> Reg {
        self.instrs.push(instr);
        Reg(self.instrs.len() - 1)
    }

    pub fn push_data_table(&mut self, table: Vec<u8>) -> Table {
        self.data_tables.push(table);
        Table(self.data_tables.len() - 1)
    }

    pub fn push_expr(&mut self, expr: Expr) -> Reg {
        match expr {
            Expr::Imm(n) => self.push_instr(Instr::Imm(n)),
            Expr::Reg(r) => r,
            Expr::StrGet(i) => {
                let i = self.push_expr(*i);
                self.push_instr(Instr::StrGet(i))
            }
            Expr::StrLen => self.push_instr(Instr::StrLen),
            Expr::TableGet(t, i) => {
                let i = self.push_expr(*i);
                self.push_instr(Instr::TableGet(t, i))
            }
            Expr::TableIndexMask(t) => self.push_instr(Instr::TableIndexMask(t)),
            Expr::HashMask => self.push_instr(Instr::HashMask),
            Expr::BinOp(op, a, b) => {
                let a = self.push_expr(*a);
                let b = self.push_expr(*b);
                self.push_instr(Instr::BinOp(op, a, b))
            }
        }
    }

    pub fn build_hash_table(&mut self, hash_bits: u32) {
        self.hash_table = Some(vec![Vec::new(); 1 << hash_bits]);

        let mut has_empty_key = false;
        let mut non_empty_keys: Vec<Vec<u32>> = Vec::new();
        for key in &self.keys {
            if key.is_empty() {
                has_empty_key = true;
            } else {
                non_empty_keys.push(key.clone());
            }
        }

        let interpreter = Interpreter::new(self, &non_empty_keys);
        let hash_reg = self.last_reg();

        for (lane, key) in non_empty_keys.into_iter().enumerate() {
            let hash = interpreter.reg_values(hash_reg)[lane];
            self.hash_table.as_mut().unwrap()[usize::try_from(hash).unwrap()] = key;
        }

        if !has_empty_key {
            let mut fake_key = vec!['!' as u32];
            for key in self.hash_table.as_ref().unwrap() {
                if !key.is_empty() {
                    fake_key = key.clone();
                    break;
                }
            }
            self.hash_table.as_mut().unwrap()[0] = fake_key;
        }

        self.validate()
    }

    fn validate(&self) {
        let hash_reg = self.last_reg();

        let mut keys = Vec::new();
        for (i, key) in self.hash_table.as_ref().unwrap().iter().enumerate() {
            let hash = if key.len() < self.min_nonzero_key_len || key.len() > self.max_key_len {
                0
            } else {
                Interpreter::new(self, &[key.clone()]).reg_values(hash_reg)[0]
            };
            if hash == i.try_into().unwrap() {
                keys.push(key.clone());
            }
        }

        assert!(
            keys.into_iter().collect::<HashSet<_>>()
                == self.keys.iter().cloned().collect::<HashSet<_>>()
        );
    }
}

pub struct Interpreter {
    regs: Vec<Vec<u32>>,
}

impl Interpreter {
    pub fn new(phf: &Phf, keys: &[Vec<u32>]) -> Interpreter {
        let mut regs: Vec<Vec<u32>> = Vec::new();
        for instr in &phf.instrs {
            let new_regs = keys
                .iter()
                .enumerate()
                .map(|(key_i, key)| match *instr {
                    Instr::Imm(n) => n,
                    Instr::StrGet(i) => key[usize::try_from(regs[i.0][key_i]).unwrap()],
                    Instr::StrLen => key.len().try_into().unwrap(),
                    Instr::TableGet(t, i) => {
                        phf.data_tables[t.0][usize::try_from(regs[i.0][key_i]).unwrap()].into()
                    }
                    Instr::TableIndexMask(t) => {
                        (phf.data_tables[t.0].len() - 1).try_into().unwrap()
                    }
                    Instr::HashMask => (phf.hash_table.as_ref().unwrap().len() - 1)
                        .try_into()
                        .unwrap(),
                    Instr::BinOp(op, a, b) => {
                        let a = regs[a.0][key_i];
                        let b = regs[b.0][key_i];
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
                })
                .collect();
            regs.push(new_regs);
        }
        Interpreter { regs }
    }

    pub fn width(&self) -> usize {
        match self.regs.first() {
            Some(vec) => vec.len(),
            None => 0,
        }
    }

    pub fn reg_values(&self, reg: Reg) -> &[u32] {
        &self.regs[reg.0]
    }
}
