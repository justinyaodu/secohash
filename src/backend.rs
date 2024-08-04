mod c_expr;
mod c_str_formatter;

use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fmt::Display,
};

use c_expr::{CBinOp, CExpr, CExprBuilder};
use c_str_formatter::CStrFormatter;

use crate::{
    ir::{constant_propagation, BinOp, Expr, Instr, Table, Tac, Var},
    search::Phf,
    spec::Spec,
    util::to_u32,
};

pub trait Backend {
    fn emit(&self, spec: &Spec, phf: &Phf) -> String;
}

pub struct CBackend();

impl CBackend {
    pub fn new() -> CBackend {
        CBackend()
    }

    fn expr_to_c_expr(phf: &Phf, expr: &Expr) -> CExpr {
        let x = CExprBuilder();
        match *expr {
            Expr::Var(Var(i)) => x.var(format!("x{i}")),
            Expr::Reg(_) => panic!(),
            Expr::Imm(n) => x.imm(n),
            Expr::StrGet(ref i) => x.index("key".into(), Self::expr_to_c_expr(phf, i.as_ref())),
            Expr::StrLen => x.cast("uint32_t".into(), x.var("len".into())),
            Expr::StrSum(mask) => x.call(
                format!("str_sum_{mask}"),
                vec![x.var("key".into()), x.var("len".into())],
            ),
            Expr::TableGet(Table(t), ref i) => {
                x.index(format!("t{t}"), Self::expr_to_c_expr(phf, i.as_ref()))
            }
            Expr::TableIndexMask(t) => x.imm(to_u32(phf.tables[t].len() - 1)),
            Expr::HashMask => x.imm(to_u32(phf.key_table.len() - 1)),
            Expr::BinOp(op, ref a, ref b) => {
                let op = match op {
                    BinOp::Add => CBinOp::Add,
                    BinOp::Sub => CBinOp::Sub,
                    BinOp::And => CBinOp::And,
                    BinOp::Shll => CBinOp::Shl,
                    BinOp::Shrl => CBinOp::Shr,
                };
                let a = Self::expr_to_c_expr(phf, a.as_ref());
                let b = Self::expr_to_c_expr(phf, b.as_ref());
                x.bin_op(op, a, b)
            }
        }
    }

    fn compile_str_sum(mask: u32) -> Vec<String> {
        let x = CExprBuilder();

        let mut shift_stride = 1;
        while shift_stride <= mask {
            shift_stride <<= 1;
        }

        let unroll = 4;

        let body = if unroll > 1 {
            let mut lines = Vec::new();

            for lane in 0..unroll {
                lines.push(format!("uint32_t sum_{lane} = 0;"));
            }
            lines.push("size_t i = 0;".into());
            lines.push(format!(
                "for (; i + {} < len; i += {unroll}) {{",
                unroll - 1
            ));

            let shift_later = unroll >= shift_stride;

            for lane in 0..unroll {
                lines.push(format!(
                    "\tsum_{lane} += {};",
                    x.shl(
                        x.index("key".into(), x.add(x.var("i".into()), x.imm(lane))),
                        if shift_later {
                            x.imm(0)
                        } else {
                            x.and(x.add(x.var("i".into()), x.imm(lane)), x.imm(mask))
                        }
                    )
                    .cleaned()
                ));
            }

            lines.push("}".into());

            let mut shifts_and_sums: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
            for lane in 0..unroll {
                let shift = if shift_later { lane & mask } else { 0 };
                shifts_and_sums.entry(shift).or_default().push(lane);
            }

            let shifted_sum = x
                .sum(
                    shifts_and_sums
                        .into_iter()
                        .map(|(shift, sums)| {
                            x.shl(
                                x.sum(
                                    sums.into_iter()
                                        .map(|i| x.var(format!("sum_{i}")))
                                        .collect(),
                                ),
                                x.imm(shift),
                            )
                        })
                        .collect(),
                )
                .cleaned();

            let compute_sum = format!("uint32_t sum = {};", shifted_sum);
            lines.push(compute_sum);

            lines.extend([
                "for (; i < len; i++) {".into(),
                format!(
                    "\tsum += {};",
                    x.shl(
                        x.index("key".into(), x.var("i".into())),
                        x.and(x.var("i".into()), x.imm(mask))
                    )
                    .cleaned()
                ),
                "}".into(),
            ]);

            lines.push("return sum;".into());

            lines
        } else {
            vec![
                "uint32_t sum = 0;".into(),
                "for (size_t i = 0; i < len; i++) {".into(),
                format!(
                    "\tsum += {};",
                    x.shl(
                        x.index("key".into(), x.var("i".into())),
                        x.and(x.var("i".into()), x.imm(mask))
                    )
                    .cleaned()
                ),
                "}".into(),
                "return sum;".into(),
            ]
        };

        let mut lines = Vec::new();
        lines.push("__attribute__((optimize(\"no-tree-vectorize\")))".into());
        lines.push(format!(
            "uint32_t str_sum_{mask}(const char* key, size_t len) {{"
        ));
        for body_line in body {
            lines.push(format!("\t{body_line}"));
        }
        lines.push("}".into());
        lines
    }
}

