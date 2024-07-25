use std::{collections::HashMap, ops::Index};

use super::{BinOp, Expr, ExprBuilder, Exprs, Table, Var};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Reg(pub usize);

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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tac(Vec<Instr>);

impl Index<Reg> for Tac {
    type Output = Instr;

    fn index(&self, index: Reg) -> &Self::Output {
        &self.0[index.0]
    }
}

impl Tac {
    pub fn new() -> Tac {
        Tac(Vec::new())
    }

    pub fn instrs(&self) -> &[Instr] {
        &self.0
    }

    pub fn last_reg(&self) -> Reg {
        Reg(self.0.len() - 1)
    }

    pub fn push(&mut self, instr: Instr) -> Reg {
        self.0.push(instr);
        Reg(self.0.len() - 1)
    }

    pub fn push_expr(&mut self, expr: Expr) -> Reg {
        expr.flatten(self, &HashMap::new())
    }

    pub fn local_value_numbering(&self) -> (Tac, HashMap<Reg, Reg>) {
        let mut instr_to_new_reg = HashMap::new();
        let mut reg_to_new_reg = HashMap::new();
        let mut new_tac = Tac::new();

        for instr in &self.0 {
            let renamed = match *instr {
                Instr::Imm(_) | Instr::StrLen | Instr::TableIndexMask(_) | Instr::HashMask => {
                    *instr
                }
                Instr::StrGet(i) => Instr::StrGet(reg_to_new_reg[&i]),
                Instr::TableGet(t, i) => Instr::TableGet(t, reg_to_new_reg[&i]),
                Instr::BinOp(op, a, b) => Instr::BinOp(op, reg_to_new_reg[&a], reg_to_new_reg[&b]),
            };
            let new_reg = match instr_to_new_reg.get(&renamed).copied() {
                Some(r) => r,
                None => {
                    let r = new_tac.push(renamed);
                    instr_to_new_reg.insert(renamed, r);
                    r
                }
            };
            let old_reg = Reg(reg_to_new_reg.len());
            reg_to_new_reg.insert(old_reg, new_reg);
        }
        (new_tac, reg_to_new_reg)
    }

    pub fn unflatten_dag(&self) -> Exprs {
        let mut refcounts = vec![0usize; self.0.len()];
        for instr in &self.0 {
            match *instr {
                Instr::Imm(_) | Instr::StrLen | Instr::TableIndexMask(_) | Instr::HashMask => (),
                Instr::StrGet(i) => {
                    refcounts[i.0] += 1;
                }
                Instr::TableGet(_, i) => {
                    refcounts[i.0] += 1;
                }
                Instr::BinOp(_, a, b) => {
                    refcounts[a.0] += 1;
                    refcounts[b.0] += 1;
                }
            }
        }

        let mut reg_to_var = HashMap::new();
        let mut exprs = Exprs::new();
        for (i, refcount) in refcounts.iter().copied().enumerate() {
            let reg = Reg(i);
            if refcount > 1 && !matches!(self[reg], Instr::Imm(_)) {
                let var = Var(reg_to_var.len());
                exprs.push(self.unflatten_tree(Reg(i), &reg_to_var));
                reg_to_var.insert(reg, var);
            }
        }
        exprs.push(self.unflatten_tree(Reg(self.0.len() - 1), &reg_to_var));
        exprs
    }

    pub fn unflatten_tree(&self, reg: Reg, reg_to_var: &HashMap<Reg, Var>) -> Expr {
        if let Some(r) = reg_to_var.get(&reg).cloned() {
            return Expr::Var(r);
        }
        let x = ExprBuilder();
        match self[reg] {
            Instr::Imm(n) => x.imm(n),
            Instr::StrGet(r) => x.str_get(self.unflatten_tree(r, reg_to_var)),
            Instr::StrLen => x.str_len(),
            Instr::TableGet(t, r) => x.table_get(t, self.unflatten_tree(r, reg_to_var)),
            Instr::TableIndexMask(t) => x.table_index_mask(t),
            Instr::HashMask => x.hash_mask(),
            Instr::BinOp(op, a, b) => x.bin_op(
                op,
                self.unflatten_tree(a, reg_to_var),
                self.unflatten_tree(b, reg_to_var),
            ),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_eliminate_common_subexprs() {
        assert_eq!(
            Tac(vec![
                Instr::StrLen,
                Instr::StrLen,
                Instr::BinOp(BinOp::Add, Reg(0), Reg(1)),
                Instr::BinOp(BinOp::Add, Reg(0), Reg(1)),
                Instr::BinOp(BinOp::Shll, Reg(1), Reg(3)),
            ])
            .local_value_numbering(),
            (
                Tac(vec![
                    Instr::StrLen,
                    Instr::BinOp(BinOp::Add, Reg(0), Reg(0)),
                    Instr::BinOp(BinOp::Shll, Reg(0), Reg(1))
                ]),
                HashMap::from([
                    (Reg(0), Reg(0)),
                    (Reg(1), Reg(0)),
                    (Reg(2), Reg(1)),
                    (Reg(3), Reg(1)),
                    (Reg(4), Reg(2))
                ])
            )
        )
    }
}
