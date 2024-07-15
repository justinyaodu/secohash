use super::pos::Pos;

fn parse(pos: &mut Pos, input: &str, delimiter: char) -> Result<Vec<(Pos, String)>, String> {
    let mut parsed = Vec::new();
    let mut cur_start_pos = pos.clone();
    let mut cur_str = String::new();

    let mut at_delimiter = true;
    for char in input.chars() {
        pos.advance(char);
        at_delimiter = char == delimiter;
        if at_delimiter {
            parsed.push((cur_start_pos, cur_str));
            cur_start_pos = pos.clone();
            cur_str = String::new();
        } else {
            cur_str.push(char);
        }
    }

    if at_delimiter {
        Ok(parsed)
    } else {
        Err(format!(
            "{}: expected a trailing delimiter {:?}",
            pos, delimiter
        ))
    }
}

pub fn parse_strings(input: &str, delimiter: char) -> Result<Vec<String>, String> {
    let parsed = parse(&mut Pos::new(), input, delimiter)?;
    Ok(parsed.into_iter().map(|x| x.1).collect())
}

pub fn parse_int_lists(input: &str) -> Result<Vec<Vec<u32>>, String> {
    let mut pos = Pos::new();
    let lines = parse(&mut pos, input, '\n')?;
    let mut lists = Vec::new();
    for (mut line_pos, line) in lines {
        let split = parse(&mut line_pos, &line, ';')?;
        let mut list = Vec::new();
        for (word_pos, word) in split {
            match word.parse::<u32>() {
                Ok(n) => list.push(n),
                Err(err) => {
                    return Err(format!(
                        "{}: cannot parse {:?} as u32: {}",
                        word_pos, word, err
                    ))
                }
            }
        }
        lists.push(list);
    }
    Ok(lists)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse_strings_empty() {
        assert_eq!(parse_strings("", ','), Ok(Vec::new()));
    }

    #[test]
    fn test_parse_strings_valid() {
        assert_eq!(
            parse_strings("apple,banana,", ','),
            Ok(vec!["apple".into(), "banana".into()])
        );
    }

    #[test]
    fn test_parse_strings_one_delimiter() {
        assert_eq!(parse_strings(",", ','), Ok(vec!["".into()]));
    }

    #[test]
    fn test_parse_strings_missing_delimiter() {
        assert_eq!(
            parse_strings("apple,banana,cherry", ','),
            Err("line 1 col 20: expected a trailing delimiter ','".into())
        );
    }

    #[test]
    fn test_parse_int_lists_empty() {
        assert_eq!(parse_int_lists(""), Ok(Vec::new()));
    }

    #[test]
    fn test_parse_int_lists_valid() {
        assert_eq!(
            parse_int_lists("\n10;\n20;30;\n"),
            Ok(vec![vec![], vec![10], vec![20, 30]])
        );
    }

    #[test]
    fn test_parse_int_lists_missing_newline() {
        assert_eq!(
            parse_int_lists("10;"),
            Err("line 1 col 4: expected a trailing delimiter '\\n'".into())
        );
    }

    #[test]
    fn test_parse_int_lists_missing_semicolon() {
        assert_eq!(
            parse_int_lists("10;\n20;30\n"),
            Err("line 2 col 6: expected a trailing delimiter ';'".into())
        );
    }

    #[test]
    fn test_parse_int_lists_invalid_int() {
        assert_eq!(
            parse_int_lists("foo;\n"),
            Err("line 1 col 1: cannot parse \"foo\" as u32: invalid digit found in string".into())
        );
    }
}
