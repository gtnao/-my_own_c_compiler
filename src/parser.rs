use std::collections::HashMap;

use crate::ast::{BinOp, Expr, Function, Program, Stmt, UnaryOp};
use crate::error::ErrorReporter;
use crate::token::{Token, TokenKind};
use crate::types::Type;

pub struct Parser<'a> {
    tokens: Vec<Token>,
    pos: usize,
    reporter: &'a ErrorReporter,
    locals: Vec<(Type, String)>,
    scopes: Vec<HashMap<String, String>>,
    unique_counter: usize,
    globals: Vec<(Type, String)>,
}

impl<'a> Parser<'a> {
    pub fn new(tokens: Vec<Token>, reporter: &'a ErrorReporter) -> Self {
        Self {
            tokens,
            pos: 0,
            reporter,
            locals: Vec::new(),
            scopes: Vec::new(),
            unique_counter: 0,
            globals: Vec::new(),
        }
    }

    // program = (function | prototype | global_var)*
    pub fn parse(&mut self) -> Program {
        let mut functions = Vec::new();
        while self.current().kind != TokenKind::Eof {
            if self.is_function() {
                if let Some(func) = self.function_or_prototype() {
                    functions.push(func);
                }
            } else {
                self.global_var();
            }
        }
        Program {
            globals: self.globals.clone(),
            functions,
        }
    }

    fn is_type_keyword(kind: &TokenKind) -> bool {
        matches!(kind, TokenKind::Int | TokenKind::Char | TokenKind::Short | TokenKind::Long | TokenKind::Void | TokenKind::Unsigned | TokenKind::Bool)
    }

    fn is_function(&self) -> bool {
        // type ident "(" → function/prototype
        // Handles multi-token types like "unsigned int"
        if !Self::is_type_keyword(&self.tokens[self.pos].kind) {
            return false;
        }
        let mut i = self.pos;
        while Self::is_type_keyword(&self.tokens[i].kind) {
            i += 1;
        }
        // Skip pointer stars
        while self.tokens[i].kind == TokenKind::Star {
            i += 1;
        }
        if let TokenKind::Ident(_) = &self.tokens[i].kind {
            return self.tokens[i + 1].kind == TokenKind::LParen;
        }
        false
    }

    fn global_var(&mut self) {
        // type ident ";"
        let ty = self.parse_type();
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
        self.expect(TokenKind::Semicolon);
        self.globals.push((ty, name));
    }

    /// Parse a type specifier and return the corresponding Type.
    fn parse_type(&mut self) -> Type {
        let is_unsigned = if self.current().kind == TokenKind::Unsigned {
            self.advance();
            true
        } else {
            false
        };

        let mut ty = match self.current().kind {
            TokenKind::Int => {
                self.advance();
                if is_unsigned { Type::uint() } else { Type::int_type() }
            }
            TokenKind::Char => {
                self.advance();
                if is_unsigned { Type::uchar() } else { Type::char_type() }
            }
            TokenKind::Short => {
                self.advance();
                if is_unsigned { Type::ushort() } else { Type::short_type() }
            }
            TokenKind::Long => {
                self.advance();
                if is_unsigned { Type::ulong() } else { Type::long_type() }
            }
            TokenKind::Void => {
                self.advance();
                Type::void()
            }
            TokenKind::Bool => {
                self.advance();
                Type::bool_type()
            }
            _ => {
                if is_unsigned {
                    // bare "unsigned" = "unsigned int"
                    Type::uint()
                } else {
                    self.reporter.error_at(
                        self.current().pos,
                        &format!("expected type, but got {:?}", self.current().kind),
                    );
                }
            }
        };

        // Parse pointer stars: type "*"*
        while self.current().kind == TokenKind::Star {
            self.advance();
            ty = Type::ptr_to(ty);
        }

        ty
    }

