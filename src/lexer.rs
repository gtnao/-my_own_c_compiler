use crate::token::{Token, TokenKind};

pub struct Lexer {
    input: Vec<u8>,
    pos: usize,
}

impl Lexer {
    pub fn new(input: &str) -> Self {
        Self {
            input: input.as_bytes().to_vec(),
            pos: 0,
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
            let kind = match ch {
                '+' => TokenKind::Plus,
                '-' => TokenKind::Minus,
                '*' => TokenKind::Star,
                '/' => TokenKind::Slash,
                '%' => TokenKind::Percent,
                '(' => TokenKind::LParen,
                ')' => TokenKind::RParen,
                _ => {
                    eprintln!("Unexpected character '{}' at position {}", ch, self.pos);
                    std::process::exit(1);
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

    #[test]
    fn test_single_number() {
        let mut lexer = Lexer::new("42");
        let tokens = lexer.tokenize();
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].kind, TokenKind::Num(42));
        assert_eq!(tokens[1].kind, TokenKind::Eof);
    }

    #[test]
    fn test_addition() {
        let mut lexer = Lexer::new("1+2");
        let tokens = lexer.tokenize();
        assert_eq!(tokens.len(), 4);
        assert_eq!(tokens[0].kind, TokenKind::Num(1));
        assert_eq!(tokens[1].kind, TokenKind::Plus);
        assert_eq!(tokens[2].kind, TokenKind::Num(2));
        assert_eq!(tokens[3].kind, TokenKind::Eof);
    }

    #[test]
    fn test_whitespace() {
        let mut lexer = Lexer::new(" 12 + 34 - 5 ");
        let tokens = lexer.tokenize();
        assert_eq!(tokens.len(), 6);
        assert_eq!(tokens[0].kind, TokenKind::Num(12));
        assert_eq!(tokens[1].kind, TokenKind::Plus);
        assert_eq!(tokens[2].kind, TokenKind::Num(34));
        assert_eq!(tokens[3].kind, TokenKind::Minus);
        assert_eq!(tokens[4].kind, TokenKind::Num(5));
        assert_eq!(tokens[5].kind, TokenKind::Eof);
    }
}
