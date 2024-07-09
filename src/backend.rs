use crate::phf::{BinOp, Instr, Phf, Reg, Table};

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
                .map(|n| format!("{n}"))
                .collect::<Vec<_>>()
                .join(", ");
            lines.push(format!("    static const uint8_t t{i}[] = {{ {nums} }};"));
        }

        for (i, instr) in phf.instrs.iter().enumerate() {
            let expr = match instr {
                Instr::Imm(n) => format!("{n}"),
                Instr::StrGet(Reg(i)) => format!("(uint32_t) key[r{i}]"),
                Instr::StrLen => "(uint32_t) len".into(),
                Instr::TableGet(Table(t), Reg(i)) => format!("(uint32_t) t{t}[r{i}]"),
                Instr::TableIndexMask(Table(t)) => {
                    format!("{}", (phf.data_tables[*t].len() - 1) as u32)
                }
                Instr::HashMask => {
                    format!("{}", (phf.hash_table.as_ref().unwrap().len() - 1) as u32)
                }
                Instr::BinOp(op, Reg(a), Reg(b)) => {
                    let op = match op {
                        BinOp::Add => "+",
                        BinOp::Sub => "-",
                        BinOp::Mul => "*",
                        BinOp::And => "&",
                        BinOp::Xor => "^",
                        BinOp::Shll => "<<",
                        BinOp::Shrl => ">>",
                    };
                    format!("r{a} {op} r{b}")
                }
            };
            lines.push(format!("    uint32_t r{i} = {expr};"));
        }
        lines.push(format!("    return r{};", phf.instrs.len() - 1));

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
