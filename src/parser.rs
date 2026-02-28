use crate::ast::{BinOp, Expr, UnaryOp};
use crate::token::{Token, TokenKind};

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    pub fn parse(&mut self) -> Expr {
        let expr = self.expr();
        if self.current().kind != TokenKind::Eof {
            eprintln!("Unexpected token: {:?}", self.current().kind);
            std::process::exit(1);
        }
        expr
    }

    // expr = mul ("+" mul | "-" mul)*
    fn expr(&mut self) -> Expr {
        let mut node = self.mul();

        loop {
            match self.current().kind {
                TokenKind::Plus => {
                    self.advance();
                    let rhs = self.mul();
                    node = Expr::BinOp {
                        op: BinOp::Add,
                        lhs: Box::new(node),
                        rhs: Box::new(rhs),
                    };
                }
                TokenKind::Minus => {
                    self.advance();
                    let rhs = self.mul();
                    node = Expr::BinOp {
                        op: BinOp::Sub,
                        lhs: Box::new(node),
                        rhs: Box::new(rhs),
                    };
                }
                _ => break,
            }
        }

        node
    }

    // mul = unary ("*" unary | "/" unary)*
    fn mul(&mut self) -> Expr {
        let mut node = self.unary();

        loop {
            match self.current().kind {
                TokenKind::Star => {
                    self.advance();
                    let rhs = self.unary();
                    node = Expr::BinOp {
                        op: BinOp::Mul,
                        lhs: Box::new(node),
                        rhs: Box::new(rhs),
                    };
                }
                TokenKind::Slash => {
                    self.advance();
                    let rhs = self.unary();
                    node = Expr::BinOp {
                        op: BinOp::Div,
                        lhs: Box::new(node),
                        rhs: Box::new(rhs),
                    };
                }
                _ => break,
            }
        }

        node
    }

    // unary = ("+" | "-") unary | primary
    fn unary(&mut self) -> Expr {
        match self.current().kind {
            TokenKind::Plus => {
                self.advance();
                self.unary()
            }
            TokenKind::Minus => {
                self.advance();
                let operand = self.unary();
                Expr::UnaryOp {
                    op: UnaryOp::Neg,
                    operand: Box::new(operand),
                }
            }
            _ => self.primary(),
        }
    }

    // primary = num | "(" expr ")"
    fn primary(&mut self) -> Expr {
        match self.current().kind {
            TokenKind::Num(val) => {
                self.advance();
                Expr::Num(val)
            }
            TokenKind::LParen => {
                self.advance();
                let node = self.expr();
                self.expect(TokenKind::RParen);
                node
            }
            _ => {
                eprintln!("Expected a number or '(', but got {:?}", self.current().kind);
                std::process::exit(1);
            }
        }
    }

    fn current(&self) -> &Token {
        &self.tokens[self.pos]
    }

    fn advance(&mut self) {
        self.pos += 1;
    }

    fn expect(&mut self, kind: TokenKind) {
        if self.current().kind != kind {
            eprintln!("Expected {:?}, but got {:?}", kind, self.current().kind);
            std::process::exit(1);
        }
        self.advance();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;

    fn parse(input: &str) -> Expr {
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        parser.parse()
    }

    #[test]
    fn test_number() {
        assert_eq!(parse("42"), Expr::Num(42));
    }

    #[test]
    fn test_add() {
        assert_eq!(
            parse("1 + 2"),
            Expr::BinOp {
                op: BinOp::Add,
                lhs: Box::new(Expr::Num(1)),
                rhs: Box::new(Expr::Num(2)),
            }
        );
    }

    #[test]
    fn test_precedence() {
        // 5 + 6 * 7 should be 5 + (6 * 7)
        assert_eq!(
            parse("5 + 6 * 7"),
            Expr::BinOp {
                op: BinOp::Add,
                lhs: Box::new(Expr::Num(5)),
                rhs: Box::new(Expr::BinOp {
                    op: BinOp::Mul,
                    lhs: Box::new(Expr::Num(6)),
                    rhs: Box::new(Expr::Num(7)),
                }),
            }
        );
    }

    #[test]
    fn test_paren() {
        // (2 + 3) * 4
        assert_eq!(
            parse("(2 + 3) * 4"),
            Expr::BinOp {
                op: BinOp::Mul,
                lhs: Box::new(Expr::BinOp {
                    op: BinOp::Add,
                    lhs: Box::new(Expr::Num(2)),
                    rhs: Box::new(Expr::Num(3)),
                }),
                rhs: Box::new(Expr::Num(4)),
            }
        );
    }

    #[test]
    fn test_unary_neg() {
        assert_eq!(
            parse("-10"),
            Expr::UnaryOp {
                op: UnaryOp::Neg,
                operand: Box::new(Expr::Num(10)),
            }
        );
    }
}
