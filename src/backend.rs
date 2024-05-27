use crate::{
    ir::{Instr, Ir, Reg},
    keys::Keys,
};

pub trait Backend {
    fn emit(&self, keys: &Keys, ir: &Ir, table: &[Option<(Vec<u32>, usize)>]) -> String;
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
        let mut s: String = "\"".into();
        for c in key.iter().copied() {
            assert!(c < 256);
            s.push_str(&self.char_escape_table[c as usize])
        }
        s.push('"');
        s
    }
}

impl Backend for CBackend {
    fn emit(&self, keys: &Keys, ir: &Ir, table: &[Option<(Vec<u32>, usize)>]) -> String {
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

        for (i, entry) in table.iter().enumerate() {
            let entry = match entry {
                Some((key, ordinal)) => {
                    let len = key.len();
                    let string_literal = self.string_literal(key);
                    format!("{{ {string_literal}, {len}, {ordinal} }}")
                }
                None => {
                    if i == 0 {
                        let len = keys.end_len;
                        let fake_key = "q".repeat(len);
                        format!("{{ \"{fake_key}\", {len}, -1 }}")
                    } else {
                        "{ \"\", 0, -1 }".into()
                    }
                }
            };
            lines.push(format!("    {entry},"));
        }

        lines.push(
            "\
};

uint32_t hash(const char *key, size_t len) {"
                .into(),
        );

        {
            let start = keys.start_len;
            let end = keys.end_len;
            lines.push(format!("    if (len < {start} || len >= {end}) {{"));
            lines.push(
                "        return 0;
    }"
                .into(),
            );
        }

        for (i, instr) in ir.instrs.iter().enumerate() {
            let expr = match instr {
                Instr::Imm(n) => format!("{n}"),
                Instr::Table(_, _) => todo!(),
                Instr::StrGet(Reg(i)) => format!("((uint32_t) key[r{i}])"),
                Instr::StrLen => "((uint32_t) len)".into(),
                Instr::Add(Reg(a), Reg(b)) => format!("r{a} + r{b}"),
                Instr::Sub(Reg(a), Reg(b)) => format!("r{a} - r{b}"),
                Instr::Mul(Reg(a), Reg(b)) => format!("r{a} * r{b}"),
                Instr::And(Reg(a), Reg(b)) => format!("r{a} & r{b}"),
                Instr::Shll(Reg(a), Reg(b)) => format!("r{a} << r{b}"),
                Instr::Shrl(Reg(a), Reg(b)) => format!("r{a} >> r{b}"),
            };
            lines.push(format!("    uint32_t r{i} = {expr};"));
        }
        lines.push(format!("    return r{};", ir.instrs.len() - 1));

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
