pub struct ErrorReporter {
    source: String,
    filename: String,
}

impl ErrorReporter {
    pub fn new(filename: &str, source: &str) -> Self {
        Self {
            source: source.to_string(),
            filename: filename.to_string(),
        }
    }

    pub fn error_at(&self, pos: usize, msg: &str) -> ! {
        let (line_num, col, line_str) = self.get_location(pos);

        eprintln!("{}:{}:{}: \x1b[1;31merror:\x1b[0m {}", self.filename, line_num, col + 1, msg);
        eprintln!("{}", line_str);
        eprintln!("{}\x1b[1;32m^\x1b[0m", " ".repeat(col));

        std::process::exit(1);
    }

    pub fn warn_at(&self, pos: usize, msg: &str) {
        let (line_num, col, line_str) = self.get_location(pos);

        eprintln!("{}:{}:{}: \x1b[1;35mwarning:\x1b[0m {}", self.filename, line_num, col + 1, msg);
        eprintln!("{}", line_str);
        eprintln!("{}\x1b[1;32m^\x1b[0m", " ".repeat(col));
    }

    fn get_location(&self, pos: usize) -> (usize, usize, &str) {
        let bytes = self.source.as_bytes();
        let mut line_num = 1;
        let mut line_start = 0;

        for i in 0..pos.min(bytes.len()) {
            if bytes[i] == b'\n' {
                line_num += 1;
                line_start = i + 1;
            }
        }

        let col = pos - line_start;

        let mut line_end = pos;
        while line_end < bytes.len() && bytes[line_end] != b'\n' {
            line_end += 1;
        }

        let line_str = &self.source[line_start..line_end];
        (line_num, col, line_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_location_single_line() {
        let reporter = ErrorReporter::new("test.c", "1 + 2");
        let (line, col, line_str) = reporter.get_location(2);
        assert_eq!(line, 1);
        assert_eq!(col, 2);
        assert_eq!(line_str, "1 + 2");
    }

    #[test]
    fn test_get_location_multi_line() {
        let reporter = ErrorReporter::new("test.c", "abc\ndef\nghi");
        let (line, col, line_str) = reporter.get_location(5); // 'e' in "def"
        assert_eq!(line, 2);
        assert_eq!(col, 1);
        assert_eq!(line_str, "def");
    }
}
