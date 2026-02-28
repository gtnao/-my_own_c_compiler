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

            // Identifiers and keywords
            if ch.is_ascii_alphabetic() || ch == '_' {
                let pos = self.pos;
                let word = self.read_ident();
                let kind = match word.as_str() {
                    "return" => TokenKind::Return,
                    "int" => TokenKind::Int,
                    "char" => TokenKind::Char,
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
                    "void" => TokenKind::Void,
                    _ => TokenKind::Ident(word),
                };
                tokens.push(Token { kind, pos });
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
                ',' => TokenKind::Comma,
                '(' => TokenKind::LParen,
                ')' => TokenKind::RParen,
                '{' => TokenKind::LBrace,
                '}' => TokenKind::RBrace,
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
