use crate::error::ErrorReporter;
use crate::token::{Token, TokenKind};

pub struct Lexer<'a> {
    input: Vec<u8>,
    pos: usize,
    reporter: &'a ErrorReporter,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &str, reporter: &'a ErrorReporter) -> Self {
        Self {
            input: input.as_bytes().to_vec(),
            pos: 0,
            reporter,
        }
    }

    pub fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();

        while self.pos < self.input.len() {
            let ch = self.input[self.pos] as char;

            if ch.is_ascii_whitespace() {
                self.pos += 1;
                continue;
            }

            // Line comment: //
            if ch == '/' && self.peek_next() == Some('/') {
                while self.pos < self.input.len() && self.input[self.pos] != b'\n' {
                    self.pos += 1;
                }
                continue;
            }

            // Block comment: /* ... */
            if ch == '/' && self.peek_next() == Some('*') {
                self.pos += 2;
                while self.pos + 1 < self.input.len() {
                    if self.input[self.pos] == b'*' && self.input[self.pos + 1] == b'/' {
                        self.pos += 2;
                        break;
                    }
                    self.pos += 1;
                }
                continue;
            }

            // Identifiers and keywords
            if ch.is_ascii_alphabetic() || ch == '_' {
                let pos = self.pos;
                let word = self.read_ident();
                let kind = match word.as_str() {
                    "return" => TokenKind::Return,
                    "int" => TokenKind::Int,
                    "char" => TokenKind::Char,
                    "short" => TokenKind::Short,
                    "long" => TokenKind::Long,
                    "if" => TokenKind::If,
                    "else" => TokenKind::Else,
                    "while" => TokenKind::While,
                    "for" => TokenKind::For,
                    "do" => TokenKind::Do,
                    "switch" => TokenKind::Switch,
                    "case" => TokenKind::Case,
                    "default" => TokenKind::Default,
                    "break" => TokenKind::Break,
                    "continue" => TokenKind::Continue,
                    "goto" => TokenKind::Goto,
                    "sizeof" => TokenKind::Sizeof,
                    "struct" => TokenKind::Struct,
                    "union" => TokenKind::Union,
                    "enum" => TokenKind::Enum,
                    "typedef" => TokenKind::Typedef,
                    "static" => TokenKind::Static,
                    "extern" => TokenKind::Extern,
                    "signed" => TokenKind::Signed,
                    "unsigned" => TokenKind::Unsigned,
                    "void" => TokenKind::Void,
                    "const" => TokenKind::Const,
                    "volatile" => TokenKind::Volatile,
                    "float" => TokenKind::FloatKw,
                    "double" => TokenKind::DoubleKw,
                    "_Bool" => TokenKind::Bool,
                    "_Alignof" => TokenKind::Alignof,
                    "_Alignas" => TokenKind::Alignas,
                    "_Generic" => TokenKind::Generic,
                    "__attribute__" => TokenKind::Attribute,
                    "inline" => TokenKind::Inline,
                    "__inline" => TokenKind::Inline,
                    "__inline__" => TokenKind::Inline,
                    "_Noreturn" => TokenKind::Noreturn,
                    "__noreturn__" => TokenKind::Noreturn,
                    "__extension__" => TokenKind::Extension,
                    "register" => TokenKind::Register,
                    "restrict" => TokenKind::Restrict,
                    "__restrict" => TokenKind::Restrict,
                    "__restrict__" => TokenKind::Restrict,
                    _ => TokenKind::Ident(word),
                };
                tokens.push(Token { kind, pos });
                continue;
            }

            // Character literals
            if ch == '\'' {
                let pos = self.pos;
                let val = self.read_char_literal();
                tokens.push(Token {
                    kind: TokenKind::Num(val as i64),
                    pos,
                });
                continue;
            }

            // String literals
            if ch == '"' {
                let pos = self.pos;
                let s = self.read_string();
                tokens.push(Token {
                    kind: TokenKind::Str(s),
                    pos,
                });
                continue;
            }

            if ch.is_ascii_digit() || (ch == '.' && self.peek_next().is_some_and(|c| c.is_ascii_digit())) {
                let pos = self.pos;
                let (is_float, int_val, float_val) = self.read_number_or_float();
                if is_float {
                    tokens.push(Token {
                        kind: TokenKind::FloatNum(float_val),
                        pos,
                    });
                } else {
                    tokens.push(Token {
                        kind: TokenKind::Num(int_val),
                        pos,
                    });
                }
                continue;
            }

            let pos = self.pos;

            // Three-character tokens: ...
            if ch == '.' && self.peek_next() == Some('.') && self.peek_at(2) == Some('.') {
                self.pos += 3;
                tokens.push(Token { kind: TokenKind::Ellipsis, pos });
                continue;
            }

            // Two-character tokens
            if ch == '=' && self.peek_next() == Some('=') {
                self.pos += 2;
                tokens.push(Token { kind: TokenKind::EqEq, pos });
                continue;
            }
            if ch == '!' && self.peek_next() == Some('=') {
                self.pos += 2;
                tokens.push(Token { kind: TokenKind::Ne, pos });
                continue;
            }
            if ch == '&' && self.peek_next() == Some('&') {
                self.pos += 2;
                tokens.push(Token { kind: TokenKind::AmpAmp, pos });
                continue;
            }
            if ch == '|' && self.peek_next() == Some('|') {
                self.pos += 2;
                tokens.push(Token { kind: TokenKind::PipePipe, pos });
                continue;
            }
            if ch == '<' && self.peek_next() == Some('<') {
                self.pos += 2;
                tokens.push(Token { kind: TokenKind::LShift, pos });
                continue;
            }
            if ch == '<' && self.peek_next() == Some('=') {
                self.pos += 2;
                tokens.push(Token { kind: TokenKind::Le, pos });
                continue;
            }
            if ch == '>' && self.peek_next() == Some('>') {
                self.pos += 2;
                tokens.push(Token { kind: TokenKind::RShift, pos });
                continue;
            }
            if ch == '>' && self.peek_next() == Some('=') {
                self.pos += 2;
                tokens.push(Token { kind: TokenKind::Ge, pos });
                continue;
            }
            if ch == '+' && self.peek_next() == Some('+') {
                self.pos += 2;
                tokens.push(Token { kind: TokenKind::PlusPlus, pos });
                continue;
            }
            if ch == '+' && self.peek_next() == Some('=') {
                self.pos += 2;
                tokens.push(Token { kind: TokenKind::PlusEq, pos });
                continue;
            }
            if ch == '-' && self.peek_next() == Some('>') {
                self.pos += 2;
                tokens.push(Token { kind: TokenKind::Arrow, pos });
                continue;
            }
            if ch == '-' && self.peek_next() == Some('-') {
                self.pos += 2;
                tokens.push(Token { kind: TokenKind::MinusMinus, pos });
                continue;
            }
            if ch == '-' && self.peek_next() == Some('=') {
                self.pos += 2;
                tokens.push(Token { kind: TokenKind::MinusEq, pos });
                continue;
            }
            if ch == '*' && self.peek_next() == Some('=') {
                self.pos += 2;
                tokens.push(Token { kind: TokenKind::StarEq, pos });
                continue;
            }
            if ch == '/' && self.peek_next() == Some('=') {
                self.pos += 2;
                tokens.push(Token { kind: TokenKind::SlashEq, pos });
                continue;
            }
            if ch == '%' && self.peek_next() == Some('=') {
                self.pos += 2;
                tokens.push(Token { kind: TokenKind::PercentEq, pos });
                continue;
            }

            let kind = match ch {
                '+' => TokenKind::Plus,
                '-' => TokenKind::Minus,
                '*' => TokenKind::Star,
                '/' => TokenKind::Slash,
                '%' => TokenKind::Percent,
                '<' => TokenKind::Lt,
                '>' => TokenKind::Gt,
                '=' => TokenKind::Eq,
                '!' => TokenKind::Bang,
                '~' => TokenKind::Tilde,
                '&' => TokenKind::Amp,
                '|' => TokenKind::Pipe,
                '^' => TokenKind::Caret,
                '?' => TokenKind::Question,
                ':' => TokenKind::Colon,
                '.' => TokenKind::Dot,
                ',' => TokenKind::Comma,
                '(' => TokenKind::LParen,
                ')' => TokenKind::RParen,
                '{' => TokenKind::LBrace,
                '}' => TokenKind::RBrace,
                '[' => TokenKind::LBracket,
                ']' => TokenKind::RBracket,
                ';' => TokenKind::Semicolon,
                _ => {
                    self.reporter.error_at(self.pos, &format!("unexpected character '{}'", ch));
                }
            };
            self.pos += 1;
            tokens.push(Token { kind, pos });
        }

        tokens.push(Token {
            kind: TokenKind::Eof,
            pos: self.pos,
        });

        tokens
    }

    fn peek_next(&self) -> Option<char> {
        if self.pos + 1 < self.input.len() {
            Some(self.input[self.pos + 1] as char)
        } else {
            None
        }
    }

    fn peek_at(&self, offset: usize) -> Option<char> {
        if self.pos + offset < self.input.len() {
            Some(self.input[self.pos + offset] as char)
        } else {
            None
        }
    }

    fn read_ident(&mut self) -> String {
        let start = self.pos;
        while self.pos < self.input.len() {
            let c = self.input[self.pos] as char;
            if c.is_ascii_alphanumeric() || c == '_' {
                self.pos += 1;
            } else {
                break;
            }
        }
        String::from_utf8(self.input[start..self.pos].to_vec()).unwrap()
    }

    fn read_char_literal(&mut self) -> u8 {
        self.pos += 1; // skip opening '\''
        let val = if self.input[self.pos] == b'\\' {
            self.pos += 1;
            match self.input[self.pos] {
                b'n' => { self.pos += 1; b'\n' }
                b't' => { self.pos += 1; b'\t' }
                b'r' => { self.pos += 1; b'\r' }
                b'a' => { self.pos += 1; 0x07 }
                b'b' => { self.pos += 1; 0x08 }
                b'f' => { self.pos += 1; 0x0C }
                b'v' => { self.pos += 1; 0x0B }
                b'\\' => { self.pos += 1; b'\\' }
                b'\'' => { self.pos += 1; b'\'' }
                b'"' => { self.pos += 1; b'"' }
                b'0' => { self.pos += 1; 0 }
                b'x' => {
                    self.pos += 1;
                    let mut val = 0u32;
                    while self.pos < self.input.len() && (self.input[self.pos] as char).is_ascii_hexdigit() {
                        val = val * 16 + (self.input[self.pos] as char).to_digit(16).unwrap();
                        self.pos += 1;
                    }
                    val as u8
                }
                d if d >= b'0' && d <= b'7' => {
                    let mut val = (d - b'0') as u32;
                    self.pos += 1;
                    for _ in 0..2 {
                        if self.pos < self.input.len() && self.input[self.pos] >= b'0' && self.input[self.pos] <= b'7' {
                            val = val * 8 + (self.input[self.pos] - b'0') as u32;
                            self.pos += 1;
                        } else {
                            break;
                        }
                    }
                    val as u8
                }
                other => { self.pos += 1; other }
            }
        } else {
            let c = self.input[self.pos];
            self.pos += 1;
            c
        };
        self.pos += 1; // skip closing '\''
        val
    }

    fn read_string(&mut self) -> Vec<u8> {
        self.pos += 1; // skip opening '"'
        let mut s = Vec::new();
        while self.pos < self.input.len() {
            let c = self.input[self.pos];
            if c == b'"' {
                self.pos += 1; // skip closing '"'
                return s;
            }
            if c == b'\\' {
                self.pos += 1;
                if self.pos < self.input.len() {
                    let ch = self.input[self.pos];
                    match ch {
                        b'n' => { s.push(b'\n'); self.pos += 1; }
                        b't' => { s.push(b'\t'); self.pos += 1; }
                        b'r' => { s.push(b'\r'); self.pos += 1; }
                        b'a' => { s.push(0x07); self.pos += 1; } // bell
                        b'b' => { s.push(0x08); self.pos += 1; } // backspace
                        b'f' => { s.push(0x0C); self.pos += 1; } // form feed
                        b'v' => { s.push(0x0B); self.pos += 1; } // vertical tab
                        b'\\' => { s.push(b'\\'); self.pos += 1; }
                        b'\'' => { s.push(b'\''); self.pos += 1; }
                        b'"' => { s.push(b'"'); self.pos += 1; }
                        b'?' => { s.push(b'?'); self.pos += 1; }
                        b'0'..=b'7' => {
                            // Octal escape: 1-3 digits
                            let mut val = (ch - b'0') as u32;
                            self.pos += 1;
                            for _ in 0..2 {
                                if self.pos < self.input.len() {
                                    let d = self.input[self.pos];
                                    if d >= b'0' && d <= b'7' {
                                        val = val * 8 + (d - b'0') as u32;
                                        self.pos += 1;
                                    } else {
                                        break;
                                    }
                                }
                            }
                            s.push(val as u8);
                        }
                        b'x' => {
                            // Hex escape: \xNN
                            self.pos += 1;
                            let mut val = 0u32;
                            while self.pos < self.input.len() {
                                let d = self.input[self.pos] as char;
                                if d.is_ascii_hexdigit() {
                                    val = val * 16 + d.to_digit(16).unwrap();
                                    self.pos += 1;
                                } else {
                                    break;
                                }
                            }
                            s.push(val as u8);
                        }
                        other => { s.push(other); self.pos += 1; }
                    }
                }
                continue;
            }
            s.push(c);
            self.pos += 1;
        }
        s
    }

    /// Read a number (integer or float). Returns (is_float, int_value, float_value).
    fn read_number_or_float(&mut self) -> (bool, i64, f64) {
        let start = self.pos;

        // Check if starting with '.' (like .5)
        if self.pos < self.input.len() && self.input[self.pos] == b'.' {
            return self.read_decimal_float(start, true);
        }

        // Check for 0x (hex), 0b (binary), or 0 (octal) prefix
        if self.pos < self.input.len() && self.input[self.pos] == b'0' {
            if self.pos + 1 < self.input.len() {
                let next = self.input[self.pos + 1];
                // Hex literal: 0x or 0X
                if next == b'x' || next == b'X' {
                    self.pos += 2; // skip '0x'
                    let mut val: i64 = 0;
                    while self.pos < self.input.len() && (self.input[self.pos] as char).is_ascii_hexdigit() {
                        val = val * 16 + (self.input[self.pos] as char).to_digit(16).unwrap() as i64;
                        self.pos += 1;
                    }
                    self.skip_int_suffix();
                    return (false, val, 0.0);
                }
                // Binary literal: 0b or 0B
                if next == b'b' || next == b'B' {
                    self.pos += 2; // skip '0b'
                    let mut val: i64 = 0;
                    while self.pos < self.input.len() && (self.input[self.pos] == b'0' || self.input[self.pos] == b'1') {
                        val = val * 2 + (self.input[self.pos] - b'0') as i64;
                        self.pos += 1;
                    }
                    self.skip_int_suffix();
                    return (false, val, 0.0);
                }
                // Octal literal: 0 followed by octal digits
                if next >= b'0' && next <= b'7' {
                    self.pos += 1; // skip leading '0'
                    let mut val: i64 = 0;
                    while self.pos < self.input.len() && self.input[self.pos] >= b'0' && self.input[self.pos] <= b'7' {
                        val = val * 8 + (self.input[self.pos] - b'0') as i64;
                        self.pos += 1;
                    }
                    self.skip_int_suffix();
                    return (false, val, 0.0);
                }
            }
        }

        // Decimal integer or float
        self.read_decimal_float(start, false)
    }

    /// Read a decimal integer or float literal starting from `start`.
    fn read_decimal_float(&mut self, start: usize, starts_with_dot: bool) -> (bool, i64, f64) {
        let mut is_float = starts_with_dot;

        // Read integer part
        while self.pos < self.input.len() && (self.input[self.pos] as char).is_ascii_digit() {
            self.pos += 1;
        }

        // Check for decimal point
        if self.pos < self.input.len() && self.input[self.pos] == b'.' {
            // Make sure it's not '...' (ellipsis)
            if self.pos + 1 < self.input.len() && self.input[self.pos + 1] == b'.' {
                // This is not a float, it's an integer followed by '..'
                let s = std::str::from_utf8(&self.input[start..self.pos]).unwrap();
                return (false, s.parse().unwrap(), 0.0);
            }
            is_float = true;
            self.pos += 1; // consume '.'
            while self.pos < self.input.len() && (self.input[self.pos] as char).is_ascii_digit() {
                self.pos += 1;
            }
        }

        // Check for exponent
        if self.pos < self.input.len() && (self.input[self.pos] == b'e' || self.input[self.pos] == b'E') {
            is_float = true;
            self.pos += 1;
            if self.pos < self.input.len() && (self.input[self.pos] == b'+' || self.input[self.pos] == b'-') {
                self.pos += 1;
            }
            while self.pos < self.input.len() && (self.input[self.pos] as char).is_ascii_digit() {
                self.pos += 1;
            }
        }

        // Check for 'f'/'F' suffix (float literal)
        if self.pos < self.input.len() && (self.input[self.pos] == b'f' || self.input[self.pos] == b'F') {
            is_float = true;
            self.pos += 1;
        }

        // Skip integer suffixes
        if !is_float {
            self.skip_int_suffix();
        }

        let s = std::str::from_utf8(&self.input[start..self.pos]).unwrap();
        if is_float {
            // Remove trailing 'f'/'F' for parsing
            let num_str = s.trim_end_matches(|c| c == 'f' || c == 'F');
            let val: f64 = num_str.parse().unwrap();
            (true, 0, val)
        } else {
            let val: i64 = s.parse().unwrap_or(0);
            (false, val, 0.0)
        }
    }

    /// Skip integer suffixes: L, l, U, u, LL, ll, ULL, etc.
    fn skip_int_suffix(&mut self) {
        while self.pos < self.input.len() {
            let c = self.input[self.pos];
            if c == b'L' || c == b'l' || c == b'U' || c == b'u' {
                self.pos += 1;
            } else {
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokenize(input: &str) -> Vec<Token> {
        let reporter = ErrorReporter::new("test", input);
        let mut lexer = Lexer::new(input, &reporter);
        lexer.tokenize()
    }

    #[test]
    fn test_single_number() {
        let tokens = tokenize("42");
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].kind, TokenKind::Num(42));
        assert_eq!(tokens[1].kind, TokenKind::Eof);
    }

    #[test]
    fn test_addition() {
        let tokens = tokenize("1+2");
        assert_eq!(tokens.len(), 4);
        assert_eq!(tokens[0].kind, TokenKind::Num(1));
        assert_eq!(tokens[1].kind, TokenKind::Plus);
        assert_eq!(tokens[2].kind, TokenKind::Num(2));
        assert_eq!(tokens[3].kind, TokenKind::Eof);
    }

    #[test]
    fn test_keywords_and_ident() {
        let tokens = tokenize("int main return");
        assert_eq!(tokens[0].kind, TokenKind::Int);
        assert_eq!(tokens[1].kind, TokenKind::Ident("main".to_string()));
        assert_eq!(tokens[2].kind, TokenKind::Return);
    }

    #[test]
    fn test_braces_semicolon() {
        let tokens = tokenize("{ return 42; }");
        assert_eq!(tokens[0].kind, TokenKind::LBrace);
        assert_eq!(tokens[1].kind, TokenKind::Return);
        assert_eq!(tokens[2].kind, TokenKind::Num(42));
        assert_eq!(tokens[3].kind, TokenKind::Semicolon);
        assert_eq!(tokens[4].kind, TokenKind::RBrace);
    }

    #[test]
    fn test_whitespace() {
        let tokens = tokenize(" 12 + 34 - 5 ");
        assert_eq!(tokens.len(), 6);
        assert_eq!(tokens[0].kind, TokenKind::Num(12));
        assert_eq!(tokens[1].kind, TokenKind::Plus);
        assert_eq!(tokens[2].kind, TokenKind::Num(34));
        assert_eq!(tokens[3].kind, TokenKind::Minus);
        assert_eq!(tokens[4].kind, TokenKind::Num(5));
        assert_eq!(tokens[5].kind, TokenKind::Eof);
    }
}