impl Backend for CBackend {
    fn emit(&self, spec: &Spec, phf: &Phf) -> String {
        let expr = phf.tac.unflatten_tree(phf.tac.last_reg(), &HashMap::new());
        let expr = constant_propagation(expr);
        let mut tac = Tac::new();
        expr.flatten(&mut tac, &HashMap::new());
        let (tac, _) = tac.local_value_numbering();

        let key_used = tac.instrs().iter().any(|i| matches!(&i, Instr::StrGet(_)));

        let unused_prefix = "__attribute__((unused)) ";
        let key_declaration = "const char* key";
        let len_declaration = "size_t len";

        let mut lines: Vec<String> = vec![
            "#include <stddef.h>".into(),
            "#include <stdint.h>".into(),
            "#include <string.h>".into(),
            "".into(),
            "struct entry {".into(),
            "\tchar* key;".into(),
            "\tuint32_t len;".into(),
            "\tuint32_t value;".into(),
            "};".into(),
            "".into(),
            "const struct entry entries[] = {".into(),
        ];

        let str_formatter = CStrFormatter::new();
        for (i, key) in phf.key_table.iter().enumerate() {
            let bytes = key.iter().map(|&c| u8::try_from(c).unwrap()).collect();
            let string_literal = str_formatter.format(bytes);

            let len = key.len();
            let is_fake_key = (i == 0) ^ (key.is_empty());
            let ordinal: String = if is_fake_key {
                "-1".into()
            } else {
                spec.keys.iter().position(|k| k == key).unwrap().to_string()
            };

            let entry = format!("{{ {string_literal}, {len}, {ordinal} }}");
            lines.push(format!("\t{entry},"));
        }

        lines.push("};".into());
        lines.push("".into());

        let mut str_sum_masks = HashSet::new();
        for instr in tac.instrs() {
            if let Instr::StrSum(mask) = *instr {
                str_sum_masks.insert(mask);
            }
        }
        let mut str_sum_masks: Vec<u32> = str_sum_masks.into_iter().collect();
        str_sum_masks.sort();
        for mask in str_sum_masks {
            lines.extend(Self::compile_str_sum(mask));
            lines.push("".into());
        }

        lines.push(format!(
            "uint32_t hash({}{key_declaration}, {len_declaration}) {{",
            if key_used { "" } else { unused_prefix },
        ));

        {
            let min = spec.min_interpreted_key_len;
            let max = spec.max_interpreted_key_len;
            let condition = if min == max {
                format!("len != {min}")
            } else {
                format!("len < {min} || len > {max}")
            };
            lines.extend([
                format!("\tif ({condition}) {{"),
                "\t\treturn 0;".into(),
                "\t}".into(),
            ]);
        }

        for (i, table) in phf.tables.tables().iter().enumerate() {
            let nums = table
                .iter()
                .map(|n| n.to_string())
                .collect::<Vec<_>>()
                .join(", ");

            let max = table.iter().copied().max().unwrap();
            let mut table_type = "uint32_t";
            if max <= u16::MAX.into() {
                table_type = "uint16_t";
            }
            if max <= u8::MAX.into() {
                table_type = "uint8_t";
            }

            let table_size = table.len();

            lines.push(format!(
                "\tstatic const {table_type} t{i}[{table_size}] = {{ {nums} }};",
            ));
        }

        let exprs = tac.unflatten_dag().0;
        for (i, expr) in exprs.iter().enumerate() {
            let expr_str = Self::expr_to_c_expr(phf, expr).cleaned().to_string();
            lines.push(format!(
                "\t{} {expr_str};",
                if i == exprs.len() - 1 {
                    "return".into()
                } else {
                    format!("uint32_t x{i} =")
                }
            ))
        }
        lines.push("}".into());

        lines.extend([
            "".into(),
            format!("uint32_t lookup({key_declaration}, {len_declaration}) {{"),
            "\tuint32_t i = hash(key, len);".into(),
            "\tif (len == entries[i].len && memcmp(key, entries[i].key, len) == 0) {".into(),
            "\t\treturn entries[i].value;".into(),
            "\t}".into(),
            "\treturn -1;".into(),
            "}".into(),
        ]);

        lines.join("\n").replace('\t', "    ")
    }
}
