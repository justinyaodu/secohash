mod c_expr;
mod c_str_formatter;
mod lines;

use crate::{
    ir::{constant_propagation, BinOp, Expr, Instr, Table, Tac, Var},
    search::Phf,
    spec::Spec,
    util::to_u32,
};
use c_expr::{CBinOp, CExpr, CExprBuilder};
use c_str_formatter::CStrFormatter;
use lines::Lines;
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

    fn compile_str_sum(lines: &mut Lines, mask: u32) {
        lines.push("__attribute__((optimize(\"no-tree-vectorize\")))");
        lines.push(&format!(
            "uint32_t str_sum_{mask}(const char* key, size_t len) {{"
        ));
        let body_indent = lines.indent();

        let x = CExprBuilder();

        let mut shift_stride = 1;
        while shift_stride <= mask {
            shift_stride <<= 1;
        }

        let unroll = 4;

        let unrolled = unroll > 1;
        if unrolled {
            for lane in 0..unroll {
                lines.push(&format!("uint32_t sum_{lane} = 0;"));
            }
            lines.push("size_t i = 0;");
            lines.push(&format!(
                "for (; i + {} < len; i += {unroll}) {{",
                unroll - 1
            ));
            let for_indent = lines.indent();

            let shift_later = unroll >= shift_stride;

            for lane in 0..unroll {
                lines.push(&format!(
                    "sum_{lane} += {};",
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

            lines.dedent(for_indent);
            lines.push("}");

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

            lines.push(&format!("uint32_t sum = {};", shifted_sum));
        } else {
            lines.push("uint32_t sum = 0;");
        };

        lines.push(&format!(
            "for ({}; i < len; i++) {{",
            if unrolled { "" } else { "size_t i = 0" }
        ));
        let for_indent = lines.indent();
        lines.push(&format!(
            "sum += {};",
            x.shl(x.index("key", x.var("i")), x.and(x.var("i"), x.imm(mask)))
                .cleaned()
        ));
        lines.dedent(for_indent);
        lines.push("}");
        lines.push("return sum;");

        lines.dedent(body_indent);
        lines.push("}");
    }

    fn compile_array(lines: &mut Lines, declaration: &str, elements: &[String]) {
        let one_liner = format!("{declaration} = {{{}}};", elements.join(", "));
        if one_liner.len() <= lines.text_cols() {
            lines.push(&one_liner);
            return;
        }

        lines.push(&format!("{declaration} = {{"));
        let arr_indent = lines.indent();

        let elements = elements.iter().map(|e| format!("{e},")).collect::<Vec<_>>();
        lines.fill(&elements);
        lines.dedent(arr_indent);
        lines.push("};");
    }

    pub fn emit(&self) -> String {
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

        let mut lines = Lines::new(80, 4, true);

        lines.extend(&[
            "#include <stddef.h>",
            "#include <stdint.h>",
            "#include <string.h>",
            "",
            "struct entry {",
            "\tchar* key;",
            "\tuint32_t len;",
            "\tuint32_t value;",
            "};",
        ]);

        lines.push_empty();
        let str_formatter = CStrFormatter::new();
        let mut entry_structs = Vec::new();
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

            entry_structs.push(format!("{{{string_literal}, {len}, {ordinal}}}"));
        }
        Self::compile_array(
            &mut lines,
            &format!("const struct entry entries[{}]", entry_structs.len()),
            &entry_structs,
        );

        let mut str_sum_masks = HashSet::new();
        for instr in tac.instrs() {
            if let Instr::StrSum(mask) = *instr {
                str_sum_masks.insert(mask);
            }
        }
        let mut str_sum_masks: Vec<u32> = str_sum_masks.into_iter().collect();
        str_sum_masks.sort();
        for mask in str_sum_masks {
            lines.push_empty();
            Self::compile_str_sum(&mut lines, mask);
        }

        lines.push_empty();
        lines.push(&format!(
            "uint32_t hash({}{key_declaration}, {len_declaration}) {{",
            if key_used { "" } else { unused_prefix },
        ));
        let hash_indent = lines.indent();

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
            let elements = table.iter().map(|x| x.to_string()).collect::<Vec<_>>();
            Self::compile_array(&mut lines, &declaration, &elements)
        }

        {
            let min = spec.min_interpreted_key_len;
            let max = spec.max_interpreted_key_len;
            let condition = if min == max {
                format!("len != {min}")
            } else {
                format!("len < {min} || len > {max}")
            };
            lines.push(&format!("if ({condition}) {{"));
            lines.push("\treturn 0;");
            lines.push("}");
        }

        let exprs = tac.unflatten_dag().0;
        for (i, expr) in exprs.iter().enumerate() {
            let expr_str = self.expr_to_c_expr(expr).cleaned().to_string();
            if i == exprs.len() - 1 {
                lines.push(&format!("return {expr_str};"));
            } else {
                lines.push(&format!("uint32_t x{i} = {expr_str};"));
            }
        }

        lines.dedent(hash_indent);
        lines.push("}");

        lines.extend(&[
            "",
            &format!("uint32_t lookup({key_declaration}, {len_declaration}) {{"),
            "\tuint32_t i = hash(key, len);",
            "\tif (len == entries[i].len && memcmp(key, entries[i].key, len) == 0) {",
            "\t\treturn entries[i].value;",
            "\t}",
            "\treturn -1;",
            "}",
        ]);

        let mut lines: Vec<String> = lines.into();
        for line in lines.iter_mut() {
            line.push('\n');
        }
        lines.join("")
    }
}
