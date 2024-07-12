use crate::{
    optimizer::optimize,
    phf::{BinOp, Expr, Instr, Phf, Reg, Table},
};

pub trait Backend {
    fn emit(&self, phf: &Phf) -> String;
}

pub struct CBackend {
    char_escape_table: Vec<String>,
}

impl CBackend {
    pub fn new() -> CBackend {
        CBackend {
            char_escape_table: Self::build_char_escape_table(),
        }
    }

    fn build_char_escape_table() -> Vec<String> {
        let mut char_escape_table = Vec::new();

        for i in 0..256 {
            char_escape_table.push(format!("\\{i:03o}"));
        }

        for i in 20..=126 {
            char_escape_table[i as usize] = String::from_utf8(vec![i]).unwrap();
        }

        for (char, escaped) in [
            ('?', "\\?"),
            ('"', "\""),
            ('\\', "\\"),
            ('\n', "\\n"),
            ('\r', "\\r"),
            ('\t', "\\t"),
        ] {
            char_escape_table[char as usize] = escaped.into();
        }

        char_escape_table
    }

    fn string_literal(&self, key: &[u32]) -> String {
        let mut s = String::new();
        s.push('"');
        for c in key.iter().copied() {
            assert!(c < 256);
            s.push_str(&self.char_escape_table[c as usize])
        }
        s.push('"');
        s
    }

    fn compile_expr(phf: &Phf, expr: &Expr) -> String {
        Self::compile_expr_rec(phf, expr).0
    }

    fn precedence(op: Option<BinOp>) -> usize {
        match op {
            None => 0,
            Some(BinOp::Mul) => 1,
            Some(BinOp::Add | BinOp::Sub | BinOp::And | BinOp::Xor) => 2,
            Some(BinOp::Shll | BinOp::Shrl) => 3,
        }
    }

    fn compile_expr_rec(phf: &Phf, expr: &Expr) -> (String, Option<BinOp>) {
        match *expr {
            Expr::Reg(Reg(i)) => (format!("x{i}"), None),
            Expr::Imm(n) => (n.to_string(), None),
            Expr::StrGet(ref i) => (format!("key[{}]", Self::compile_expr_rec(phf, i).0), None),
            Expr::StrLen => ("(uint32_t) len".into(), None),
            Expr::TableGet(Table(t), ref i) => {
                (format!("t{t}[{}]", Self::compile_expr_rec(phf, i).0), None)
            }
            Expr::TableIndexMask(t) => (format!("{:#x}", phf.data_tables[t.0].len() - 1), None),
            Expr::HashMask => (
                format!("{:#x}", phf.hash_table.as_ref().unwrap().len() - 1),
                None,
            ),
            Expr::BinOp(op, ref a, ref b) => {
                let op_str = match op {
                    BinOp::Add => "+",
                    BinOp::Sub => "-",
                    BinOp::Mul => "*",
                    BinOp::And => "&",
                    BinOp::Xor => "^",
                    BinOp::Shll => "<<",
                    BinOp::Shrl => ">>",
                };
                let op_prec = Self::precedence(Some(op));
                (
                    [a, b]
                        .into_iter()
                        .map(|child| {
                            let (child_str, child_op) = Self::compile_expr_rec(phf, child);
                            let child_prec = Self::precedence(child_op);
                            if child_prec < op_prec || (child_op == Some(op) && op.commutative()) {
                                child_str
                            } else {
                                format!("({child_str})")
                            }
                        })
                        .collect::<Vec<_>>()
                        .join(&format!(" {op_str} ")),
                    Some(op),
                )
            }
        }
    }
}

impl Backend for CBackend {
    fn emit(&self, phf: &Phf) -> String {
        let mut lines: Vec<String> = Vec::new();

        lines.push(
            "\
#include <stddef.h>
#include <stdint.h>
#include <string.h>

struct entry {
    char* key;
    uint32_t len;
    uint32_t value;
};

const struct entry entries[] = {"
                .into(),
        );

        for (i, key) in phf.hash_table.as_ref().unwrap().iter().enumerate() {
            let string_literal = self.string_literal(key);

            let len = key.len();
            let is_fake_key = (i == 0) ^ (key.is_empty());
            let ordinal: String = if is_fake_key {
                "-1".into()
            } else {
                phf.keys.iter().position(|k| k == key).unwrap().to_string()
            };

            let entry = format!("{{ {string_literal}, {len}, {ordinal} }}");
            lines.push(format!("    {entry},"));
        }

        lines.push(
            "\
};

uint32_t hash(const char *key, size_t len) {"
                .into(),
        );

        {
            let min = phf.min_nonzero_key_len;
            let max = phf.max_key_len;
            lines.push(format!("    if (len < {min} || len > {max}) {{"));
            lines.push(
                "        return 0;
    }"
                .into(),
            );
        }

        for (i, table) in phf.data_tables.iter().enumerate() {
            let nums = table
                .iter()
                .map(|n| n.to_string())
                .collect::<Vec<_>>()
                .join(", ");
            lines.push(format!("    static const uint8_t t{i}[] = {{ {nums} }};"));
        }

        let exprs = optimize(&phf.instrs);
        for (i, expr) in exprs.iter().enumerate() {
            let expr_str = Self::compile_expr(phf, expr);
            lines.push(format!(
                "    {} {expr_str};",
                if i == exprs.len() - 1 {
                    "return".into()
                } else {
                    format!("uint32_t x{i} =")
                }
            ))
        }

        lines.push(
            "\
}

uint32_t lookup(const char *key, size_t len) {
    uint32_t i = hash(key, len);
    if (len == entries[i].len && memcmp(key, entries[i].key, len) == 0) {
        return entries[i].value;
    }
    return -1;
}"
            .into(),
        );

        lines.join("\n")
    }
}
