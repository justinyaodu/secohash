mod c_expr;
mod c_str_formatter;
mod format_array;

use crate::{
    ir::{constant_propagation, BinOp, Expr, Instr, Table, Tac, Var},
    search::Phf,
    spec::Spec,
    util::to_u32,
};
use c_expr::{CBinOp, CExpr, CExprBuilder};
use c_str_formatter::CStrFormatter;
use format_array::format_array;
use std::collections::{BTreeMap, HashMap, HashSet};

pub struct CBackend {
    spec: Spec,
    phf: Phf,
}

impl CBackend {
    pub fn new(spec: Spec, phf: Phf) -> CBackend {
        CBackend { spec, phf }
    }

    fn expr_to_c_expr(&self, expr: &Expr) -> CExpr {
        let x = CExprBuilder();
        match *expr {
            Expr::Var(Var(i)) => x.var(format!("x{i}")),
            Expr::Reg(_) => panic!(),
            Expr::Imm(n) => x.imm(n),
            Expr::StrGet(ref i) => x.index("key", self.expr_to_c_expr(i.as_ref())),
            Expr::StrLen => x.cast("uint32_t", x.var("len")),
            Expr::StrSum(mask) => {
                x.call(format!("str_sum_{mask}"), vec![x.var("key"), x.var("len")])
            }
            Expr::TableGet(Table(t), ref i) => {
                x.index(format!("t{t}"), self.expr_to_c_expr(i.as_ref()))
            }
            Expr::TableIndexMask(t) => x.imm(to_u32(self.phf.tables[t].len() - 1)),
            Expr::HashMask => x.imm(to_u32(self.phf.key_table.len() - 1)),
            Expr::BinOp(op, ref a, ref b) => {
                let op = match op {
                    BinOp::Add => CBinOp::Add,
                    BinOp::Sub => CBinOp::Sub,
                    BinOp::And => CBinOp::And,
                    BinOp::Shll => CBinOp::Shl,
                    BinOp::Shrl => CBinOp::Shr,
                };
                let a = self.expr_to_c_expr(a.as_ref());
                let b = self.expr_to_c_expr(b.as_ref());
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

        let mut body = Vec::new();
        let unrolled = unroll > 1;
        if unrolled {
            for lane in 0..unroll {
                body.push(format!("uint32_t sum_{lane} = 0;"));
            }
            body.push("size_t i = 0;".into());
            body.push(format!(
                "for (; i + {} < len; i += {unroll}) {{",
                unroll - 1
            ));

            let shift_later = unroll >= shift_stride;

            for lane in 0..unroll {
                body.push(format!(
                    "\tsum_{lane} += {};",
                    x.shl(
                        x.index("key", x.add(x.var("i"), x.imm(lane))),
                        if shift_later {
                            x.imm(0)
                        } else {
                            x.and(x.add(x.var("i"), x.imm(lane)), x.imm(mask))
                        }
                    )
                    .cleaned()
                ));
            }

            body.push("}".into());

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

            body.push(format!("uint32_t sum = {};", shifted_sum));
        } else {
            body.push("uint32_t sum = 0;".into());
        };

        body.extend([
            format!(
                "for ({}; i < len; i++) {{",
                if unrolled { "" } else { "size_t i = 0" }
            ),
            format!(
                "\tsum += {};",
                x.shl(x.index("key", x.var("i")), x.and(x.var("i"), x.imm(mask)))
                    .cleaned()
            ),
            "}".into(),
            "return sum;".into(),
        ]);

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

    pub fn emit(&self) -> String {
        let tab_size = 4;

        let spec = &self.spec;
        let phf = &self.phf;

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

        for (i, table) in phf.tables.tables().iter().enumerate() {
            let max = table.iter().copied().max().unwrap();
            let mut table_type = "uint32_t";
            if max <= u16::MAX.into() {
                table_type = "uint16_t";
            }
            if max <= u8::MAX.into() {
                table_type = "uint8_t";
            }

            let table_size = table.len();

            let declaration = format!("static const {table_type} t{i}[{table_size}]");

            for table_line in format_array(80 - tab_size, tab_size, &declaration, table) {
                lines.push(format!("\t{table_line}"));
            }
        }

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

        let exprs = tac.unflatten_dag().0;
        for (i, expr) in exprs.iter().enumerate() {
            let expr_str = self.expr_to_c_expr(expr).cleaned().to_string();
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

        lines.join("\n").replace('\t', &" ".repeat(tab_size))
    }
}
