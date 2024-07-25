use super::{BinOp, Expr};

pub fn remove_zero_shifts(expr: Expr) -> Expr {
    expr.transform(&|top| match top {
        Expr::BinOp(BinOp::Shll | BinOp::Shrl, e, shift_amount)
            if *shift_amount == Expr::Imm(0) =>
        {
            *e
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
            remove_zero_shifts(x.sum(vec![
                x.shll(x.imm(0), x.imm(2)),
                x.shll(x.shrl(x.hash_mask(), x.imm(0)), x.imm(0))
            ])),
            x.sum(vec![x.shll(x.imm(0), x.imm(2)), x.hash_mask()])
        )
    }
}
