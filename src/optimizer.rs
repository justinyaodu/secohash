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

fn cleanup(expr: Expr) -> Expr {
    remove_zero_shifts(expr)
}

fn lvn(instrs: &[Instr]) -> Vec<Instr> {
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
                let r = Reg(instr_to_new_reg.len());
                instr_to_new_reg.insert(renamed, r);
                new_instrs.push(renamed);
                r
            }
        };
        reg_to_new_reg.push(new_reg);
    }

    new_instrs
}

fn instr_to_expr(instrs: &[Instr], i: usize, top: bool, reg_map: &HashMap<usize, usize>) -> Expr {
    if !top {
        if let Some(r) = reg_map.get(&i).cloned() {
            return Expr::Reg(Reg(r));
        }
    }
    let e = ExprBuilder();
    match instrs[i] {
        Instr::Imm(n) => e.imm(n),
        Instr::StrGet(r) => e.str_get(instr_to_expr(instrs, r.0, false, reg_map)),
        Instr::StrLen => e.str_len(),
        Instr::TableGet(t, r) => e.table_get(t, instr_to_expr(instrs, r.0, false, reg_map)),
        Instr::TableIndexMask(t) => e.table_index_mask(t),
        Instr::HashMask => e.hash_mask(),
        Instr::BinOp(op, a, b) => e.bin_op(
            op,
            instr_to_expr(instrs, a.0, false, reg_map),
            instr_to_expr(instrs, b.0, false, reg_map),
        ),
    }
}

fn thing(instrs: &[Instr]) -> Vec<Expr> {
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

    let mut reg_map = HashMap::new();
    for (i, refcount) in refcounts.iter().copied().enumerate() {
        if refcount > 1 && !matches!(instrs[i], Instr::Imm(_)) {
            let reg = reg_map.len();
            reg_map.insert(i, reg);
        }
    }

    let mut exprs = Vec::new();
    for i in 0..instrs.len() {
        if reg_map.contains_key(&i) || i == instrs.len() - 1 {
            exprs.push(instr_to_expr(instrs, i, true, &reg_map));
        }
    }
    exprs
}

pub fn optimize(instrs: &[Instr]) -> Vec<Expr> {
    let top = cleanup(instr_to_expr(
        instrs,
        instrs.len() - 1,
        true,
        &HashMap::new(),
    ));
    let mut instrs = Vec::new();
    top.flatten(&mut instrs);
    let instrs = lvn(&instrs);
    thing(&instrs)
}

#[cfg(test)]
mod test {
    use crate::phf::Table;

    use super::*;

    #[test]
    fn test_remove_zero_shifts() {
        let b = ExprBuilder();
        assert_eq!(
            remove_zero_shifts(b.sum(vec![
                b.shll(b.imm(0), b.imm(2)),
                b.shll(b.shrl(b.hash_mask(), b.imm(0)), b.imm(0))
            ])),
            b.sum(vec![b.shll(b.imm(0), b.imm(2)), b.hash_mask()])
        )
    }

    #[test]
    fn test_lvn() {
        assert_eq!(
            lvn(&[
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
