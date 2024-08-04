pub fn format_array(
    cols: usize,
    tab_size: usize,
    declaration: &str,
    values: &[u32],
) -> Vec<String> {
    let joined = values
        .iter()
        .map(|x| x.to_string())
        .collect::<Vec<_>>()
        .join(", ");
    let one_liner = format!("{declaration} = {{{}}};", joined);
    if one_liner.len() <= cols {
        return vec![one_liner];
    }
    let mut lines = vec![format!("{declaration} = {{")];
    let value_cols = cols - tab_size;
    let mut i = 0;
    while i < joined.len() {
        if joined.as_bytes()[i] == b' ' {
            i += 1;
        } else {
            let mut end = i;
            let mut j = i;
            while j < joined.len() && (end == i || j - i < value_cols) {
                j += 1;
                if j == joined.len() || joined.as_bytes()[j] == b' ' {
                    end = j;
                }
            }
            lines.push("\t".to_string() + &joined[i..end]);
            i = end;
        }
    }
    lines.push("};".into());
    lines
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_one_liner() {
        assert_eq!(
            format_array(13, 4, "foo", &[0, 1]),
            vec!["foo = {0, 1};".to_string()]
        );
        assert_eq!(
            format_array(14, 4, "foo", &[0, 1]),
            vec!["foo = {0, 1};".to_string()]
        );
    }

    #[test]
    fn test_long_items() {
        assert_eq!(
            format_array(13, 4, "foo", &[123456789, 0, 0, 0, 123456789, 0, 123456789]),
            vec![
                "foo = {".to_string(),
                "\t123456789,".to_string(),
                "\t0, 0, 0,".to_string(),
                "\t123456789,".to_string(),
                "\t0,".to_string(),
                "\t123456789".to_string(),
                "};".to_string(),
            ]
        );
    }
}
