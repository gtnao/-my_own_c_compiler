use std::collections::HashMap;

use crate::ast::{BinOp, Expr, Function, Program, Stmt, UnaryOp};
use crate::error::ErrorReporter;
use crate::token::{Token, TokenKind};
use crate::types::{StructMember, Type};

pub struct Parser<'a> {
    tokens: Vec<Token>,
    pos: usize,
    reporter: &'a ErrorReporter,
    locals: Vec<(Type, String)>,
    scopes: Vec<HashMap<String, String>>,
    unique_counter: usize,
    globals: Vec<(Type, String)>,
    struct_tags: HashMap<String, Type>,
    enum_values: HashMap<String, i64>,
    typedefs: HashMap<String, Type>,
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
            struct_tags: HashMap::new(),
            enum_values: HashMap::new(),
            typedefs: HashMap::new(),
        }
    }

    // program = (typedef | function | prototype | global_var)*
    pub fn parse(&mut self) -> Program {
        let mut functions = Vec::new();
        while self.current().kind != TokenKind::Eof {
            // Handle top-level typedef
            if self.current().kind == TokenKind::Typedef {
                self.advance();
                let ty = self.parse_type();
                let name = match &self.current().kind {
                    TokenKind::Ident(s) => s.clone(),
                    _ => {
                        self.reporter.error_at(
                            self.current().pos,
                            "expected typedef name",
                        );
                    }
                };
                self.advance();
                self.expect(TokenKind::Semicolon);
                self.typedefs.insert(name, ty);
                continue;
            }
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
        matches!(kind, TokenKind::Int | TokenKind::Char | TokenKind::Short | TokenKind::Long | TokenKind::Void | TokenKind::Unsigned | TokenKind::Bool | TokenKind::Struct | TokenKind::Union | TokenKind::Enum)
    }

    fn is_type_start(&self, kind: &TokenKind) -> bool {
        if Self::is_type_keyword(kind) {
            return true;
        }
        if let TokenKind::Ident(name) = kind {
            return self.typedefs.contains_key(name);
        }
        false
    }

    fn is_function(&self) -> bool {
        // type ident "(" → function/prototype
        // Handles multi-token types like "unsigned int"
        if !self.is_type_start(&self.tokens[self.pos].kind) {
            return false;
        }
        let mut i = self.pos;
        // Skip type keywords (including typedef names)
        while self.is_type_start(&self.tokens[i].kind) {
            // For "struct"/"union", skip optional tag name and body
            if self.tokens[i].kind == TokenKind::Struct || self.tokens[i].kind == TokenKind::Union || self.tokens[i].kind == TokenKind::Enum {
                i += 1;
                // Skip tag name if present
                if let TokenKind::Ident(_) = &self.tokens[i].kind {
                    i += 1;
                }
                // Skip struct body { ... } if present
                if self.tokens[i].kind == TokenKind::LBrace {
                    i += 1;
                    let mut depth = 1;
                    while depth > 0 {
                        if self.tokens[i].kind == TokenKind::LBrace {
                            depth += 1;
                        } else if self.tokens[i].kind == TokenKind::RBrace {
                            depth -= 1;
                        }
                        i += 1;
                    }
                }
            } else {
                i += 1;
            }
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
        // type ident ("[" num "]")* ";"
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

        // Array dimensions
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
            for &len in dims.iter().rev() {
                ty = Type::array_of(ty, len);
            }
            ty
        };

        self.expect(TokenKind::Semicolon);
        self.globals.push((ty, name));
    }

    /// Parse a type specifier and return the corresponding Type.
    /// Parse struct or union type (after "struct"/"union" keyword is consumed).
    fn parse_struct_or_union(&mut self, is_union: bool) -> Type {
        let kind_name = if is_union { "union" } else { "struct" };
        // Check for tag name
        let tag_name = if let TokenKind::Ident(s) = &self.current().kind {
            let name = s.clone();
            self.advance();
            Some(name)
        } else {
            None
        };

        // Parse body if present
        if self.current().kind == TokenKind::LBrace {
            self.advance();
            let mut members = Vec::new();
            let mut offset = 0;
            while self.current().kind != TokenKind::RBrace {
                let mem_ty = self.parse_type();
                let mem_name = match &self.current().kind {
                    TokenKind::Ident(s) => s.clone(),
                    _ => {
                        self.reporter.error_at(
                            self.current().pos,
                            "expected member name",
                        );
                    }
                };
                self.advance();
                self.expect(TokenKind::Semicolon);
                if is_union {
                    // Union: all members at offset 0
                    members.push(StructMember {
                        name: mem_name,
                        ty: mem_ty.clone(),
                        offset: 0,
                    });
                } else {
                    // Struct: align offset to member alignment
                    let align = mem_ty.align();
                    offset = (offset + align - 1) & !(align - 1);
                    members.push(StructMember {
                        name: mem_name,
                        ty: mem_ty.clone(),
                        offset,
                    });
                    offset += mem_ty.size();
                }
            }
            self.expect(TokenKind::RBrace);
            let ty = Type {
                kind: crate::types::TypeKind::Struct(members),
                is_unsigned: false,
            };
            // Register tag if present
            if let Some(ref tag) = tag_name {
                self.struct_tags.insert(tag.clone(), ty.clone());
            }
            ty
        } else if let Some(ref tag) = tag_name {
            // Look up tag
            match self.struct_tags.get(tag) {
                Some(ty) => ty.clone(),
                None => {
                    self.reporter.error_at(
                        self.current().pos,
                        &format!("unknown {} tag '{}'", kind_name, tag),
                    );
                }
            }
        } else {
            self.reporter.error_at(
                self.current().pos,
                &format!("expected {} tag or body", kind_name),
            );
        }
    }

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
            TokenKind::Struct => {
                self.advance();
                self.parse_struct_or_union(false)
            }
            TokenKind::Union => {
                self.advance();
                self.parse_struct_or_union(true)
            }
            TokenKind::Enum => {
                self.advance();
                // Skip optional tag name
                if let TokenKind::Ident(_) = &self.current().kind {
                    self.advance();
                }
                // Parse enum body if present
                if self.current().kind == TokenKind::LBrace {
                    self.advance();
                    let mut val: i64 = 0;
                    while self.current().kind != TokenKind::RBrace {
                        let name = match &self.current().kind {
                            TokenKind::Ident(s) => s.clone(),
                            _ => {
                                self.reporter.error_at(
                                    self.current().pos,
                                    "expected enum constant name",
                                );
                            }
                        };
                        self.advance();
                        // Optional explicit value: = num
                        if self.current().kind == TokenKind::Eq {
                            self.advance();
                            if let TokenKind::Num(n) = self.current().kind {
                                val = n;
                                self.advance();
                            }
                        }
                        self.enum_values.insert(name, val);
                        val += 1;
                        if self.current().kind == TokenKind::Comma {
                            self.advance();
                        }
                    }
                    self.expect(TokenKind::RBrace);
                }
                // enum type is int
                Type::int_type()
            }
            _ => {
                if is_unsigned {
                    // bare "unsigned" = "unsigned int"
                    Type::uint()
                } else if let TokenKind::Ident(name) = &self.current().kind {
                    if let Some(ty) = self.typedefs.get(name).cloned() {
                        self.advance();
                        ty
                    } else {
                        self.reporter.error_at(
                            self.current().pos,
                            &format!("expected type, but got {:?}", self.current().kind),
                        );
                    }
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
            loop {
                let mut param_ty = self.parse_type();
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
                // Array parameter: int a[] → int *a
                if self.current().kind == TokenKind::LBracket {
                    self.advance();
                    if self.current().kind == TokenKind::Num(0) || self.current().kind != TokenKind::RBracket {
                        // Skip optional size
                        self.advance();
                    }
                    self.expect(TokenKind::RBracket);
                    param_ty = Type::ptr_to(param_ty);
                }
                let unique = self.declare_var(&param_name, param_ty.clone());
                params.push((param_ty, unique));

                if self.current().kind != TokenKind::Comma {
                    break;
                }
                self.advance();
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
            TokenKind::Typedef => {
                self.advance();
                let ty = self.parse_type();
                let name = match &self.current().kind {
                    TokenKind::Ident(s) => s.clone(),
                    _ => {
                        self.reporter.error_at(
                            self.current().pos,
                            "expected typedef name",
                        );
                    }
                };
                self.advance();
                self.expect(TokenKind::Semicolon);
                self.typedefs.insert(name, ty);
                Stmt::Block(vec![])
            }
            TokenKind::Int | TokenKind::Char | TokenKind::Short | TokenKind::Long | TokenKind::Unsigned | TokenKind::Bool | TokenKind::Struct | TokenKind::Union | TokenKind::Enum => {
                self.var_decl()
            }
            _ => {
                // Check for typedef name as type
                if let TokenKind::Ident(name) = &self.current().kind {
                    if self.typedefs.contains_key(name) {
                        return self.var_decl();
                    }
                }
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
    //         | "struct" tag "{" ... "}" ";"  (tag definition only)
    fn var_decl(&mut self) -> Stmt {
        let ty = self.parse_type();
        // Allow struct tag definition without variable declaration
        if self.current().kind == TokenKind::Semicolon {
            self.advance();
            return Stmt::Block(vec![]);
        }
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

        // Array declaration: ident ("[" num? "]")*
        let (ty, has_empty_bracket) = {
            let mut dims = Vec::new();
            let mut has_empty = false;
            while self.current().kind == TokenKind::LBracket {
                self.advance();
                if self.current().kind == TokenKind::RBracket {
                    // Empty brackets: int a[] = {...}
                    has_empty = true;
                    self.advance();
                    dims.push(0); // placeholder, will be filled from initializer
                } else {
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
            }
            let mut ty = ty;
            // Build type from innermost to outermost:
            // int a[2][3] → Array(Array(Int, 3), 2)
            for &len in dims.iter().rev() {
                ty = Type::array_of(ty, len);
            }
            (ty, has_empty)
        };

        // Handle initializer
        if self.current().kind == TokenKind::Eq {
            self.advance();
            if self.current().kind == TokenKind::LBrace {
                // Brace initializer: = { expr, expr, ... }
                // Supports designated initializers: .member = val, [idx] = val
                self.advance();

                if let crate::types::TypeKind::Struct(ref members) = ty.kind {
                    // Struct initializer with optional designators
                    let members_list = members.clone();
                    let unique = self.declare_var(&name, ty.clone());
                    let mut stmts = vec![Stmt::VarDecl { name: unique.clone(), ty: ty.clone(), init: None }];
                    let mut seq_idx = 0;

                    while self.current().kind != TokenKind::RBrace {
                        if self.current().kind == TokenKind::Dot {
                            // Designated: .member = val
                            self.advance();
                            let mem_name = match &self.current().kind {
                                TokenKind::Ident(s) => s.clone(),
                                _ => {
                                    self.reporter.error_at(
                                        self.current().pos,
                                        "expected member name after '.'",
                                    );
                                }
                            };
                            self.advance();
                            self.expect(TokenKind::Eq);
                            let val = self.assign();
                            stmts.push(Stmt::ExprStmt(Expr::Assign {
                                lhs: Box::new(Expr::Member(
                                    Box::new(Expr::Var(unique.clone())),
                                    mem_name.clone(),
                                )),
                                rhs: Box::new(val),
                            }));
                            // Update seq_idx to position after this member
                            if let Some(pos) = members_list.iter().position(|m| m.name == mem_name) {
                                seq_idx = pos + 1;
                            }
                        } else {
                            // Sequential
                            let val = self.assign();
                            if seq_idx < members_list.len() {
                                let mem_name = members_list[seq_idx].name.clone();
                                stmts.push(Stmt::ExprStmt(Expr::Assign {
                                    lhs: Box::new(Expr::Member(
                                        Box::new(Expr::Var(unique.clone())),
                                        mem_name,
                                    )),
                                    rhs: Box::new(val),
                                }));
                            }
                            seq_idx += 1;
                        }
                        if self.current().kind == TokenKind::Comma {
                            self.advance();
                        }
                    }
                    self.expect(TokenKind::RBrace);
                    self.expect(TokenKind::Semicolon);
                    return Stmt::Block(stmts);
                } else {
                    // Array initializer with optional designators
                    let mut indexed_exprs: Vec<(usize, Expr)> = Vec::new();
                    let mut seq_idx: usize = 0;
                    let mut max_idx: usize = 0;

                    while self.current().kind != TokenKind::RBrace {
                        if self.current().kind == TokenKind::LBracket {
                            // Designated: [idx] = val
                            self.advance();
                            let idx = match &self.current().kind {
                                TokenKind::Num(n) => *n as usize,
                                _ => {
                                    self.reporter.error_at(
                                        self.current().pos,
                                        "expected array index",
                                    );
                                }
                            };
                            self.advance();
                            self.expect(TokenKind::RBracket);
                            self.expect(TokenKind::Eq);
                            let val = self.assign();
                            indexed_exprs.push((idx, val));
                            if idx + 1 > max_idx { max_idx = idx + 1; }
                            seq_idx = idx + 1;
                        } else {
                            // Sequential
                            let val = self.assign();
                            indexed_exprs.push((seq_idx, val));
                            seq_idx += 1;
                            if seq_idx > max_idx { max_idx = seq_idx; }
                        }
                        if self.current().kind == TokenKind::Comma {
                            self.advance();
                        }
                    }
                    self.expect(TokenKind::RBrace);
                    self.expect(TokenKind::Semicolon);

                    // Determine array type (fill in size for empty brackets)
                    let ty = if has_empty_bracket {
                        let base = ty.base_type().unwrap().clone();
                        Type::array_of(base, max_idx)
                    } else {
                        ty
                    };

                    let unique = self.declare_var(&name, ty.clone());
                    let mut stmts = vec![Stmt::VarDecl { name: unique.clone(), ty: ty.clone(), init: None }];

                    for (idx, init_expr) in indexed_exprs {
                        stmts.push(Stmt::ExprStmt(Expr::Assign {
                            lhs: Box::new(Expr::Deref(Box::new(Expr::BinOp {
                                op: BinOp::Add,
                                lhs: Box::new(Expr::Var(unique.clone())),
                                rhs: Box::new(Expr::Num(idx as i64)),
                            }))),
                            rhs: Box::new(init_expr),
                        }));
                    }
                    return Stmt::Block(stmts);
                }
            } else if matches!(self.current().kind, TokenKind::Str(_)) && (has_empty_bracket || matches!(ty.kind, crate::types::TypeKind::Array(_, _))) {
                // String initializer for char array: char s[] = "hello";
                let s = match &self.current().kind {
                    TokenKind::Str(s) => s.clone(),
                    _ => unreachable!(),
                };
                self.advance();
                // Concatenate adjacent strings
                let mut bytes = s;
                while let TokenKind::Str(ref next) = self.current().kind {
                    bytes.extend_from_slice(next);
                    self.advance();
                }
                self.expect(TokenKind::Semicolon);

                // Determine type (include null terminator)
                let array_len = bytes.len() + 1; // +1 for null terminator
                let ty = if has_empty_bracket {
                    Type::array_of(Type::char_type(), array_len)
                } else {
                    ty
                };

                let unique = self.declare_var(&name, ty.clone());
                let mut stmts = vec![Stmt::VarDecl { name: unique.clone(), ty, init: None }];

                // Generate assignment for each byte + null terminator
                for (i, &b) in bytes.iter().enumerate() {
                    stmts.push(Stmt::ExprStmt(Expr::Assign {
                        lhs: Box::new(Expr::Deref(Box::new(Expr::BinOp {
                            op: BinOp::Add,
                            lhs: Box::new(Expr::Var(unique.clone())),
                            rhs: Box::new(Expr::Num(i as i64)),
                        }))),
                        rhs: Box::new(Expr::Num(b as i64)),
                    }));
                }
                // Null terminator
                stmts.push(Stmt::ExprStmt(Expr::Assign {
                    lhs: Box::new(Expr::Deref(Box::new(Expr::BinOp {
                        op: BinOp::Add,
                        lhs: Box::new(Expr::Var(unique.clone())),
                        rhs: Box::new(Expr::Num(bytes.len() as i64)),
                    }))),
                    rhs: Box::new(Expr::Num(0)),
                }));
                return Stmt::Block(stmts);
            } else {
                // Normal initializer
                let unique = self.declare_var(&name, ty.clone());
                let init = Some(self.expr());
                self.expect(TokenKind::Semicolon);
                return Stmt::VarDecl { name: unique, ty, init };
            }
        }

        let unique = self.declare_var(&name, ty.clone());
        self.expect(TokenKind::Semicolon);
        Stmt::VarDecl { name: unique, ty, init: None }
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
                // sizeof(type) vs sizeof expr
                if self.current().kind == TokenKind::LParen
                    && self.pos + 1 < self.tokens.len()
                    && Self::is_type_keyword(&self.tokens[self.pos + 1].kind)
                {
                    self.advance(); // consume "("
                    let ty = self.parse_type();
                    self.expect(TokenKind::RParen);
                    return Expr::SizeofType(ty);
                }
                let operand = self.unary();
                return Expr::SizeofExpr(Box::new(operand));
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
                // Cast expression or compound literal: "(" type ")" ...
                if self.current().kind == TokenKind::LParen
                    && self.pos + 1 < self.tokens.len()
                    && Self::is_type_keyword(&self.tokens[self.pos + 1].kind)
                {
                    self.advance(); // consume "("
                    let mut ty = self.parse_type();

                    // Parse optional array dimensions: (int[3]) or (int[])
                    let mut has_empty_bracket = false;
                    while self.current().kind == TokenKind::LBracket {
                        self.advance();
                        if self.current().kind == TokenKind::RBracket {
                            has_empty_bracket = true;
                            self.advance();
                            ty = Type::array_of(ty, 0); // placeholder
                        } else {
                            let n = match &self.current().kind {
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
                            ty = Type::array_of(ty, n);
                        }
                    }

                    self.expect(TokenKind::RParen);

                    if self.current().kind == TokenKind::LBrace {
                        // Compound literal: (type){initializers}
                        return self.parse_compound_literal(ty, has_empty_bracket);
                    }

                    // Regular cast
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
                TokenKind::Dot => {
                    self.advance();
                    let member_name = match &self.current().kind {
                        TokenKind::Ident(s) => s.clone(),
                        _ => {
                            self.reporter.error_at(
                                self.current().pos,
                                "expected member name after '.'",
                            );
                        }
                    };
                    self.advance();
                    node = Expr::Member(Box::new(node), member_name);
                }
                TokenKind::Arrow => {
                    // p->member is (*p).member
                    self.advance();
                    let member_name = match &self.current().kind {
                        TokenKind::Ident(s) => s.clone(),
                        _ => {
                            self.reporter.error_at(
                                self.current().pos,
                                "expected member name after '->'",
                            );
                        }
                    };
                    self.advance();
                    node = Expr::Member(Box::new(Expr::Deref(Box::new(node))), member_name);
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
            TokenKind::Str(s) => {
                self.advance();
                // String concatenation: "hello" " " "world"
                let mut bytes = s;
                while let TokenKind::Str(next) = &self.current().kind {
                    bytes.extend_from_slice(next);
                    self.advance();
                }
                Expr::StrLit(bytes)
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
                // Check for enum constant
                if let Some(&val) = self.enum_values.get(&name) {
                    return Expr::Num(val);
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

    /// Parse compound literal: (type){initializers}
    /// Desugars to an anonymous variable initialized via comma expressions.
    fn parse_compound_literal(&mut self, ty: Type, has_empty_bracket: bool) -> Expr {
        self.advance(); // consume "{"

        // Generate anonymous variable
        self.unique_counter += 1;
        let anon_name = format!("__compound_{}", self.unique_counter);

        if let crate::types::TypeKind::Struct(ref members) = ty.kind {
            // Struct compound literal
            let members_list = members.clone();
            let unique = self.declare_var(&anon_name, ty.clone());
            let mut assigns: Vec<Expr> = Vec::new();
            let mut seq_idx = 0;

            while self.current().kind != TokenKind::RBrace {
                if self.current().kind == TokenKind::Dot {
                    self.advance();
                    let mem_name = match &self.current().kind {
                        TokenKind::Ident(s) => s.clone(),
                        _ => {
                            self.reporter.error_at(
                                self.current().pos,
                                "expected member name",
                            );
                        }
                    };
                    self.advance();
                    self.expect(TokenKind::Eq);
                    let val = self.assign();
                    assigns.push(Expr::Assign {
                        lhs: Box::new(Expr::Member(
                            Box::new(Expr::Var(unique.clone())),
                            mem_name.clone(),
                        )),
                        rhs: Box::new(val),
                    });
                    if let Some(pos) = members_list.iter().position(|m| m.name == mem_name) {
                        seq_idx = pos + 1;
                    }
                } else {
                    let val = self.assign();
                    if seq_idx < members_list.len() {
                        let mem_name = members_list[seq_idx].name.clone();
                        assigns.push(Expr::Assign {
                            lhs: Box::new(Expr::Member(
                                Box::new(Expr::Var(unique.clone())),
                                mem_name,
                            )),
                            rhs: Box::new(val),
                        });
                    }
                    seq_idx += 1;
                }
                if self.current().kind == TokenKind::Comma {
                    self.advance();
                }
            }
            self.expect(TokenKind::RBrace);

            // Chain assignments via comma operator, ending with variable reference
            self.build_comma_chain(assigns, unique)
        } else {
            // Array/scalar compound literal
            let mut indexed_exprs: Vec<(usize, Expr)> = Vec::new();
            let mut seq_idx: usize = 0;
            let mut max_idx: usize = 0;

            while self.current().kind != TokenKind::RBrace {
                if self.current().kind == TokenKind::LBracket {
                    self.advance();
                    let idx = match &self.current().kind {
                        TokenKind::Num(n) => *n as usize,
                        _ => {
                            self.reporter.error_at(
                                self.current().pos,
                                "expected array index",
                            );
                        }
                    };
                    self.advance();
                    self.expect(TokenKind::RBracket);
                    self.expect(TokenKind::Eq);
                    let val = self.assign();
                    indexed_exprs.push((idx, val));
                    if idx + 1 > max_idx { max_idx = idx + 1; }
                    seq_idx = idx + 1;
                } else {
                    let val = self.assign();
                    indexed_exprs.push((seq_idx, val));
                    seq_idx += 1;
                    if seq_idx > max_idx { max_idx = seq_idx; }
                }
                if self.current().kind == TokenKind::Comma {
                    self.advance();
                }
            }
            self.expect(TokenKind::RBrace);

            // Determine array type
            let ty = if has_empty_bracket {
                let base = ty.base_type().unwrap().clone();
                Type::array_of(base, max_idx)
            } else if matches!(ty.kind, crate::types::TypeKind::Array(_, _)) {
                ty
            } else {
                // Scalar compound literal: (int){5}
                ty
            };

            let unique = self.declare_var(&anon_name, ty);

            let assigns: Vec<Expr> = if indexed_exprs.len() == 1 && !matches!(self.local_types_last(&unique), Some(ty) if matches!(ty.kind, crate::types::TypeKind::Array(_, _))) {
                // Scalar: just assign directly
                vec![Expr::Assign {
                    lhs: Box::new(Expr::Var(unique.clone())),
                    rhs: Box::new(indexed_exprs.into_iter().next().unwrap().1),
                }]
            } else {
                // Array: element-by-element assignment
                indexed_exprs.into_iter().map(|(idx, val)| {
                    Expr::Assign {
                        lhs: Box::new(Expr::Deref(Box::new(Expr::BinOp {
                            op: BinOp::Add,
                            lhs: Box::new(Expr::Var(unique.clone())),
                            rhs: Box::new(Expr::Num(idx as i64)),
                        }))),
                        rhs: Box::new(val),
                    }
                }).collect()
            };

            self.build_comma_chain(assigns, unique)
        }
    }

    /// Build a comma chain: (assign1, (assign2, (..., var)))
    fn build_comma_chain(&self, assigns: Vec<Expr>, var_name: String) -> Expr {
        let var_ref = Expr::Var(var_name);
        if assigns.is_empty() {
            return var_ref;
        }
        let mut node = var_ref;
        for assign in assigns.into_iter().rev() {
            node = Expr::Comma(Box::new(assign), Box::new(node));
        }
        node
    }

    /// Look up the type of the last declared variable with this name.
    fn local_types_last(&self, name: &str) -> Option<&Type> {
        self.locals.iter().rev().find(|(_, n)| n == name).map(|(ty, _)| ty)
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
