use super::{BinOp, Expr};

pub fn constant_propagation(expr: Expr) -> Expr {
    expr.transform(&|top| match top {
        Expr::BinOp(BinOp::Add | BinOp::Sub | BinOp::Shll | BinOp::Shrl, a, b)
            if *b == Expr::Imm(0) =>
        {
            *a
        }
        _ => top,
    })
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::ir::ExprBuilder;

    #[test]
    fn test_remove_zero_shifts() {
        let x = ExprBuilder();
        assert_eq!(
            constant_propagation(x.sum(vec![
                x.shll(x.imm(0), x.imm(2)),
                x.shll(x.shrl(x.hash_mask(), x.imm(0)), x.imm(0)),
                x.imm(0),
            ])),
            x.sum(vec![x.shll(x.imm(0), x.imm(2)), x.hash_mask()])
        )
    }
}
