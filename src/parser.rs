use crate::ast::{BinOp, Expr, Function, Stmt, UnaryOp};
use crate::error::ErrorReporter;
use crate::token::{Token, TokenKind};

pub struct Parser<'a> {
    tokens: Vec<Token>,
    pos: usize,
    reporter: &'a ErrorReporter,
    locals: Vec<String>,
}

impl<'a> Parser<'a> {
    pub fn new(tokens: Vec<Token>, reporter: &'a ErrorReporter) -> Self {
        Self { tokens, pos: 0, reporter, locals: Vec::new() }
    }

    // program = function
    pub fn parse(&mut self) -> Function {
        let func = self.function();
        if self.current().kind != TokenKind::Eof {
            self.reporter.error_at(
                self.current().pos,
                &format!("unexpected token: {:?}", self.current().kind),
            );
        }
        func
    }

    // function = "int" ident "(" ")" "{" stmt* "}"
    fn function(&mut self) -> Function {
        self.expect(TokenKind::Int);
        let name = match &self.current().kind {
            TokenKind::Ident(s) => s.clone(),
            _ => {
                self.reporter.error_at(
                    self.current().pos,
                    &format!("expected function name, but got {:?}", self.current().kind),
                );
            }
        };
        self.advance();
        self.expect(TokenKind::LParen);
        self.expect(TokenKind::RParen);
        self.expect(TokenKind::LBrace);

        self.locals.clear();
        let mut body = Vec::new();
        while self.current().kind != TokenKind::RBrace {
            body.push(self.stmt());
        }
        self.expect(TokenKind::RBrace);

        Function { name, body }
    }

    // stmt = "return" expr ";"
    //      | "if" "(" expr ")" stmt ("else" stmt)?
    //      | "int" ident ("=" expr)? ";"
    //      | expr ";"
    fn stmt(&mut self) -> Stmt {
        match &self.current().kind {
            TokenKind::Return => {
                self.advance();
                let expr = self.expr();
                self.expect(TokenKind::Semicolon);
                Stmt::Return(expr)
            }
            TokenKind::If => {
                self.advance();
                self.expect(TokenKind::LParen);
                let cond = self.expr();
                self.expect(TokenKind::RParen);
                let then_stmt = self.stmt();
                let else_stmt = if self.current().kind == TokenKind::Else {
                    self.advance();
                    Some(Box::new(self.stmt()))
                } else {
                    None
                };
                Stmt::If {
                    cond,
                    then_stmt: Box::new(then_stmt),
                    else_stmt,
                }
            }
            TokenKind::While => {
                self.advance();
                self.expect(TokenKind::LParen);
                let cond = self.expr();
                self.expect(TokenKind::RParen);
                let body = self.stmt();
                Stmt::While {
                    cond,
                    body: Box::new(body),
                }
            }
            TokenKind::For => {
                self.advance();
                self.expect(TokenKind::LParen);

                // init
                let init = if self.current().kind == TokenKind::Semicolon {
                    self.advance();
                    None
                } else if self.current().kind == TokenKind::Int {
                    Some(Box::new(self.var_decl()))
                } else {
                    let expr = self.expr();
                    self.expect(TokenKind::Semicolon);
                    Some(Box::new(Stmt::ExprStmt(expr)))
                };

                // cond
                let cond = if self.current().kind == TokenKind::Semicolon {
                    None
                } else {
                    Some(self.expr())
                };
                self.expect(TokenKind::Semicolon);

                // inc
                let inc = if self.current().kind == TokenKind::RParen {
                    None
                } else {
                    Some(self.expr())
                };
                self.expect(TokenKind::RParen);

                let body = self.stmt();

                Stmt::For {
                    init,
                    cond,
                    inc,
                    body: Box::new(body),
                }
            }
            TokenKind::LBrace => {
                self.advance();
                let mut stmts = Vec::new();
                while self.current().kind != TokenKind::RBrace {
                    stmts.push(self.stmt());
                }
                self.expect(TokenKind::RBrace);
                Stmt::Block(stmts)
            }
            TokenKind::Int => {
                self.var_decl()
            }
            _ => {
                let expr = self.expr();
                self.expect(TokenKind::Semicolon);
                Stmt::ExprStmt(expr)
            }
        }
    }

    // var_decl = "int" ident ("=" expr)? ";"
    fn var_decl(&mut self) -> Stmt {
        self.expect(TokenKind::Int);
        let name = match &self.current().kind {
            TokenKind::Ident(s) => s.clone(),
            _ => {
                self.reporter.error_at(
                    self.current().pos,
                    "expected variable name",
                );
            }
        };
        self.advance();

        if !self.locals.contains(&name) {
            self.locals.push(name.clone());
        }

        let init = if self.current().kind == TokenKind::Eq {
            self.advance();
            Some(self.expr())
        } else {
            None
        };
        self.expect(TokenKind::Semicolon);
        Stmt::VarDecl { name, init }
    }

    // expr = assign
    fn expr(&mut self) -> Expr {
        self.assign()
    }

