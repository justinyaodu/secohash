use std::{collections::HashMap, fmt::Display};

use crate::{
    ir::{remove_zero_shifts, BinOp, Expr, Instr, Table, Tac, Var},
    search::Phf,
    spec::Spec,
};

pub trait Backend {
    fn emit(&self, spec: &Spec, phf: &Phf) -> String;
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
            Some(BinOp::Add | BinOp::Sub | BinOp::And | BinOp::Shll | BinOp::Shrl) => 1,
        }
    }

    fn compile_expr_rec(phf: &Phf, expr: &Expr) -> (String, Option<BinOp>) {
        match *expr {
            Expr::Var(Var(i)) => (format!("x{i}"), None),
            Expr::Reg(_) => panic!(),
            Expr::Imm(n) => (n.to_string(), None),
            Expr::StrGet(ref i) => (format!("key[{}]", Self::compile_expr_rec(phf, i).0), None),
            Expr::StrLen => ("(uint32_t) len".into(), None),
            Expr::StrSum => ("str_sum(key, len)".into(), None),
            Expr::TableGet(Table(t), ref i) => {
                (format!("t{t}[{}]", Self::compile_expr_rec(phf, i).0), None)
            }
            Expr::TableIndexMask(t) => ((phf.tables[t].len() - 1).to_string(), None),
            Expr::HashMask => ((phf.key_table.len() - 1).to_string(), None),
            Expr::BinOp(op, ref a, ref b) => {
                let op_str = match op {
                    BinOp::Add => "+",
                    BinOp::Sub => "-",
                    BinOp::And => "&",
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
struct Declaration {
    used: bool,
    declaration: String,
}

impl Declaration {
    fn new(used: bool, declaration: &str) -> Declaration {
        Declaration {
            used,
            declaration: declaration.to_string(),
        }
    }

    fn with_unused_attribute(&self) -> String {
        format!(
            "{}{}",
            if self.used {
                ""
            } else {
                "__attribute__((unused)) "
            },
            self.declaration
        )
    }
}

impl Display for Declaration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.declaration)
    }
}

impl Backend for CBackend {
    fn emit(&self, spec: &Spec, phf: &Phf) -> String {
        let expr = phf.tac.unflatten_tree(phf.tac.last_reg(), &HashMap::new());
        let expr = remove_zero_shifts(expr);
        let mut tac = Tac::new();
        expr.flatten(&mut tac, &HashMap::new());
        let (tac, _) = tac.local_value_numbering();

        let key_used = tac.instrs().iter().any(|i| matches!(&i, Instr::StrGet(_)));
        let key = Declaration::new(key_used, "const char* key");
        let len = Declaration::new(true, "size_t len");

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

        for (i, key) in phf.key_table.iter().enumerate() {
            let string_literal = self.string_literal(key);

            let len = key.len();
            let is_fake_key = (i == 0) ^ (key.is_empty());
            let ordinal: String = if is_fake_key {
                "-1".into()
            } else {
                spec.keys.iter().position(|k| k == key).unwrap().to_string()
            };

            let entry = format!("{{ {string_literal}, {len}, {ordinal} }}");
            lines.push(format!("    {entry},"));
        }

        lines.push(format!(
            "}};

uint32_t str_sum(const char* key, size_t len) {{
    uint32_t sum = 0;
    for (size_t i = 0; i < len; i++) {{
        sum += key[i] << (i & 3);
    }}
    return sum;
}}

uint32_t hash({}, {}) {{",
            key.with_unused_attribute(),
            len.with_unused_attribute()
        ));

        {
            let min = spec.min_interpreted_key_len;
            let max = spec.max_interpreted_key_len;
            lines.push(format!(
                "    if (len < {min} || len > {max}) {{
        return 0;
    }}"
            ));
        }

        for (i, table) in phf.tables.tables().iter().enumerate() {
            let nums = table
                .iter()
                .map(|n| n.to_string())
                .collect::<Vec<_>>()
                .join(", ");
            lines.push(format!("    static const uint32_t t{i}[] = {{ {nums} }};"));
        }

        let exprs = tac.unflatten_dag().0;
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

        lines.push(format!(
            "}}

uint32_t lookup({key}, {len}) {{
    uint32_t i = hash(key, len);
    if (len == entries[i].len && memcmp(key, entries[i].key, len) == 0) {{
        return entries[i].value;
    }}
    return -1;
}}"
        ));

        lines.join("\n")
    }
}
