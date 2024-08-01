pub struct CStrFormatter {
    char_escape_table: Vec<String>,
}

impl CStrFormatter {
    pub fn new() -> CStrFormatter {
        CStrFormatter {
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
            ('"', "\\\""),
            ('\\', "\\\\"),
            ('\n', "\\n"),
            ('\r', "\\r"),
            ('\t', "\\t"),
        ] {
            char_escape_table[char as usize] = escaped.into();
        }

        char_escape_table
    }

    pub fn format(&self, bytes: Vec<u8>) -> String {
        let mut s = String::new();
        s.push('"');
        for &b in &bytes {
            s.push_str(&self.char_escape_table[usize::from(b)])
        }
        s.push('"');
        s
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_empty() {
        let formatted = CStrFormatter::new().format(vec![]);
        assert_eq!(formatted, "\"\"");
    }

    #[test]
    fn test_escapes() {
        let bytes = vec![
            b'?', b'"', b'\\', b'\n', b'\r', b'\t', b' ', b'a', b'~', 0, 127,
        ];
        let formatted = CStrFormatter::new().format(bytes);
        assert_eq!(formatted, r#""\?\"\\\n\r\t a~\000\177""#);
    }
}