    // function_or_prototype = type ident "(" params? ")" ("{" stmt* "}" | ";")
    fn function_or_prototype(&mut self) -> Option<Function> {
        let return_ty = self.parse_type();

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

        self.locals.clear();
        self.scopes.clear();
        self.unique_counter = 0;
        self.enter_scope();
        let mut params = Vec::new();

        // Parse parameter list: (type ident ("," type ident)*)?
        if self.current().kind != TokenKind::RParen {
            let param_ty = self.parse_type();
            let param_name = match &self.current().kind {
                TokenKind::Ident(s) => s.clone(),
                _ => {
                    self.reporter.error_at(
                        self.current().pos,
                        "expected parameter name",
                    );
                }
            };
            self.advance();
            let unique = self.declare_var(&param_name, param_ty.clone());
            params.push((param_ty, unique));

            while self.current().kind == TokenKind::Comma {
                self.advance();
                let param_ty = self.parse_type();
                let param_name = match &self.current().kind {
                    TokenKind::Ident(s) => s.clone(),
                    _ => {
                        self.reporter.error_at(
                            self.current().pos,
                            "expected parameter name",
                        );
                    }
                };
                self.advance();
                let unique = self.declare_var(&param_name, param_ty.clone());
                params.push((param_ty, unique));
            }
        }
        self.expect(TokenKind::RParen);

        // Forward declaration (prototype): ends with ";"
        if self.current().kind == TokenKind::Semicolon {
            self.advance();
            return None;
        }

        // Function definition: has body
        self.expect(TokenKind::LBrace);

        let mut body = Vec::new();
        while self.current().kind != TokenKind::RBrace {
            body.push(self.stmt());
        }
        self.expect(TokenKind::RBrace);
        self.leave_scope();

        let locals = self.locals.clone();
        Some(Function { name, return_ty, params, body, locals })
    }

