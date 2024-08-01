use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CBinOp {
    Add,
    Sub,
    And,
    Shl,
    Shr,
}

impl CBinOp {
    fn commutative(&self) -> bool {
        use CBinOp::*;

        match self {
            Add | And => true,
            Sub | Shl | Shr => false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CExpr {
    Var(String),
    Imm(u32),
    Call(String, Vec<CExpr>),
    Index(String, Box<CExpr>),
    Cast(String, Box<CExpr>),
    BinOp(CBinOp, Box<CExpr>, Box<CExpr>),
}

impl CExpr {
    fn transform<F>(self, f: &mut F) -> CExpr
    where
        F: FnMut(CExpr) -> CExpr,
    {
        use CExpr::*;

        let tmp = match self {
            Var(_) | Imm(_) => self,
            Call(name, args) => {
                let args = args.into_iter().map(|arg| arg.transform(f)).collect();
                Call(name, args)
            }
            Index(name, e) => {
                let e = e.transform(f);
                Index(name, Box::new(e))
            }
            Cast(t, e) => {
                let e = e.transform(f);
                Cast(t, Box::new(e))
            }
            BinOp(op, a, b) => {
                let a = a.transform(f);
                let b = b.transform(f);
                BinOp(op, Box::new(a), Box::new(b))
            }
        };
        f(tmp)
    }

    fn clean_step(self) -> CExpr {
        use CBinOp::*;
        use CExpr::*;

        match self {
            BinOp(And, _, b) if *b == Imm(0) => *b,
            BinOp(Add | Sub | Shl | Shr, a, b) if *b == Imm(0) => *a,
            _ => self,
        }
    }

    pub fn cleaned(self) -> CExpr {
        self.transform(&mut Self::clean_step)
    }

    pub fn precedence(&self) -> u8 {
        use CBinOp::*;
        use CExpr::*;
        match *self {
            Var(_) | Imm(_) => 0,
            Call(_, _) | Index(_, _) => 1,
            BinOp(Add | Sub | And | Shl | Shr, _, _) => 4,
            Cast(_, _) => 100,
        }
    }

    fn write_with_parens(&self, f: &mut fmt::Formatter, parens: bool) -> fmt::Result {
        if parens {
            write!(f, "({self})")
        } else {
            write!(f, "{self}")
        }
    }

    fn needs_parens_in_bin_op(&self, bin_op: &CExpr) -> bool {
        use CExpr::*;
        let BinOp(outer_op, _, _) = bin_op else {
            panic!();
        };
        let mut parens = self.precedence() >= bin_op.precedence();
        if let BinOp(op, _, _) = self {
            if op == outer_op && op.commutative() {
                parens = false;
            }
        }
        parens
    }
}

impl fmt::Display for CExpr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use CExpr::*;
        match self {
            Var(s) => write!(f, "{s}"),
            Imm(n) => write!(f, "{n}"),
            Call(name, args) => {
                write!(f, "{name}(")?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{arg}")?;
                }
                write!(f, ")")
            }
            Index(name, e) => {
                write!(f, "{name}[{e}]")
            }
            Cast(t, e) => {
                write!(f, "({t}) ")?;
                e.write_with_parens(f, e.precedence() > 1)
            }
            BinOp(op, left, right) => {
                use CBinOp::*;

                left.write_with_parens(f, left.needs_parens_in_bin_op(self))?;

                write!(
                    f,
                    " {} ",
                    match op {
                        Add => "+",
                        Sub => "-",
                        And => "&",
                        Shl => "<<",
                        Shr => ">>",
                    }
                )?;

                right.write_with_parens(f, right.needs_parens_in_bin_op(self))
            }
        }
    }
}

pub struct CExprBuilder();

impl CExprBuilder {
    pub fn var(&self, name: String) -> CExpr {
        CExpr::Var(name)
    }

    pub fn imm(&self, n: u32) -> CExpr {
        CExpr::Imm(n)
    }

    pub fn call(&self, name: String, args: Vec<CExpr>) -> CExpr {
        CExpr::Call(name, args)
    }

    pub fn index(&self, name: String, e: CExpr) -> CExpr {
        CExpr::Index(name, Box::new(e))
    }

    pub fn cast(&self, t: String, e: CExpr) -> CExpr {
        CExpr::Cast(t, Box::new(e))
    }

    pub fn bin_op(&self, op: CBinOp, a: CExpr, b: CExpr) -> CExpr {
        CExpr::BinOp(op, Box::new(a), Box::new(b))
    }

    pub fn add(&self, a: CExpr, b: CExpr) -> CExpr {
        self.bin_op(CBinOp::Add, a, b)
    }

    pub fn sub(&self, a: CExpr, b: CExpr) -> CExpr {
        self.bin_op(CBinOp::Sub, a, b)
    }

    pub fn and(&self, a: CExpr, b: CExpr) -> CExpr {
        self.bin_op(CBinOp::And, a, b)
    }

    pub fn shl(&self, a: CExpr, b: CExpr) -> CExpr {
        self.bin_op(CBinOp::Shl, a, b)
    }

    pub fn shr(&self, a: CExpr, b: CExpr) -> CExpr {
        self.bin_op(CBinOp::Shr, a, b)
    }

    pub fn sum(&self, exprs: Vec<CExpr>) -> CExpr {
        exprs.into_iter().reduce(|a, b| self.add(a, b)).unwrap()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_cleaned() {
        let x = CExprBuilder();
        let e = x.call(
            "foo".into(),
            vec![
                x.add(x.imm(10), x.imm(0)),
                x.sub(x.imm(20), x.imm(0)),
                x.shl(x.imm(30), x.imm(0)),
                x.shr(x.imm(40), x.imm(0)),
                x.shl(x.imm(50), x.and(x.var("i".into()), x.imm(0))),
            ],
        );
        let cleaned = x.call(
            "foo".into(),
            vec![x.imm(10), x.imm(20), x.imm(30), x.imm(40), x.imm(50)],
        );
        assert_eq!(e.clone().cleaned(), cleaned);
    }

    #[test]
    fn test_fmt_add_and_shift() {
        let x = CExprBuilder();
        let e = x.sum(vec![
            x.sum(vec![x.imm(10), x.imm(20), x.imm(30)]),
            x.shl(x.sum(vec![x.imm(40), x.imm(50), x.imm(60)]), x.imm(1)),
        ]);
        assert_eq!(format!("{e}"), "10 + 20 + 30 + ((40 + 50 + 60) << 1)");
    }

    #[test]
    fn test_fmt_cast() {
        let x = CExprBuilder();
        let e = x.index(
            "key".into(),
            x.sub(x.cast("uint32_t".into(), x.var("len".into())), x.imm(1)),
        );
        assert_eq!(format!("{e}"), "key[((uint32_t) len) - 1]");
    }
}
