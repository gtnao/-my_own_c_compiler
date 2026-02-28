use crate::ast::{BinOp, Expr, UnaryOp};
use crate::error::ErrorReporter;
use crate::token::{Token, TokenKind};

pub struct Parser<'a> {
    tokens: Vec<Token>,
    pos: usize,
    reporter: &'a ErrorReporter,
}

impl<'a> Parser<'a> {
    pub fn new(tokens: Vec<Token>, reporter: &'a ErrorReporter) -> Self {
        Self { tokens, pos: 0, reporter }
    }

    pub fn parse(&mut self) -> Expr {
        let expr = self.expr();
        if self.current().kind != TokenKind::Eof {
            self.reporter.error_at(
                self.current().pos,
                &format!("unexpected token: {:?}", self.current().kind),
            );
        }
        expr
    }

    // expr = equality
    fn expr(&mut self) -> Expr {
        self.equality()
    }

    // equality = relational ("==" relational | "!=" relational)*
    fn equality(&mut self) -> Expr {
        let mut node = self.relational();

        loop {
            match self.current().kind {
                TokenKind::EqEq => {
                    self.advance();
                    let rhs = self.relational();
                    node = Expr::BinOp {
                        op: BinOp::Eq,
                        lhs: Box::new(node),
                        rhs: Box::new(rhs),
                    };
                }
                TokenKind::Ne => {
                    self.advance();
                    let rhs = self.relational();
                    node = Expr::BinOp {
                        op: BinOp::Ne,
                        lhs: Box::new(node),
                        rhs: Box::new(rhs),
                    };
                }
                _ => break,
            }
        }

        node
    }

    // relational = add ("<" add | "<=" add | ">" add | ">=" add)*
    fn relational(&mut self) -> Expr {
        let mut node = self.add();

        loop {
            match self.current().kind {
                TokenKind::Lt => {
                    self.advance();
                    let rhs = self.add();
                    node = Expr::BinOp {
                        op: BinOp::Lt,
                        lhs: Box::new(node),
                        rhs: Box::new(rhs),
                    };
                }
                TokenKind::Le => {
                    self.advance();
                    let rhs = self.add();
                    node = Expr::BinOp {
                        op: BinOp::Le,
                        lhs: Box::new(node),
                        rhs: Box::new(rhs),
                    };
                }
                TokenKind::Gt => {
                    self.advance();
                    let rhs = self.add();
                    node = Expr::BinOp {
                        op: BinOp::Gt,
                        lhs: Box::new(node),
                        rhs: Box::new(rhs),
                    };
                }
                TokenKind::Ge => {
                    self.advance();
                    let rhs = self.add();
                    node = Expr::BinOp {
                        op: BinOp::Ge,
                        lhs: Box::new(node),
                        rhs: Box::new(rhs),
                    };
                }
                _ => break,
            }
        }

        node
    }

    // add = mul ("+" mul | "-" mul)*
    fn add(&mut self) -> Expr {
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

    // mul = unary ("*" unary | "/" unary | "%" unary)*
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
                TokenKind::Percent => {
                    self.advance();
                    let rhs = self.unary();
                    node = Expr::BinOp {
                        op: BinOp::Mod,
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
                self.reporter.error_at(
                    self.current().pos,
                    &format!("expected a number or '(', but got {:?}", self.current().kind),
                );
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
            self.reporter.error_at(
                self.current().pos,
                &format!("expected {:?}, but got {:?}", kind, self.current().kind),
            );
        }
        self.advance();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;

    fn parse(input: &str) -> Expr {
        let reporter = crate::error::ErrorReporter::new("test", input);
        let mut lexer = Lexer::new(input, &reporter);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens, &reporter);
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
