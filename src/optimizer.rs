use std::collections::{HashMap, HashSet};

use crate::phf::{BinOp, Expr, ExprBuilder, Instr, Reg};

fn instr_to_expr(instrs: &[Instr], i: usize) -> Expr {
    let e = ExprBuilder();
    match instrs[i] {
        Instr::Imm(n) => e.imm(n),
        Instr::StrGet(r) => e.str_get(instr_to_expr(instrs, r.0)),
        Instr::StrLen => e.str_len(),
        Instr::TableGet(t, r) => e.table_get(t, instr_to_expr(instrs, r.0)),
        Instr::TableIndexMask(t) => e.table_index_mask(t),
        Instr::HashMask => e.hash_mask(),
        Instr::BinOp(op, a, b) => e.reduce(
            op,
            vec![instr_to_expr(instrs, a.0), instr_to_expr(instrs, b.0)],
        ),
    }
}

fn remove_zero_shifts(expr: Expr) -> Expr {
    expr.transform(&|top| match top {
        Expr::Reduce(BinOp::Shll | BinOp::Shrl, ref operands) => match &operands[..] {
            [e, Expr::Imm(0)] => e.clone(),
            _ => top,
        },
        _ => top,
    })
}

fn flatten_nested_reductions(expr: Expr) -> Expr {
    expr.transform(&|top| match top {
        Expr::Reduce(op, children) => Expr::Reduce(
            op,
            children
                .into_iter()
                .flat_map(|child| match child {
                    Expr::Reduce(child_op, grandchildren) if child_op == op => {
                        grandchildren.into_iter()
                    }
                    child => vec![child].into_iter(),
                })
                .collect(),
        ),
        _ => top,
    })
}

fn cleanup(expr: Expr) -> Expr {
    remove_zero_shifts(flatten_nested_reductions(expr))
}

fn find_common_subexprs(expr: &Expr, seen: &mut HashSet<Expr>, common: &mut HashMap<Expr, usize>) {
    match expr {
        Expr::Reg(_) => panic!(),
        Expr::Imm(_) => return,
        Expr::StrGet(i) => {
            find_common_subexprs(i, seen, common);
        }
        Expr::StrLen => (),
        Expr::TableGet(_, i) => {
            find_common_subexprs(i, seen, common);
        }
        Expr::TableIndexMask(_) => (),
        Expr::HashMask => (),
        Expr::Reduce(_, operands) => {
            for operand in operands {
                find_common_subexprs(operand, seen, common);
            }
        }
    }
    if seen.contains(expr) {
        if !common.contains_key(expr) {
            let id = common.len();
            common.insert(expr.clone(), id);
        }
    } else {
        seen.insert(expr.clone());
    }
}

fn dedup_common_subexprs(expr: &Expr, common: &HashMap<Expr, usize>, top: bool) -> Expr {
    let e = ExprBuilder();
    if !top {
        if let Some(&id) = common.get(expr) {
            return e.reg(Reg(id));
        }
    }

    match *expr {
        Expr::Reg(_) => panic!(),
        Expr::Imm(_) | Expr::StrLen | Expr::TableIndexMask(_) | Expr::HashMask => expr.clone(),
        Expr::StrGet(ref i) => e.str_get(dedup_common_subexprs(i, common, false)),
        Expr::TableGet(t, ref i) => e.table_get(t, dedup_common_subexprs(i, common, false)),
        Expr::Reduce(op, ref operands) => e.reduce(
            op,
            operands
                .iter()
                .map(|operand| dedup_common_subexprs(operand, common, false))
                .collect(),
        ),
    }
}

pub fn optimize(instrs: &[Instr]) -> Vec<Expr> {
    let top = cleanup(instr_to_expr(instrs, instrs.len() - 1));
    let mut seen = HashSet::new();
    let mut common = HashMap::new();
    find_common_subexprs(&top, &mut seen, &mut common);
    eprintln!("common={common:?}");
    eprintln!("seen={seen:?}");

    let mut exprs = vec![Expr::HashMask; common.len() + 1];
    for (expr, &i) in &common {
        exprs[i] = dedup_common_subexprs(expr, &common, true)
    }
    exprs[common.len()] = dedup_common_subexprs(&top, &common, true);
    exprs
}

#[cfg(test)]
mod test {
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
    fn test_flatten_nested_reductions() {
        let b = ExprBuilder();
        assert_eq!(
            flatten_nested_reductions(b.sum(vec![
                b.sum(vec![b.imm(0), b.sum(vec![b.imm(1), b.imm(2)])]),
                b.imm(3),
                b.sub(b.imm(4), b.imm(5))
            ])),
            b.sum(vec![
                b.imm(0),
                b.imm(1),
                b.imm(2),
                b.imm(3),
                b.sub(b.imm(4), b.imm(5))
            ])
        )
    }
}