    // assign = logical_or ("=" assign | "+=" assign | "-=" assign | "*=" assign | "/=" assign | "%=" assign)?
    fn assign(&mut self) -> Expr {
        let node = self.logical_or();

        if self.current().kind == TokenKind::Eq {
            self.advance();
            let rhs = self.assign();
            return Expr::Assign {
                lhs: Box::new(node),
                rhs: Box::new(rhs),
            };
        }


        // Compound assignment: desugar a op= b into a = a op b
        let op = match self.current().kind {
            TokenKind::PlusEq => Some(BinOp::Add),
            TokenKind::MinusEq => Some(BinOp::Sub),
            TokenKind::StarEq => Some(BinOp::Mul),
            TokenKind::SlashEq => Some(BinOp::Div),
            TokenKind::PercentEq => Some(BinOp::Mod),
            _ => None,
        };

        if let Some(op) = op {
            self.advance();
            let rhs = self.assign();
            return Expr::Assign {
                lhs: Box::new(node.clone()),
                rhs: Box::new(Expr::BinOp {
                    op,
                    lhs: Box::new(node),
                    rhs: Box::new(rhs),
                }),
            };
        }

        node
    }

    // logical_or = logical_and ("||" logical_and)*
    fn logical_or(&mut self) -> Expr {
        let mut node = self.logical_and();

        while self.current().kind == TokenKind::PipePipe {
            self.advance();
            let rhs = self.logical_and();
            node = Expr::LogicalOr(Box::new(node), Box::new(rhs));
        }

        node
    }

    // logical_and = equality ("&&" equality)*
    fn logical_and(&mut self) -> Expr {
        let mut node = self.equality();

        while self.current().kind == TokenKind::AmpAmp {
            self.advance();
            let rhs = self.equality();
            node = Expr::LogicalAnd(Box::new(node), Box::new(rhs));
        }

        node
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

    // unary = ("+" | "-") unary | "++" unary | "--" unary | postfix
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
            TokenKind::Bang => {
                self.advance();
                let operand = self.unary();
                Expr::UnaryOp {
                    op: UnaryOp::LogicalNot,
                    operand: Box::new(operand),
                }
            }
            TokenKind::PlusPlus => {
                self.advance();
                let operand = self.unary();
                Expr::PreInc(Box::new(operand))
            }
            TokenKind::MinusMinus => {
                self.advance();
                let operand = self.unary();
                Expr::PreDec(Box::new(operand))
            }
            _ => self.postfix(),
        }
    }

    // postfix = primary ("++" | "--")*
    fn postfix(&mut self) -> Expr {
        let mut node = self.primary();

        loop {
            match self.current().kind {
                TokenKind::PlusPlus => {
                    self.advance();
                    node = Expr::PostInc(Box::new(node));
                }
                TokenKind::MinusMinus => {
                    self.advance();
                    node = Expr::PostDec(Box::new(node));
                }
                _ => break,
            }
        }

        node
    }

    // primary = num | ident | "(" expr ")"
    fn primary(&mut self) -> Expr {
        match self.current().kind.clone() {
            TokenKind::Num(val) => {
                self.advance();
                Expr::Num(val)
            }
            TokenKind::Ident(name) => {
                self.advance();
                Expr::Var(name)
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
                    &format!("expected a number, identifier or '(', but got {:?}", self.current().kind),
                );
            }
        }
    }

    pub fn get_locals(&self) -> &[String] {
        &self.locals
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

    fn parse_func(input: &str) -> Function {
        let reporter = crate::error::ErrorReporter::new("test", input);
        let mut lexer = Lexer::new(input, &reporter);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens, &reporter);
        parser.parse()
    }

    #[test]
    fn test_return_number() {
        let func = parse_func("int main() { return 42; }");
        assert_eq!(func.name, "main");
        assert_eq!(func.body.len(), 1);
        assert_eq!(func.body[0], Stmt::Return(Expr::Num(42)));
    }

    #[test]
    fn test_expr_stmt() {
        let func = parse_func("int main() { 1; 2; return 3; }");
        assert_eq!(func.body.len(), 3);
        assert_eq!(func.body[0], Stmt::ExprStmt(Expr::Num(1)));
        assert_eq!(func.body[1], Stmt::ExprStmt(Expr::Num(2)));
        assert_eq!(func.body[2], Stmt::Return(Expr::Num(3)));
    }

    #[test]
    fn test_return_add() {
        let func = parse_func("int main() { return 1 + 2; }");
        assert_eq!(func.body.len(), 1);
        match &func.body[0] {
            Stmt::Return(Expr::BinOp { op: BinOp::Add, .. }) => {}
            _ => panic!("expected return with add"),
        }
    }

    #[test]
    fn test_var_decl() {
        let func = parse_func("int main() { int a; a = 3; return a; }");
        assert_eq!(func.body.len(), 3);
        assert_eq!(func.body[0], Stmt::VarDecl { name: "a".to_string(), init: None });
    }

    #[test]
    fn test_var_with_init() {
        let func = parse_func("int main() { int a = 5; return a; }");
        assert_eq!(func.body.len(), 2);
        assert_eq!(
            func.body[0],
            Stmt::VarDecl { name: "a".to_string(), init: Some(Expr::Num(5)) }
        );
    }
}
