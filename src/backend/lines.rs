pub struct Lines {
    line_len: usize,
    indent_width: usize,
    indent_with_spaces: bool,
    indent_level: usize,
    lines: Vec<String>,
}

#[must_use]
pub struct Indent(usize);

impl Lines {
    pub fn new(line_len: usize, indent_width: usize, indent_with_spaces: bool) -> Lines {
        Lines {
            line_len,
            indent_width,
            indent_with_spaces,
            indent_level: 0,
            lines: Vec::new(),
        }
    }

    pub fn push(&mut self, mut line: &str) {
        let mut indent_level = self.indent_level;
        while line.starts_with('\t') {
            indent_level += 1;
            line = &line[1..];
        }

        if line.is_empty() {
            self.lines.push("".to_string());
            return;
        }

        let mut temp = if self.indent_with_spaces {
            " ".repeat(indent_level * self.indent_width)
        } else {
            "\t".repeat(indent_level)
        };
        temp.push_str(line);
        self.lines.push(temp);
    }

    pub fn extend(&mut self, lines: &[&str]) {
        for line in lines {
            self.push(line);
        }
    }

    pub fn fill(&mut self, items: &[String]) {
        let sep = " ";
        let cols = self.text_cols();
        let mut i = 0;
        while i < items.len() {
            let mut line = String::with_capacity(cols);
            line.push_str(&items[i]);
            i += 1;
            // In principle, we should use the display widths of the strings, but
            // counting bytes is a lot easier.
            // See also: https://github.com/psf/black/issues/1197
            while i < items.len() && line.len() + sep.len() + items[i].len() <= cols {
                line.push_str(sep);
                line.push_str(&items[i]);
                i += 1;
            }
            self.push(&line);
        }
    }

    pub fn push_empty(&mut self) {
        self.push("")
    }

    pub fn indent(&mut self) -> Indent {
        self.indent_level += 1;
        Indent(self.indent_level)
    }

    pub fn dedent(&mut self, indent: Indent) {
        assert!(self.indent_level == indent.0);
        self.indent_level -= 1;
    }

    pub fn text_cols(&self) -> usize {
        usize::saturating_sub(self.line_len, self.indent_level * self.indent_width)
    }
}

impl From<Lines> for Vec<String> {
    fn from(lines: Lines) -> Self {
        lines.lines
    }
}
