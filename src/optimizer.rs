use std::collections::HashMap;

use crate::phf::{BinOp, Expr, ExprBuilder, Instr, Reg};

fn remove_zero_shifts(expr: Expr) -> Expr {
    expr.transform(&|top| match top {
        Expr::BinOp(BinOp::Shll | BinOp::Shrl, e, shift_amount)
            if matches!(*shift_amount, Expr::Imm(0)) =>
        {
            *e
        }
        _ => top,
    })
}

fn eliminate_common_subexprs(instrs: &[Instr]) -> Vec<Instr> {
    let mut instr_to_new_reg: HashMap<Instr, Reg> = HashMap::new();
    let mut reg_to_new_reg = Vec::new();
    let mut new_instrs = Vec::new();

    for instr in instrs {
        let renamed = match *instr {
            Instr::Imm(_) | Instr::StrLen | Instr::TableIndexMask(_) | Instr::HashMask => *instr,
            Instr::StrGet(i) => Instr::StrGet(reg_to_new_reg[i.0]),
            Instr::TableGet(t, i) => Instr::TableGet(t, reg_to_new_reg[i.0]),
            Instr::BinOp(op, a, b) => Instr::BinOp(op, reg_to_new_reg[a.0], reg_to_new_reg[b.0]),
        };
        let new_reg = match instr_to_new_reg.get(&renamed).copied() {
            Some(r) => r,
            None => {
                let r = renamed.push_into(&mut new_instrs);
                instr_to_new_reg.insert(renamed, r);
                r
            }
        };
        reg_to_new_reg.push(new_reg);
    }

    new_instrs
}

fn unflatten_one(instrs: &[Instr], i: usize, subexpr_regs: &HashMap<usize, usize>) -> Expr {
    if let Some(r) = subexpr_regs.get(&i).cloned() {
        return Expr::Reg(Reg(r));
    }
    let x = ExprBuilder();
    match instrs[i] {
        Instr::Imm(n) => x.imm(n),
        Instr::StrGet(r) => x.str_get(unflatten_one(instrs, r.0, subexpr_regs)),
        Instr::StrLen => x.str_len(),
        Instr::TableGet(t, r) => x.table_get(t, unflatten_one(instrs, r.0, subexpr_regs)),
        Instr::TableIndexMask(t) => x.table_index_mask(t),
        Instr::HashMask => x.hash_mask(),
        Instr::BinOp(op, a, b) => x.bin_op(
            op,
            unflatten_one(instrs, a.0, subexpr_regs),
            unflatten_one(instrs, b.0, subexpr_regs),
        ),
    }
}

pub fn unflatten_many(instrs: &[Instr]) -> Vec<Expr> {
    let mut refcounts = vec![0usize; instrs.len()];
    for instr in instrs {
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

    let mut subexpr_regs = HashMap::new();
    let mut exprs = Vec::new();
    for (i, refcount) in refcounts.iter().copied().enumerate() {
        if refcount > 1 && !matches!(instrs[i], Instr::Imm(_)) {
            let reg = subexpr_regs.len();
            exprs.push(unflatten_one(instrs, i, &subexpr_regs));
            subexpr_regs.insert(i, reg);
        }
    }
    exprs.push(unflatten_one(instrs, instrs.len() - 1, &subexpr_regs));
    exprs
}

pub fn optimize(instrs: &[Instr]) -> Vec<Instr> {
    let top = remove_zero_shifts(unflatten_one(instrs, instrs.len() - 1, &HashMap::new()));
    let mut instrs = Vec::new();
    top.flatten(&mut instrs);
    eliminate_common_subexprs(&instrs)
}

#[cfg(test)]
mod test {
    use crate::phf::Table;

    use super::*;

    #[test]
    fn test_remove_zero_shifts() {
        let x = ExprBuilder();
        assert_eq!(
            remove_zero_shifts(x.sum(vec![
                x.shll(x.imm(0), x.imm(2)),
                x.shll(x.shrl(x.hash_mask(), x.imm(0)), x.imm(0))
            ])),
            x.sum(vec![x.shll(x.imm(0), x.imm(2)), x.hash_mask()])
        )
    }

    #[test]
    fn test_eliminate_common_subexprs() {
        assert_eq!(
            eliminate_common_subexprs(&[
                Instr::StrLen,
                Instr::StrLen,
                Instr::BinOp(BinOp::Add, Reg(0), Reg(1)),
                Instr::BinOp(BinOp::Add, Reg(0), Reg(1)),
                Instr::TableGet(Table(0), Reg(3)),
            ]),
            vec![
                Instr::StrLen,
                Instr::BinOp(BinOp::Add, Reg(0), Reg(0)),
                Instr::TableGet(Table(0), Reg(1))
            ]
        )
    }
}