    // stmt = "return" expr ";"
    //      | "if" "(" expr ")" stmt ("else" stmt)?
    //      | "int" ident ("=" expr)? ";"
    //      | expr ";"
    fn stmt(&mut self) -> Stmt {
        match &self.current().kind {
            TokenKind::Return => {
                self.advance();
                if self.current().kind == TokenKind::Semicolon {
                    self.advance();
                    Stmt::Return(None)
                } else {
                    let expr = self.expr();
                    self.expect(TokenKind::Semicolon);
                    Stmt::Return(Some(expr))
                }
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
                } else if Self::is_type_keyword(&self.current().kind) && self.current().kind != TokenKind::Void {
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
            TokenKind::Do => {
                self.advance();
                let body = self.stmt();
                self.expect(TokenKind::While);
                self.expect(TokenKind::LParen);
                let cond = self.expr();
                self.expect(TokenKind::RParen);
                self.expect(TokenKind::Semicolon);
                Stmt::DoWhile {
                    body: Box::new(body),
                    cond,
                }
            }
            TokenKind::Switch => {
                self.advance();
                self.expect(TokenKind::LParen);
                let cond = self.expr();
                self.expect(TokenKind::RParen);
                self.expect(TokenKind::LBrace);

                let mut cases = Vec::new();
                let mut default = None;

                while self.current().kind != TokenKind::RBrace {
                    if self.current().kind == TokenKind::Case {
                        self.advance();
                        let val = match &self.current().kind {
                            TokenKind::Num(n) => *n,
                            _ => {
                                self.reporter.error_at(
                                    self.current().pos,
                                    "expected integer constant in case",
                                );
                            }
                        };
                        self.advance();
                        self.expect(TokenKind::Colon);

                        let mut stmts = Vec::new();
                        while self.current().kind != TokenKind::Case
                            && self.current().kind != TokenKind::Default
                            && self.current().kind != TokenKind::RBrace
                        {
                            stmts.push(self.stmt());
                        }
                        cases.push((val, stmts));
                    } else if self.current().kind == TokenKind::Default {
                        self.advance();
                        self.expect(TokenKind::Colon);

                        let mut stmts = Vec::new();
                        while self.current().kind != TokenKind::Case
                            && self.current().kind != TokenKind::Default
                            && self.current().kind != TokenKind::RBrace
                        {
                            stmts.push(self.stmt());
                        }
                        default = Some(stmts);
                    } else {
                        self.reporter.error_at(
                            self.current().pos,
                            "expected case or default in switch",
                        );
                    }
                }
                self.expect(TokenKind::RBrace);

                Stmt::Switch { cond, cases, default }
            }
            TokenKind::Break => {
                self.advance();
                self.expect(TokenKind::Semicolon);
                Stmt::Break
            }
            TokenKind::Continue => {
                self.advance();
                self.expect(TokenKind::Semicolon);
                Stmt::Continue
            }
            TokenKind::Goto => {
                self.advance();
                let name = match &self.current().kind {
                    TokenKind::Ident(s) => s.clone(),
                    _ => {
                        self.reporter.error_at(
                            self.current().pos,
                            "expected label name after goto",
                        );
                    }
                };
                self.advance();
                self.expect(TokenKind::Semicolon);
                Stmt::Goto(name)
            }
            TokenKind::LBrace => {
                self.advance();
                self.enter_scope();
                let mut stmts = Vec::new();
                while self.current().kind != TokenKind::RBrace {
                    stmts.push(self.stmt());
                }
                self.expect(TokenKind::RBrace);
                self.leave_scope();
                Stmt::Block(stmts)
            }
            TokenKind::Int | TokenKind::Char | TokenKind::Short | TokenKind::Long | TokenKind::Unsigned | TokenKind::Bool => {
                self.var_decl()
            }
            _ => {
                // Check for label: "ident :"
                if let TokenKind::Ident(name) = &self.current().kind {
                    if self.pos + 1 < self.tokens.len()
                        && self.tokens[self.pos + 1].kind == TokenKind::Colon
                    {
                        let name = name.clone();
                        self.advance(); // ident
                        self.advance(); // :
                        let stmt = self.stmt();
                        return Stmt::Label {
                            name,
                            stmt: Box::new(stmt),
                        };
                    }
                }

                let expr = self.expr();
                self.expect(TokenKind::Semicolon);
                Stmt::ExprStmt(expr)
            }
        }
    }

    // var_decl = type ident ("[" num "]")* ("=" expr)? ";"
    fn var_decl(&mut self) -> Stmt {
        let ty = self.parse_type();
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

        // Array declaration: ident ("[" num "]")*
        let ty = {
            let mut dims = Vec::new();
            while self.current().kind == TokenKind::LBracket {
                self.advance();
                let len = match &self.current().kind {
                    TokenKind::Num(n) => *n as usize,
                    _ => {
                        self.reporter.error_at(
                            self.current().pos,
                            "expected array size",
                        );
                    }
                };
                self.advance();
                self.expect(TokenKind::RBracket);
                dims.push(len);
            }
            let mut ty = ty;
            // Build type from innermost to outermost:
            // int a[2][3] → Array(Array(Int, 3), 2)
            for &len in dims.iter().rev() {
                ty = Type::array_of(ty, len);
            }
            ty
        };

        let unique = self.declare_var(&name, ty.clone());

        let init = if self.current().kind == TokenKind::Eq {
            self.advance();
            Some(self.expr())
        } else {
            None
        };
        self.expect(TokenKind::Semicolon);
        Stmt::VarDecl { name: unique, ty, init }
    }

    // expr = assign ("," assign)*
    fn expr(&mut self) -> Expr {
        let mut node = self.assign();

        while self.current().kind == TokenKind::Comma {
            self.advance();
            let rhs = self.assign();
            node = Expr::Comma(Box::new(node), Box::new(rhs));
        }

        node
    }

    // assign = ternary ("=" assign | "+=" assign | "-=" assign | "*=" assign | "/=" assign | "%=" assign)?
    fn assign(&mut self) -> Expr {
        let node = self.ternary();

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

    // ternary = logical_or ("?" expr ":" ternary)?
    fn ternary(&mut self) -> Expr {
        let node = self.logical_or();

        if self.current().kind == TokenKind::Question {
            self.advance();
            let then_expr = self.expr();
            self.expect(TokenKind::Colon);
            let else_expr = self.ternary();
            return Expr::Ternary {
                cond: Box::new(node),
                then_expr: Box::new(then_expr),
                else_expr: Box::new(else_expr),
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

    // logical_and = bitwise_or ("&&" bitwise_or)*
    fn logical_and(&mut self) -> Expr {
        let mut node = self.bitwise_or();

        while self.current().kind == TokenKind::AmpAmp {
            self.advance();
            let rhs = self.bitwise_or();
            node = Expr::LogicalAnd(Box::new(node), Box::new(rhs));
        }

        node
    }

    // bitwise_or = bitwise_xor ("|" bitwise_xor)*
    fn bitwise_or(&mut self) -> Expr {
        let mut node = self.bitwise_xor();

        while self.current().kind == TokenKind::Pipe {
            self.advance();
            let rhs = self.bitwise_xor();
            node = Expr::BinOp {
                op: BinOp::BitOr,
                lhs: Box::new(node),
                rhs: Box::new(rhs),
            };
        }

        node
    }

    // bitwise_xor = bitwise_and ("^" bitwise_and)*
    fn bitwise_xor(&mut self) -> Expr {
        let mut node = self.bitwise_and();

        while self.current().kind == TokenKind::Caret {
            self.advance();
            let rhs = self.bitwise_and();
            node = Expr::BinOp {
                op: BinOp::BitXor,
                lhs: Box::new(node),
                rhs: Box::new(rhs),
            };
        }

        node
    }

    // bitwise_and = equality ("&" equality)*
    fn bitwise_and(&mut self) -> Expr {
        let mut node = self.equality();

        while self.current().kind == TokenKind::Amp {
            self.advance();
            let rhs = self.equality();
            node = Expr::BinOp {
                op: BinOp::BitAnd,
                lhs: Box::new(node),
                rhs: Box::new(rhs),
            };
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

    // relational = shift ("<" shift | "<=" shift | ">" shift | ">=" shift)*
    fn relational(&mut self) -> Expr {
        let mut node = self.shift();

        loop {
            match self.current().kind {
                TokenKind::Lt => {
                    self.advance();
                    let rhs = self.shift();
                    node = Expr::BinOp {
                        op: BinOp::Lt,
                        lhs: Box::new(node),
                        rhs: Box::new(rhs),
                    };
                }
                TokenKind::Le => {
                    self.advance();
                    let rhs = self.shift();
                    node = Expr::BinOp {
                        op: BinOp::Le,
                        lhs: Box::new(node),
                        rhs: Box::new(rhs),
                    };
                }
                TokenKind::Gt => {
                    self.advance();
                    let rhs = self.shift();
                    node = Expr::BinOp {
                        op: BinOp::Gt,
                        lhs: Box::new(node),
                        rhs: Box::new(rhs),
                    };
                }
                TokenKind::Ge => {
                    self.advance();
                    let rhs = self.shift();
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

    // shift = add ("<<" add | ">>" add)*
    fn shift(&mut self) -> Expr {
        let mut node = self.add();

        loop {
            match self.current().kind {
                TokenKind::LShift => {
                    self.advance();
                    let rhs = self.add();
                    node = Expr::BinOp {
                        op: BinOp::Shl,
                        lhs: Box::new(node),
                        rhs: Box::new(rhs),
                    };
                }
                TokenKind::RShift => {
                    self.advance();
                    let rhs = self.add();
                    node = Expr::BinOp {
                        op: BinOp::Shr,
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

    // unary = ("+" | "-" | "&" | "*") unary | "++" unary | "--" unary | postfix
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
            TokenKind::Amp => {
                self.advance();
                let operand = self.unary();
                Expr::Addr(Box::new(operand))
            }
            TokenKind::Star => {
                self.advance();
                let operand = self.unary();
                Expr::Deref(Box::new(operand))
            }
            TokenKind::Bang => {
                self.advance();
                let operand = self.unary();
                Expr::UnaryOp {
                    op: UnaryOp::LogicalNot,
                    operand: Box::new(operand),
                }
            }
            TokenKind::Tilde => {
                self.advance();
                let operand = self.unary();
                Expr::UnaryOp {
                    op: UnaryOp::BitNot,
                    operand: Box::new(operand),
                }
            }
            TokenKind::Sizeof => {
                self.advance();
                self.expect(TokenKind::LParen);
                let ty = self.parse_type();
                self.expect(TokenKind::RParen);
                return Expr::SizeofType(ty);
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
            _ => {
                // Cast expression: "(" type ")" unary
                if self.current().kind == TokenKind::LParen
                    && self.pos + 1 < self.tokens.len()
                    && Self::is_type_keyword(&self.tokens[self.pos + 1].kind)
                {
                    self.advance(); // consume "("
                    let ty = self.parse_type();
                    self.expect(TokenKind::RParen);
                    let operand = self.unary();
                    return Expr::Cast {
                        ty,
                        expr: Box::new(operand),
                    };
                }
                self.postfix()
            }
        }
    }

    // postfix = primary ("[" expr "]" | "++" | "--")*
    fn postfix(&mut self) -> Expr {
        let mut node = self.primary();

        loop {
            match self.current().kind {
                TokenKind::LBracket => {
                    self.advance();
                    let index = self.expr();
                    self.expect(TokenKind::RBracket);
                    // a[i] is *(a + i)
                    node = Expr::Deref(Box::new(Expr::BinOp {
                        op: BinOp::Add,
                        lhs: Box::new(node),
                        rhs: Box::new(index),
                    }));
                }
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
                // Function call: ident "(" args ")"
                if self.current().kind == TokenKind::LParen {
                    self.advance();
                    let mut args = Vec::new();
                    if self.current().kind != TokenKind::RParen {
                        args.push(self.assign());
                        while self.current().kind == TokenKind::Comma {
                            self.advance();
                            args.push(self.assign());
                        }
                    }
                    self.expect(TokenKind::RParen);
                    return Expr::FuncCall { name, args };
                }
                let resolved = self.resolve_var(&name);
                Expr::Var(resolved)
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

    fn enter_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn leave_scope(&mut self) {
        self.scopes.pop();
    }

    fn declare_var(&mut self, name: &str, ty: Type) -> String {
        // Generate a unique internal name if the variable already exists
        let unique = if self.locals.iter().any(|(_, n)| n == name) {
            self.unique_counter += 1;
            format!("{}.{}", name, self.unique_counter)
        } else {
            name.to_string()
        };
        self.locals.push((ty, unique.clone()));
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_string(), unique.clone());
        }
        unique
    }

    fn resolve_var(&self, name: &str) -> String {
        // Search from innermost scope to outermost
        for scope in self.scopes.iter().rev() {
            if let Some(unique) = scope.get(name) {
                return unique.clone();
            }
        }
        // Fallback: return original name (may be a global variable)
        name.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;

    fn parse_program(input: &str) -> Program {
        let reporter = crate::error::ErrorReporter::new("test", input);
        let mut lexer = Lexer::new(input, &reporter);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens, &reporter);
        parser.parse()
    }

    #[test]
    fn test_return_number() {
        let prog = parse_program("int main() { return 42; }");
        assert_eq!(prog.functions.len(), 1);
        assert_eq!(prog.functions[0].name, "main");
        assert_eq!(prog.functions[0].return_ty, Type::int_type());
        assert_eq!(prog.functions[0].body.len(), 1);
        assert_eq!(prog.functions[0].body[0], Stmt::Return(Some(Expr::Num(42))));
    }

    #[test]
    fn test_expr_stmt() {
        let prog = parse_program("int main() { 1; 2; return 3; }");
        assert_eq!(prog.functions[0].body.len(), 3);
        assert_eq!(prog.functions[0].body[0], Stmt::ExprStmt(Expr::Num(1)));
        assert_eq!(prog.functions[0].body[1], Stmt::ExprStmt(Expr::Num(2)));
        assert_eq!(prog.functions[0].body[2], Stmt::Return(Some(Expr::Num(3))));
    }

    #[test]
    fn test_return_add() {
        let prog = parse_program("int main() { return 1 + 2; }");
        assert_eq!(prog.functions[0].body.len(), 1);
        match &prog.functions[0].body[0] {
            Stmt::Return(Some(Expr::BinOp { op: BinOp::Add, .. })) => {}
            _ => panic!("expected return with add"),
        }
    }

    #[test]
    fn test_var_decl() {
        let prog = parse_program("int main() { int a; a = 3; return a; }");
        assert_eq!(prog.functions[0].body.len(), 3);
        assert_eq!(prog.functions[0].body[0], Stmt::VarDecl { name: "a".to_string(), ty: Type::int_type(), init: None });
    }

    #[test]
    fn test_var_with_init() {
        let prog = parse_program("int main() { int a = 5; return a; }");
        assert_eq!(prog.functions[0].body.len(), 2);
        assert_eq!(
            prog.functions[0].body[0],
            Stmt::VarDecl { name: "a".to_string(), ty: Type::int_type(), init: Some(Expr::Num(5)) }
        );
    }

    #[test]
    fn test_multiple_functions() {
        let prog = parse_program("int ret3() { return 3; } int main() { return ret3(); }");
        assert_eq!(prog.functions.len(), 2);
        assert_eq!(prog.functions[0].name, "ret3");
        assert_eq!(prog.functions[1].name, "main");
        match &prog.functions[1].body[0] {
            Stmt::Return(Some(Expr::FuncCall { name, args })) => {
                assert_eq!(name, "ret3");
                assert_eq!(args.len(), 0);
            }
            _ => panic!("expected return with func call"),
        }
    }

    #[test]
    fn test_global_var() {
        let prog = parse_program("int g; int main() { g = 5; return g; }");
        assert_eq!(prog.globals, vec![(Type::int_type(), "g".to_string())]);
        assert_eq!(prog.functions.len(), 1);
    }

    #[test]
    fn test_void_function() {
        let prog = parse_program("void noop() {} int main() { return 0; }");
        assert_eq!(prog.functions[0].return_ty, Type::void());
        assert_eq!(prog.functions[1].return_ty, Type::int_type());
    }

    #[test]
    fn test_function_params_typed() {
        let prog = parse_program("int add(int a, int b) { return a + b; }");
        assert_eq!(prog.functions[0].params.len(), 2);
        assert_eq!(prog.functions[0].params[0], (Type::int_type(), "a".to_string()));
        assert_eq!(prog.functions[0].params[1], (Type::int_type(), "b".to_string()));
    }
}
