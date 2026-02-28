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
                    "unsigned" => TokenKind::Unsigned,
                    "void" => TokenKind::Void,
                    "_Bool" => TokenKind::Bool,
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

            if ch.is_ascii_digit() {
                let pos = self.pos;
                let val = self.read_number();
                tokens.push(Token {
                    kind: TokenKind::Num(val),
                    pos,
                });
                continue;
            }

            let pos = self.pos;

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

    fn read_number(&mut self) -> i64 {
        let start = self.pos;
        while self.pos < self.input.len() && (self.input[self.pos] as char).is_ascii_digit() {
            self.pos += 1;
        }
        let s = std::str::from_utf8(&self.input[start..self.pos]).unwrap();
        s.parse().unwrap()
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
