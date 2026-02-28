use std::collections::HashMap;

use crate::ast::{BinOp, Expr, Function, Program, Stmt, UnaryOp};
use crate::error::ErrorReporter;
use crate::token::{Token, TokenKind};
use crate::types::{StructMember, Type, TypeKind};

pub struct Parser<'a> {
    tokens: Vec<Token>,
    pos: usize,
    reporter: &'a ErrorReporter,
    locals: Vec<(Type, String)>,
    scopes: Vec<HashMap<String, String>>,
    unique_counter: usize,
    globals: Vec<(Type, String, Option<Vec<u8>>)>,
    struct_tags: HashMap<String, Type>,
    enum_values: HashMap<String, i64>,
    typedefs: HashMap<String, Type>,
    /// Maps typedef name to struct tag name, for resolving forward-declared structs
    typedef_struct_tags: HashMap<String, String>,
    /// Last struct/union tag name parsed by parse_struct_or_union
    last_struct_tag: Option<String>,
    /// Tags of forward-declared (empty) structs, for tracking which tag an empty struct came from
    forward_declared_tags: std::collections::HashSet<String>,
    extern_names: std::collections::HashSet<String>,
}

impl<'a> Parser<'a> {
    pub fn new(tokens: Vec<Token>, reporter: &'a ErrorReporter) -> Self {
        let mut typedefs = HashMap::new();
        // Register built-in fixed-width integer type aliases
        typedefs.insert("int8_t".to_string(), Type::char_type());
        typedefs.insert("int16_t".to_string(), Type::short_type());
        typedefs.insert("int32_t".to_string(), Type::int_type());
        typedefs.insert("int64_t".to_string(), Type::long_type());
        typedefs.insert("uint8_t".to_string(), Type::uchar());
        typedefs.insert("uint16_t".to_string(), Type::ushort());
        typedefs.insert("uint32_t".to_string(), Type::uint());
        typedefs.insert("uint64_t".to_string(), Type::ulong());
        typedefs.insert("size_t".to_string(), Type::ulong());
        typedefs.insert("ssize_t".to_string(), Type::long_type());
        typedefs.insert("intptr_t".to_string(), Type::long_type());
        typedefs.insert("uintptr_t".to_string(), Type::ulong());
        typedefs.insert("ptrdiff_t".to_string(), Type::long_type());
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
            typedefs,
            typedef_struct_tags: HashMap::new(),
            last_struct_tag: None,
            forward_declared_tags: std::collections::HashSet::new(),
            extern_names: std::collections::HashSet::new(),
        }
    }

    // program = (typedef | function | prototype | global_var)*
    pub fn parse(&mut self) -> Program {
        let mut functions = Vec::new();
        while self.current().kind != TokenKind::Eof {
            // Skip _Static_assert at top level
            if self.current().kind == TokenKind::StaticAssert {
                self.skip_static_assert();
                continue;
            }
            // Skip top-level 'static' qualifier (treat static functions/vars as normal)
            if self.current().kind == TokenKind::Static {
                self.advance();
            }
            // Handle extern declaration (just skip it — no storage allocated)
            if self.current().kind == TokenKind::Extern {
                self.advance();
                let ty = self.parse_type();
                let name = match &self.current().kind {
                    TokenKind::Ident(s) => s.clone(),
                    _ => {
                        self.reporter.error_at(
                            self.current().pos,
                            "expected name after extern",
                        );
                    }
                };
                self.advance();
                // Skip function prototype: extern int foo(int, int);
                if self.current().kind == TokenKind::LParen {
                    let mut depth = 1;
                    self.advance();
                    while depth > 0 {
                        if self.current().kind == TokenKind::LParen { depth += 1; }
                        if self.current().kind == TokenKind::RParen { depth -= 1; }
                        self.advance();
                    }
                }
                // Skip array dimensions: extern int foo[];
                while self.current().kind == TokenKind::LBracket {
                    self.advance();
                    while self.current().kind != TokenKind::RBracket && self.current().kind != TokenKind::Eof {
                        self.advance();
                    }
                    if self.current().kind == TokenKind::RBracket { self.advance(); }
                }
                self.skip_attribute();
                self.expect(TokenKind::Semicolon);
                // Register type for codegen but mark as extern (no storage)
                self.extern_names.insert(name.clone());
                self.globals.push((ty, name, None));
                continue;
            }
            // Handle top-level typedef
            if self.current().kind == TokenKind::Typedef {
                self.advance();
                let ty = self.parse_type();

                // Check for function pointer typedef: typedef RetType (*Name)(params...)
                if self.current().kind == TokenKind::LParen
                    && self.pos + 1 < self.tokens.len()
                    && self.tokens[self.pos + 1].kind == TokenKind::Star
                {
                    self.advance(); // (
                    self.advance(); // *
                    // Skip qualifiers between * and name
                    while matches!(self.current().kind, TokenKind::Const | TokenKind::Volatile | TokenKind::Restrict) {
                        self.advance();
                    }
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
                    self.expect(TokenKind::RParen);
                    // Skip parameter list: (params...)
                    if self.current().kind == TokenKind::LParen {
                        let mut depth = 1;
                        self.advance();
                        while depth > 0 {
                            match self.current().kind {
                                TokenKind::LParen => depth += 1,
                                TokenKind::RParen => depth -= 1,
                                TokenKind::Eof => break,
                                _ => {}
                            }
                            self.advance();
                        }
                    }
                    self.skip_attribute();
                    self.expect(TokenKind::Semicolon);
                    // Function pointer typedef: store as pointer to void (simplified)
                    if let Some(ref tag) = self.last_struct_tag {
                        self.typedef_struct_tags.insert(name.clone(), tag.clone());
                    }
                    self.typedefs.insert(name, Type::ptr_to(ty));
                    continue;
                }

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
                // Handle array typedef: typedef int Name[N];
                if self.current().kind == TokenKind::LBracket {
                    self.advance();
                    let size = if self.current().kind != TokenKind::RBracket {
                        let s = self.eval_const_expr();
                        s as usize
                    } else {
                        0
                    };
                    self.expect(TokenKind::RBracket);
                    self.skip_attribute();
                    self.expect(TokenKind::Semicolon);
                    let arr_ty = Type { kind: crate::types::TypeKind::Array(Box::new(ty), size), is_unsigned: false };
                    if let Some(ref tag) = self.last_struct_tag {
                        self.typedef_struct_tags.insert(name.clone(), tag.clone());
                    }
                    self.typedefs.insert(name, arr_ty);
                    continue;
                }
                self.skip_attribute();
                self.expect(TokenKind::Semicolon);
                if let Some(ref tag) = self.last_struct_tag {
                    self.typedef_struct_tags.insert(name.clone(), tag.clone());
                }
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
            extern_names: self.extern_names.clone(),
        }
    }

    fn is_type_keyword(kind: &TokenKind) -> bool {
        matches!(kind, TokenKind::Int | TokenKind::Char | TokenKind::Short | TokenKind::Long | TokenKind::Void | TokenKind::Signed | TokenKind::Unsigned | TokenKind::Bool | TokenKind::Struct | TokenKind::Union | TokenKind::Enum | TokenKind::Const | TokenKind::Volatile | TokenKind::Restrict | TokenKind::Alignas | TokenKind::FloatKw | TokenKind::DoubleKw | TokenKind::Attribute | TokenKind::Inline | TokenKind::Noreturn | TokenKind::Register | TokenKind::Extension | TokenKind::Typeof | TokenKind::Auto)
    }

    fn is_type_start(&self, kind: &TokenKind) -> bool {
        if Self::is_type_keyword(kind) {
            return true;
        }
        if let TokenKind::Ident(name) = kind {
            return self.typedefs.contains_key(name) || name == "va_list" || name == "__builtin_va_list"
                || name == "__int128" || name == "__int128_t" || name == "__uint128_t";
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
            } else if self.tokens[i].kind == TokenKind::Alignas || self.tokens[i].kind == TokenKind::Attribute {
                // Skip _Alignas(...) or __attribute__((...))
                i += 1;
                if self.tokens[i].kind == TokenKind::LParen {
                    i += 1;
                    let mut depth = 1;
                    while depth > 0 {
                        if self.tokens[i].kind == TokenKind::LParen { depth += 1; }
                        else if self.tokens[i].kind == TokenKind::RParen { depth -= 1; }
                        i += 1;
                    }
                }
            } else {
                i += 1;
            }
        }
        // Skip pointer stars and qualifiers
        while self.tokens[i].kind == TokenKind::Star {
            i += 1;
            while matches!(self.tokens[i].kind, TokenKind::Const | TokenKind::Volatile) {
                i += 1;
            }
        }
        if let TokenKind::Ident(_) = &self.tokens[i].kind {
            return self.tokens[i + 1].kind == TokenKind::LParen;
        }
        false
    }

    fn global_var(&mut self) {
        // type ident ("[" num "]")* ";"
        let ty = self.parse_type();
        // Handle standalone struct/union/enum definition: "struct Tag { ... };"
        if self.current().kind == TokenKind::Semicolon {
            self.advance();
            return;
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

        // Array dimensions
        let ty = {
            let mut dims = Vec::new();
            while self.current().kind == TokenKind::LBracket {
                self.advance();
                if self.current().kind == TokenKind::RBracket {
                    // Empty brackets: type name[] = {...}
                    self.advance();
                    dims.push(0);
                } else {
                    let len = self.eval_const_expr() as usize;
                    self.expect(TokenKind::RBracket);
                    dims.push(len);
                }
            }
            let mut ty = ty;
            for &len in dims.iter().rev() {
                ty = Type::array_of(ty, len);
            }
            ty
        };

        // Parse optional initializer
        if self.current().kind == TokenKind::Eq {
            self.advance();
            if self.current().kind == TokenKind::LBrace {
                // Brace initializer: = { val, val, ... }
                self.advance();
                let mut vals: Vec<i64> = Vec::new();
                while self.current().kind != TokenKind::RBrace {
                    let val = match &self.current().kind {
                        TokenKind::Num(n) => *n,
                        _ => {
                            self.reporter.error_at(
                                self.current().pos,
                                "expected constant in global initializer",
                            );
                        }
                    };
                    self.advance();
                    vals.push(val);
                    if self.current().kind == TokenKind::Comma {
                        self.advance();
                    }
                }
                self.expect(TokenKind::RBrace);
                self.expect(TokenKind::Semicolon);

                // Determine array size for empty brackets
                let ty = if matches!(ty.kind, crate::types::TypeKind::Array(_, 0)) {
                    let base = ty.base_type().unwrap().clone();
                    Type::array_of(base, vals.len())
                } else {
                    ty
                };

                // Convert values to raw bytes based on element type
                let elem_size = ty.base_type().map(|b| b.size()).unwrap_or(ty.size());
                let mut bytes = Vec::new();
                for val in &vals {
                    for i in 0..elem_size {
                        bytes.push(((val >> (i * 8)) & 0xff) as u8);
                    }
                }
                // Pad remaining space with zeros
                let total_size = ty.size();
                while bytes.len() < total_size {
                    bytes.push(0);
                }

                self.globals.push((ty, name, Some(bytes)));
            } else if let TokenKind::Str(s) = &self.current().kind {
                // String initializer: char g[] = "hello";
                let mut data = s.clone();
                self.advance();
                // Concatenate adjacent strings
                while let TokenKind::Str(ref next) = self.current().kind {
                    data.extend_from_slice(next);
                    self.advance();
                }
                self.expect(TokenKind::Semicolon);

                let array_len = data.len() + 1;
                let ty = if matches!(ty.kind, crate::types::TypeKind::Array(_, 0)) {
                    Type::array_of(Type::char_type(), array_len)
                } else {
                    ty
                };

                data.push(0); // null terminator
                self.globals.push((ty, name, Some(data)));
            } else {
                // Scalar initializer: = num
                let val = match &self.current().kind {
                    TokenKind::Num(n) => *n,
                    _ => {
                        self.reporter.error_at(
                            self.current().pos,
                            "expected constant in global initializer",
                        );
                    }
                };
                self.advance();
                self.expect(TokenKind::Semicolon);

                let elem_size = ty.size();
                let mut bytes = Vec::new();
                for i in 0..elem_size {
                    bytes.push(((val >> (i * 8)) & 0xff) as u8);
                }

                self.globals.push((ty, name, Some(bytes)));
            }
        } else {
            self.expect(TokenKind::Semicolon);
            self.globals.push((ty, name, None));
        }
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
        self.last_struct_tag = tag_name.clone();

        // Parse body if present
        if self.current().kind == TokenKind::LBrace {
            self.advance();
            let mut members = Vec::new();
            let mut offset = 0;
            let mut bit_offset: usize = 0; // current bit offset within the current storage unit
            while self.current().kind != TokenKind::RBrace {
                let mut mem_ty = self.parse_type();

                // Function pointer member: type (*name)(params...)
                let mem_name = if self.current().kind == TokenKind::LParen
                    && self.pos + 1 < self.tokens.len()
                    && self.tokens[self.pos + 1].kind == TokenKind::Star
                {
                    self.advance(); // (
                    self.advance(); // *
                    // Skip qualifiers
                    while matches!(self.current().kind, TokenKind::Const | TokenKind::Volatile | TokenKind::Restrict) {
                        self.advance();
                    }
                    let name = match &self.current().kind {
                        TokenKind::Ident(s) => s.clone(),
                        _ => {
                            self.reporter.error_at(
                                self.current().pos,
                                "expected function pointer member name",
                            );
                        }
                    };
                    self.advance();
                    self.expect(TokenKind::RParen);
                    // Skip parameter list
                    self.expect(TokenKind::LParen);
                    while self.current().kind != TokenKind::RParen {
                        if self.is_type_start(&self.current().kind.clone()) {
                            let _pty = self.parse_type();
                            if let TokenKind::Ident(_) = &self.current().kind {
                                self.advance();
                            }
                        }
                        if self.current().kind == TokenKind::Comma {
                            self.advance();
                        }
                    }
                    self.expect(TokenKind::RParen);
                    // Store as pointer to void (function pointer simplified)
                    mem_ty = Type::ptr_to(Type::void());
                    name
                } else if self.current().kind == TokenKind::Semicolon {
                    // Anonymous struct/union member: no name, inline members
                    if let TypeKind::Struct(inner_members) = &mem_ty.kind {
                        // Flatten inner members into the parent struct
                        for inner in inner_members {
                            let member_offset = if is_union {
                                inner.offset
                            } else {
                                let align = inner.ty.align();
                                offset = (offset + align - 1) & !(align - 1);
                                let o = offset + inner.offset;
                                o
                            };
                            members.push(StructMember {
                                name: inner.name.clone(),
                                ty: inner.ty.clone(),
                                offset: member_offset,
                                bit_width: inner.bit_width,
                                bit_offset: inner.bit_offset,
                            });
                        }
                        if !is_union {
                            offset += mem_ty.size();
                        }
                        self.expect(TokenKind::Semicolon);
                        continue;
                    }
                    // Not a struct/union — treat as error
                    self.reporter.error_at(
                        self.current().pos,
                        "expected member name",
                    );
                } else {
                    match &self.current().kind {
                        TokenKind::Ident(s) => {
                            let s = s.clone();
                            self.advance();
                            s
                        }
                        _ => {
                            self.reporter.error_at(
                                self.current().pos,
                                "expected member name",
                            );
                        }
                    }
                };

                // Check for array member: name "[" num? "]"
                let mem_ty = if self.current().kind == TokenKind::LBracket {
                    self.advance();
                    if self.current().kind == TokenKind::RBracket {
                        // Flexible array member: name[]
                        self.advance();
                        Type::array_of(mem_ty, 0)
                    } else {
                        let len = self.eval_const_expr() as usize;
                        self.expect(TokenKind::RBracket);
                        Type::array_of(mem_ty, len)
                    }
                } else {
                    mem_ty
                };

                // Check for bit-field: "name : width"
                let bit_width = if self.current().kind == TokenKind::Colon {
                    self.advance();
                    let width = match &self.current().kind {
                        TokenKind::Num(n) => *n as usize,
                        _ => {
                            self.reporter.error_at(
                                self.current().pos,
                                "expected bit-field width",
                            );
                        }
                    };
                    self.advance();
                    width
                } else {
                    0
                };

                self.expect(TokenKind::Semicolon);
                if is_union {
                    // Union: all members at offset 0
                    members.push(StructMember {
                        name: mem_name,
                        ty: mem_ty.clone(),
                        offset: 0,
                        bit_width,
                        bit_offset: 0,
                    });
                } else if bit_width > 0 {
                    // Bit-field member
                    let storage_size = mem_ty.size(); // e.g., 4 for int
                    let storage_bits = storage_size * 8;
                    // Align to storage unit boundary if needed
                    let align = mem_ty.align();
                    if bit_offset == 0 {
                        offset = (offset + align - 1) & !(align - 1);
                    }
                    // Check if the bit-field fits in current storage unit
                    if bit_offset + bit_width > storage_bits {
                        // Move to next storage unit
                        offset += storage_size;
                        offset = (offset + align - 1) & !(align - 1);
                        bit_offset = 0;
                    }
                    members.push(StructMember {
                        name: mem_name,
                        ty: mem_ty.clone(),
                        offset,
                        bit_width,
                        bit_offset,
                    });
                    bit_offset += bit_width;
                    // If we filled the storage unit, advance offset
                    if bit_offset >= storage_bits {
                        offset += storage_size;
                        bit_offset = 0;
                    }
                } else {
                    // Normal member: finish any pending bit-field storage unit
                    if bit_offset > 0 {
                        let prev_storage = mem_ty.size(); // approximate
                        offset += prev_storage;
                        bit_offset = 0;
                    }
                    // Struct: align offset to member alignment
                    let align = mem_ty.align();
                    offset = (offset + align - 1) & !(align - 1);
                    members.push(StructMember {
                        name: mem_name,
                        ty: mem_ty.clone(),
                        offset,
                        bit_width: 0,
                        bit_offset: 0,
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
                // If this tag was forward-declared, update all references to it
                if self.forward_declared_tags.remove(tag) {
                    self.update_typedefs_for_tag(tag, &ty);
                    self.update_struct_members_with_struct(&ty);
                }
            }
            ty
        } else if let Some(ref tag) = tag_name {
            // Look up tag, or create forward declaration
            match self.struct_tags.get(tag) {
                Some(ty) => ty.clone(),
                None => {
                    // Forward declaration: register an empty struct/union
                    let ty = Type {
                        kind: crate::types::TypeKind::Struct(Vec::new()),
                        is_unsigned: false,
                    };
                    self.struct_tags.insert(tag.clone(), ty.clone());
                    self.forward_declared_tags.insert(tag.clone());
                    ty
                }
            }
        } else {
            self.reporter.error_at(
                self.current().pos,
                &format!("expected {} tag or body", kind_name),
            );
        }
    }

    /// Update typedefs that reference a specific struct tag with the full definition.
    fn update_typedefs_for_tag(&mut self, tag: &str, full_ty: &Type) {
        // Find all typedef names that reference this struct tag
        let typedef_names: Vec<String> = self.typedef_struct_tags.iter()
            .filter(|(_, v)| *v == tag)
            .map(|(k, _)| k.clone())
            .collect();
        for name in typedef_names {
            let ty = self.typedefs.get(&name).unwrap().clone();
            if let Some(updated) = Self::replace_empty_struct(&ty, full_ty) {
                self.typedefs.insert(name, updated);
            }
        }
    }

    /// Update struct members in all defined structs (both struct_tags and typedefs)
    /// that reference a forward-declared struct.
    fn update_struct_members_with_struct(&mut self, full_ty: &Type) {
        // Update struct_tags
        let keys: Vec<String> = self.struct_tags.keys().cloned().collect();
        for key in keys {
            let st = self.struct_tags.get(&key).unwrap().clone();
            if let Some(updated) = Self::update_struct_type_members(&st, full_ty) {
                self.struct_tags.insert(key, updated);
            }
        }
        // Update typedefs (some structs are anonymous and only in typedefs)
        let keys: Vec<String> = self.typedefs.keys().cloned().collect();
        for key in keys {
            let ty = self.typedefs.get(&key).unwrap().clone();
            if let Some(updated) = Self::update_struct_type_members(&ty, full_ty) {
                self.typedefs.insert(key, updated);
            }
        }
    }

    /// If the type is a struct, update its members that reference empty structs.
    fn update_struct_type_members(ty: &Type, full_ty: &Type) -> Option<Type> {
        match &ty.kind {
            TypeKind::Struct(members) if !members.is_empty() => {
                let mut updated_members = members.clone();
                let mut changed = false;
                for m in &mut updated_members {
                    if let Some(new_ty) = Self::replace_empty_struct(&m.ty, full_ty) {
                        m.ty = new_ty;
                        changed = true;
                    }
                }
                if changed {
                    Some(Type {
                        kind: TypeKind::Struct(updated_members),
                        is_unsigned: ty.is_unsigned,
                    })
                } else {
                    None
                }
            }
            TypeKind::Ptr(base) => {
                Self::update_struct_type_members(base, full_ty).map(|updated| Type {
                    kind: TypeKind::Ptr(Box::new(updated)),
                    is_unsigned: ty.is_unsigned,
                })
            }
            _ => None,
        }
    }

    /// Recursively replace empty structs in a type tree with the full struct definition.
    fn replace_empty_struct(ty: &Type, full_ty: &Type) -> Option<Type> {
        match &ty.kind {
            TypeKind::Struct(members) if members.is_empty() => {
                if let TypeKind::Struct(full_members) = &full_ty.kind {
                    if !full_members.is_empty() {
                        return Some(full_ty.clone());
                    }
                }
                None
            }
            TypeKind::Ptr(base) => {
                Self::replace_empty_struct(base, full_ty).map(|updated| Type {
                    kind: TypeKind::Ptr(Box::new(updated)),
                    is_unsigned: ty.is_unsigned,
                })
            }
            TypeKind::Array(base, size) => {
                Self::replace_empty_struct(base, full_ty).map(|updated| Type {
                    kind: TypeKind::Array(Box::new(updated), *size),
                    is_unsigned: ty.is_unsigned,
                })
            }
            _ => None,
        }
    }

    fn parse_type(&mut self) -> Type {
        // Skip __attribute__ and inline before type
        self.skip_attribute();
        while matches!(self.current().kind, TokenKind::Inline | TokenKind::Noreturn | TokenKind::Register | TokenKind::Extension | TokenKind::Auto | TokenKind::ThreadLocal) {
            self.advance();
        }
        self.skip_attribute();
        // Skip type qualifiers (const, volatile) and _Alignas
        while matches!(self.current().kind, TokenKind::Const | TokenKind::Volatile | TokenKind::Restrict | TokenKind::Alignas) {
            if self.current().kind == TokenKind::Alignas {
                self.advance(); // _Alignas
                self.expect(TokenKind::LParen);
                // Skip the alignment value (number or type)
                if self.is_type_start(&self.current().kind.clone()) {
                    let _ty = self.parse_type();
                } else {
                    self.advance(); // skip number
                }
                self.expect(TokenKind::RParen);
            } else {
                self.advance();
            }
        }
        // Handle signed/unsigned specifiers
        let mut has_signedness = false;
        let is_unsigned = if self.current().kind == TokenKind::Unsigned {
            self.advance();
            has_signedness = true;
            true
        } else {
            if self.current().kind == TokenKind::Signed {
                self.advance();
                has_signedness = true;
            }
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
                // Skip optional "int" (short int)
                if self.current().kind == TokenKind::Int {
                    self.advance();
                }
                if is_unsigned { Type::ushort() } else { Type::short_type() }
            }
            TokenKind::Long => {
                self.advance();
                // Skip optional "long" (long long) or "int" (long int)
                if self.current().kind == TokenKind::Long {
                    self.advance();
                    // Skip optional "int" after "long long"
                    if self.current().kind == TokenKind::Int {
                        self.advance();
                    }
                } else if self.current().kind == TokenKind::Int {
                    self.advance();
                }
                if is_unsigned { Type::ulong() } else { Type::long_type() }
            }
            TokenKind::FloatKw => {
                self.advance();
                Type::float_type()
            }
            TokenKind::DoubleKw => {
                self.advance();
                Type::double_type()
            }
            TokenKind::Void => {
                self.advance();
                Type::void()
            }
            TokenKind::Bool => {
                self.advance();
                Type::bool_type()
            }
            TokenKind::Typeof => {
                self.advance();
                self.expect(TokenKind::LParen);
                let ty = if self.is_type_start(&self.current().kind.clone()) {
                    // typeof(type)
                    let t = self.parse_type();
                    t
                } else {
                    // typeof(expr)
                    let expr = self.expr();
                    self.infer_type(&expr)
                };
                self.expect(TokenKind::RParen);
                ty
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
                        // Optional explicit value: = constant_expr
                        if self.current().kind == TokenKind::Eq {
                            self.advance();
                            val = self.eval_const_expr();
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
                // Check for __int128 and similar identifier types first
                // (even after unsigned/signed)
                if let TokenKind::Ident(name) = &self.current().kind {
                    if name == "__int128" || name == "__int128_t" || name == "__uint128_t" {
                        let is_u128 = name == "__uint128_t";
                        self.advance();
                        if is_unsigned || is_u128 { Type::ulong() } else { Type::long_type() }
                    } else if has_signedness {
                        // After unsigned/signed with no base type keyword:
                        // "unsigned" or "signed" alone = int
                        if is_unsigned { Type::uint() } else { Type::int_type() }
                    } else if name == "va_list" || name == "__builtin_va_list" {
                        self.advance();
                        Type::ptr_to(Type::char_type())
                    } else if let Some(ty) = self.typedefs.get(name).cloned() {
                        self.advance();
                        ty
                    } else {
                        self.reporter.error_at(
                            self.current().pos,
                            &format!("expected type, but got {:?}", self.current().kind),
                        );
                    }
                } else if is_unsigned {
                    Type::uint()
                } else if has_signedness {
                    Type::int_type()
                } else {
                    self.reporter.error_at(
                        self.current().pos,
                        &format!("expected type, but got {:?}", self.current().kind),
                    );
                }
            }
        };

        // Parse pointer stars: type ("*" qualifier*)*
        while self.current().kind == TokenKind::Star {
            self.advance();
            // Skip qualifiers after * (e.g., int *const p)
            while matches!(self.current().kind, TokenKind::Const | TokenKind::Volatile | TokenKind::Restrict | TokenKind::Alignas) {
                self.advance();
            }
            ty = Type::ptr_to(ty);
        }
        // Skip trailing __attribute__
        self.skip_attribute();

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

        // Parse parameter list: (type ident ("," type ident)* ("," "...")?)?
        let mut is_variadic = false;
        let mut is_kr_style = false;
        if self.current().kind != TokenKind::RParen {
            // Detect K&R style: first token is an identifier that is NOT a type name
            if let TokenKind::Ident(ref _first_name) = self.current().kind {
                if !self.is_type_start(&self.current().kind.clone()) {
                    // K&R style parameter list: (a, b, c)
                    is_kr_style = true;
                    loop {
                        let pname = match &self.current().kind {
                            TokenKind::Ident(s) => s.clone(),
                            _ => break,
                        };
                        self.advance();
                        // Default K&R params to int
                        let param_ty = Type::int_type();
                        let unique = self.declare_var(&pname, param_ty.clone());
                        params.push((param_ty, unique));
                        if self.current().kind != TokenKind::Comma {
                            break;
                        }
                        self.advance();
                    }
                }
            }
        }
        if !is_kr_style && self.current().kind != TokenKind::RParen {
            loop {
                // Check for ... (variadic)
                if self.current().kind == TokenKind::Ellipsis {
                    is_variadic = true;
                    self.advance();
                    break;
                }
                let mut param_ty = self.parse_type();

                // void as sole parameter means no parameters
                if param_ty.kind == TypeKind::Void && self.current().kind == TokenKind::RParen {
                    break;
                }

                // Function pointer parameter: type (*name)(param_types)
                if self.current().kind == TokenKind::LParen
                    && self.pos + 1 < self.tokens.len()
                    && self.tokens[self.pos + 1].kind == TokenKind::Star
                {
                    self.advance(); // (
                    self.advance(); // *
                    // Skip qualifiers
                    while matches!(self.current().kind, TokenKind::Const | TokenKind::Volatile | TokenKind::Restrict) {
                        self.advance();
                    }
                    let param_name = match &self.current().kind {
                        TokenKind::Ident(s) => {
                            let s = s.clone();
                            self.advance();
                            s
                        }
                        _ => {
                            // Anonymous function pointer parameter (abstract declarator)
                            self.unique_counter += 1;
                            format!("__anon_fptr.{}", self.unique_counter)
                        }
                    };
                    self.expect(TokenKind::RParen); // )
                    // Skip parameter type list
                    self.expect(TokenKind::LParen);
                    while self.current().kind != TokenKind::RParen {
                        if self.current().kind == TokenKind::Ellipsis {
                            self.advance();
                        } else if self.is_type_start(&self.current().kind.clone()) {
                            let _pty = self.parse_type();
                            // Skip optional parameter name
                            if let TokenKind::Ident(_) = &self.current().kind {
                                self.advance();
                            }
                        } else if self.current().kind == TokenKind::Void {
                            self.advance();
                        }
                        if self.current().kind == TokenKind::Comma {
                            self.advance();
                        }
                    }
                    self.expect(TokenKind::RParen);
                    param_ty = Type::ptr_to(Type::void());
                    let unique = self.declare_var(&param_name, param_ty.clone());
                    params.push((param_ty, unique));
                    if self.current().kind != TokenKind::Comma {
                        break;
                    }
                    self.advance();
                    continue;
                }

                let param_name = match &self.current().kind {
                    TokenKind::Ident(s) => {
                        let s = s.clone();
                        self.advance();
                        s
                    }
                    // Abstract declarator: no parameter name (e.g., prototype "void foo(int, int);")
                    TokenKind::Comma | TokenKind::RParen => {
                        self.unique_counter += 1;
                        format!("__anon_param.{}", self.unique_counter)
                    }
                    // Pointer to pointer or other modifier
                    TokenKind::Star => {
                        // Already consumed by parse_type, generate anonymous name
                        self.unique_counter += 1;
                        format!("__anon_param.{}", self.unique_counter)
                    }
                    _ => {
                        self.reporter.error_at(
                            self.current().pos,
                            "expected parameter name",
                        );
                    }
                };
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
        // Skip __attribute__ after parameter list
        self.skip_attribute();

        // K&R style parameter declarations: int add(a, b) int a; int b; { ... }
        // Detected when params have default int type and next token is a type keyword
        if !params.is_empty() && self.current().kind != TokenKind::LBrace
            && self.current().kind != TokenKind::Semicolon
            && self.is_type_start(&self.current().kind.clone())
        {
            // Read K&R parameter type declarations until '{'
            while self.current().kind != TokenKind::LBrace {
                let kr_ty = self.parse_type();
                // Parse declarator names (may be comma-separated)
                loop {
                    if let TokenKind::Ident(pname) = &self.current().kind {
                        let pname = pname.clone();
                        self.advance();
                        // Update the matching parameter's type
                        for (pty, uname) in params.iter_mut() {
                            // Match by original name (unique name starts with original)
                            let orig = uname.split('.').next().unwrap_or(uname);
                            if orig == pname {
                                *pty = kr_ty.clone();
                                // Update local variable type
                                if let Some(local) = self.locals.iter_mut().find(|l| l.1 == *uname) {
                                    local.0 = kr_ty.clone();
                                }
                                break;
                            }
                        }
                    }
                    if self.current().kind == TokenKind::Comma {
                        self.advance();
                    } else {
                        break;
                    }
                }
                self.expect(TokenKind::Semicolon);
            }
        }

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
        Some(Function { name, return_ty, params, is_variadic, body, locals })
    }

    // stmt = "return" expr ";"
    //      | "if" "(" expr ")" stmt ("else" stmt)?
    //      | "int" ident ("=" expr)? ";"
    //      | expr ";"
    fn stmt(&mut self) -> Stmt {
        match &self.current().kind {
            // Empty statement: just a semicolon
            TokenKind::Semicolon => {
                self.advance();
                Stmt::Block(vec![])
            }
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

                // Check if init is a variable declaration (needs scope)
                let has_decl_init = self.is_type_start(&self.current().kind.clone()) && self.current().kind != TokenKind::Void;
                if has_decl_init {
                    self.enter_scope();
                }

                // init
                let init = if self.current().kind == TokenKind::Semicolon {
                    self.advance();
                    None
                } else if has_decl_init {
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

                if has_decl_init {
                    self.leave_scope();
                }

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
                        let val = self.eval_const_expr();
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
            TokenKind::Asm => {
                // Skip inline assembly: asm/volatile (...);
                self.advance();
                // Skip optional "volatile" or "__volatile__" qualifier
                if let TokenKind::Ident(ref s) = self.current().kind {
                    if s == "volatile" || s == "__volatile__" {
                        self.advance();
                    }
                }
                if self.current().kind == TokenKind::Volatile {
                    self.advance();
                }
                // Skip balanced parentheses
                if self.current().kind == TokenKind::LParen {
                    self.advance();
                    let mut depth = 1;
                    while depth > 0 {
                        match self.current().kind {
                            TokenKind::LParen => depth += 1,
                            TokenKind::RParen => depth -= 1,
                            TokenKind::Eof => break,
                            _ => {}
                        }
                        self.advance();
                    }
                }
                self.expect(TokenKind::Semicolon);
                Stmt::Block(vec![]) // no-op
            }
            TokenKind::Goto => {
                self.advance();
                if self.current().kind == TokenKind::Star {
                    // Computed goto: goto *expr;
                    self.advance();
                    let expr = self.expr();
                    self.expect(TokenKind::Semicolon);
                    Stmt::GotoExpr(expr)
                } else {
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
                // Check for function pointer typedef: typedef RetType (*Name)(params...)
                if self.current().kind == TokenKind::LParen
                    && self.pos + 1 < self.tokens.len()
                    && self.tokens[self.pos + 1].kind == TokenKind::Star
                {
                    self.advance(); // (
                    self.advance(); // *
                    while matches!(self.current().kind, TokenKind::Const | TokenKind::Volatile | TokenKind::Restrict) {
                        self.advance();
                    }
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
                    self.expect(TokenKind::RParen);
                    if self.current().kind == TokenKind::LParen {
                        let mut depth = 1;
                        self.advance();
                        while depth > 0 {
                            match self.current().kind {
                                TokenKind::LParen => depth += 1,
                                TokenKind::RParen => depth -= 1,
                                TokenKind::Eof => break,
                                _ => {}
                            }
                            self.advance();
                        }
                    }
                    self.skip_attribute();
                    self.expect(TokenKind::Semicolon);
                    if let Some(ref tag) = self.last_struct_tag {
                        self.typedef_struct_tags.insert(name.clone(), tag.clone());
                    }
                    self.typedefs.insert(name, Type::ptr_to(ty));
                    Stmt::Block(vec![])
                } else {
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
                    // Handle array typedef
                    if self.current().kind == TokenKind::LBracket {
                        self.advance();
                        let size = if self.current().kind != TokenKind::RBracket {
                            let s = self.eval_const_expr();
                            s as usize
                        } else {
                            0
                        };
                        self.expect(TokenKind::RBracket);
                        self.expect(TokenKind::Semicolon);
                        let arr_ty = Type { kind: crate::types::TypeKind::Array(Box::new(ty), size), is_unsigned: false };
                        if let Some(ref tag) = self.last_struct_tag {
                            self.typedef_struct_tags.insert(name.clone(), tag.clone());
                        }
                        self.typedefs.insert(name, arr_ty);
                        Stmt::Block(vec![])
                    } else {
                        self.expect(TokenKind::Semicolon);
                        if let Some(ref tag) = self.last_struct_tag {
                            self.typedef_struct_tags.insert(name.clone(), tag.clone());
                        }
                        self.typedefs.insert(name, ty);
                        Stmt::Block(vec![])
                    }
                }
            }
            TokenKind::StaticAssert => {
                self.skip_static_assert();
                Stmt::Block(vec![])
            }
            TokenKind::Static => {
                self.advance();
                self.static_local_var()
            }
            TokenKind::Void | TokenKind::Int | TokenKind::Char | TokenKind::Short | TokenKind::Long | TokenKind::Signed | TokenKind::Unsigned | TokenKind::Bool | TokenKind::Struct | TokenKind::Union | TokenKind::Enum | TokenKind::Const | TokenKind::Volatile | TokenKind::Restrict | TokenKind::Alignas | TokenKind::FloatKw | TokenKind::DoubleKw | TokenKind::Attribute | TokenKind::Inline | TokenKind::Noreturn | TokenKind::Register | TokenKind::Extension | TokenKind::Typeof | TokenKind::Auto => {
                self.var_decl()
            }
            _ => {
                // Check for typedef name or va_list as type
                if let TokenKind::Ident(name) = &self.current().kind {
                    if self.typedefs.contains_key(name) || name == "va_list" || name == "__builtin_va_list" {
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

    /// Parse function pointer declaration: type (*name)(param_types) (= expr)?;
    /// The return type has already been parsed. Current token is '('.
    fn parse_func_ptr_or_array_ptr_decl(&mut self, base_ty: Type) -> Stmt {
        self.expect(TokenKind::LParen);  // (
        self.expect(TokenKind::Star);    // *
        let name = match &self.current().kind {
            TokenKind::Ident(s) => s.clone(),
            _ => {
                self.reporter.error_at(
                    self.current().pos,
                    "expected pointer name",
                );
            }
        };
        self.advance();

        // Check for array dimension inside parens: (*name[N])
        let mut array_size: Option<usize> = None;
        if self.current().kind == TokenKind::LBracket {
            self.advance();
            let size = self.eval_const_expr();
            self.expect(TokenKind::RBracket);
            array_size = Some(size as usize);
        }

        self.expect(TokenKind::RParen);  // )

        if self.current().kind == TokenKind::LBracket && array_size.is_none() {
            // Array pointer: type (*name)[size]
            self.advance();
            let size = self.eval_const_expr();
            self.expect(TokenKind::RBracket);

            // type (*name)[N] is a pointer to array of N elements of type
            let arr_ty = Type::array_of(base_ty, size as usize);
            let ptr_ty = Type::ptr_to(arr_ty);
            let unique = self.declare_var(&name, ptr_ty.clone());

            let init = if self.current().kind == TokenKind::Eq {
                self.advance();
                Some(self.assign())
            } else {
                None
            };
            self.expect(TokenKind::Semicolon);
            Stmt::VarDecl { name: unique, ty: ptr_ty, init }
        } else if self.current().kind == TokenKind::LParen {
            // Function pointer: type (*name)(param_types)
            // Or function pointer array: type (*name[N])(param_types)
            self.advance(); // skip (
            while self.current().kind != TokenKind::RParen {
                if self.current().kind == TokenKind::Ellipsis {
                    self.advance();
                } else if self.is_type_start(&self.current().kind.clone()) {
                    let _param_ty = self.parse_type();
                    // Skip optional parameter name
                    if let TokenKind::Ident(_) = &self.current().kind {
                        self.advance();
                    }
                    // Skip array dimensions in params
                    while self.current().kind == TokenKind::LBracket {
                        self.advance();
                        while self.current().kind != TokenKind::RBracket {
                            self.advance();
                        }
                        self.expect(TokenKind::RBracket);
                    }
                } else if self.current().kind == TokenKind::Void {
                    self.advance();
                }
                if self.current().kind == TokenKind::Comma {
                    self.advance();
                }
            }
            self.expect(TokenKind::RParen);

            if let Some(arr_len) = array_size {
                // Function pointer array: type (*name[N])(params)
                // Each element is a function pointer (8 bytes)
                let fptr_ty = Type::ptr_to(Type::void());
                let arr_ty = Type::array_of(fptr_ty, arr_len);
                let unique = self.declare_var(&name, arr_ty.clone());

                let init = if self.current().kind == TokenKind::Eq {
                    self.advance();
                    Some(self.assign())
                } else {
                    None
                };
                self.expect(TokenKind::Semicolon);
                Stmt::VarDecl { name: unique, ty: arr_ty, init }
            } else {
                // Function pointer: type (*name)(params)
                let fptr_ty = Type::ptr_to(Type::void());
                let unique = self.declare_var(&name, fptr_ty.clone());

                let init = if self.current().kind == TokenKind::Eq {
                    self.advance();
                    Some(self.assign())
                } else {
                    None
                };
                self.expect(TokenKind::Semicolon);
                Stmt::VarDecl { name: unique, ty: fptr_ty, init }
            }
        } else {
            // Plain pointer declaration (shouldn't normally reach here)
            let fptr_ty = Type::ptr_to(Type::void());
            let unique = self.declare_var(&name, fptr_ty.clone());
            self.expect(TokenKind::Semicolon);
            Stmt::VarDecl { name: unique, ty: fptr_ty, init: None }
        }
    }

    // var_decl = type ident ("[" num "]")* ("=" expr)? ";"
    //         | type "(" "*" ident ")" "(" param_types ")" ("=" expr)? ";"
    //         | "struct" tag "{" ... "}" ";"  (tag definition only)
    fn var_decl(&mut self) -> Stmt {
        let ty = self.parse_type();
        // Allow struct tag definition without variable declaration
        if self.current().kind == TokenKind::Semicolon {
            self.advance();
            return Stmt::Block(vec![]);
        }

        // Function pointer declaration: type (*name)(param_types)
        if self.current().kind == TokenKind::LParen
            && self.pos + 1 < self.tokens.len()
            && self.tokens[self.pos + 1].kind == TokenKind::Star
        {
            return self.parse_func_ptr_or_array_ptr_decl(ty);
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
                    let len = self.eval_const_expr() as usize;
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
                let init = Some(self.assign());

                // Check for multi-variable declaration: int a=1, b=2;
                if self.current().kind == TokenKind::Comma {
                    let mut stmts = vec![Stmt::VarDecl { name: unique, ty: ty.clone(), init }];
                    while self.current().kind == TokenKind::Comma {
                        self.advance();
                        // Parse pointer stars for this declarator
                        let mut decl_ty = ty.clone();
                        while self.current().kind == TokenKind::Star {
                            self.advance();
                            while matches!(self.current().kind, TokenKind::Const | TokenKind::Volatile | TokenKind::Restrict | TokenKind::Alignas) {
                                self.advance();
                            }
                            decl_ty = Type::ptr_to(decl_ty);
                        }
                        let next_name = match &self.current().kind {
                            TokenKind::Ident(s) => s.clone(),
                            _ => {
                                self.reporter.error_at(
                                    self.current().pos,
                                    "expected variable name",
                                );
                            }
                        };
                        self.advance();
                        // Parse array dimensions for this declarator
                        while self.current().kind == TokenKind::LBracket {
                            self.advance();
                            if self.current().kind == TokenKind::RBracket {
                                self.advance();
                                decl_ty = Type { kind: TypeKind::Array(Box::new(decl_ty), 0), is_unsigned: false };
                            } else {
                                let size = self.eval_const_expr();
                                self.expect(TokenKind::RBracket);
                                decl_ty = Type { kind: TypeKind::Array(Box::new(decl_ty), size as usize), is_unsigned: false };
                            }
                        }
                        let next_init = if self.current().kind == TokenKind::Eq {
                            self.advance();
                            Some(self.assign())
                        } else {
                            None
                        };
                        let next_unique = self.declare_var(&next_name, decl_ty.clone());
                        stmts.push(Stmt::VarDecl { name: next_unique, ty: decl_ty, init: next_init });
                    }
                    self.expect(TokenKind::Semicolon);
                    return Stmt::Block(stmts);
                }

                self.expect(TokenKind::Semicolon);
                return Stmt::VarDecl { name: unique, ty, init };
            }
        }

        let unique = self.declare_var(&name, ty.clone());

        // Check for multi-variable declaration without initializer: int a, b;
        if self.current().kind == TokenKind::Comma {
            let mut stmts = vec![Stmt::VarDecl { name: unique, ty: ty.clone(), init: None }];
            while self.current().kind == TokenKind::Comma {
                self.advance();
                let mut decl_ty = ty.clone();
                while self.current().kind == TokenKind::Star {
                    self.advance();
                    while matches!(self.current().kind, TokenKind::Const | TokenKind::Volatile | TokenKind::Restrict | TokenKind::Alignas) {
                        self.advance();
                    }
                    decl_ty = Type::ptr_to(decl_ty);
                }
                let next_name = match &self.current().kind {
                    TokenKind::Ident(s) => s.clone(),
                    _ => {
                        self.reporter.error_at(
                            self.current().pos,
                            "expected variable name",
                        );
                    }
                };
                self.advance();
                // Parse array dimensions for this declarator
                while self.current().kind == TokenKind::LBracket {
                    self.advance();
                    if self.current().kind == TokenKind::RBracket {
                        self.advance();
                        decl_ty = Type { kind: TypeKind::Array(Box::new(decl_ty), 0), is_unsigned: false };
                    } else {
                        let size = self.eval_const_expr();
                        self.expect(TokenKind::RBracket);
                        decl_ty = Type { kind: TypeKind::Array(Box::new(decl_ty), size as usize), is_unsigned: false };
                    }
                }
                let next_init = if self.current().kind == TokenKind::Eq {
                    self.advance();
                    Some(self.assign())
                } else {
                    None
                };
                let next_unique = self.declare_var(&next_name, decl_ty.clone());
                stmts.push(Stmt::VarDecl { name: next_unique, ty: decl_ty, init: next_init });
            }
            self.expect(TokenKind::Semicolon);
            return Stmt::Block(stmts);
        }

        self.expect(TokenKind::Semicolon);
        Stmt::VarDecl { name: unique, ty, init: None }
    }

    /// Parse static local variable declaration.
    /// Static locals are stored as global variables with unique names.
    fn static_local_var(&mut self) -> Stmt {
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

        // Generate unique global name: __static.func.name.counter
        self.unique_counter += 1;
        let global_name = format!("__static.{}.{}", name, self.unique_counter);

        // Parse optional initializer
        let init_bytes = if self.current().kind == TokenKind::Eq {
            self.advance();
            let val = match &self.current().kind {
                TokenKind::Num(n) => *n,
                _ => {
                    self.reporter.error_at(
                        self.current().pos,
                        "expected constant in static initializer",
                    );
                }
            };
            self.advance();
            let elem_size = ty.size();
            let mut bytes = Vec::new();
            for i in 0..elem_size {
                bytes.push(((val >> (i * 8)) & 0xff) as u8);
            }
            Some(bytes)
        } else {
            None
        };

        self.expect(TokenKind::Semicolon);

        // Register as global variable
        self.globals.push((ty.clone(), global_name.clone(), init_bytes));

        // Register the global name in the current scope so local code uses it
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name, global_name);
        }

        // No local VarDecl needed — it's a global
        Stmt::Block(vec![])
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
            TokenKind::AmpEq => Some(BinOp::BitAnd),
            TokenKind::PipeEq => Some(BinOp::BitOr),
            TokenKind::CaretEq => Some(BinOp::BitXor),
            TokenKind::LShiftEq => Some(BinOp::Shl),
            TokenKind::RShiftEq => Some(BinOp::Shr),
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
            node = Self::make_binop(BinOp::BitOr, node, rhs);
        }

        node
    }

    // bitwise_xor = bitwise_and ("^" bitwise_and)*
    fn bitwise_xor(&mut self) -> Expr {
        let mut node = self.bitwise_and();

        while self.current().kind == TokenKind::Caret {
            self.advance();
            let rhs = self.bitwise_and();
            node = Self::make_binop(BinOp::BitXor, node, rhs);
        }

        node
    }

    // bitwise_and = equality ("&" equality)*
    fn bitwise_and(&mut self) -> Expr {
        let mut node = self.equality();

        while self.current().kind == TokenKind::Amp {
            self.advance();
            let rhs = self.equality();
            node = Self::make_binop(BinOp::BitAnd, node, rhs);
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
                    node = Self::make_binop(BinOp::Eq, node, rhs);
                }
                TokenKind::Ne => {
                    self.advance();
                    let rhs = self.relational();
                    node = Self::make_binop(BinOp::Ne, node, rhs);
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
                    node = Self::make_binop(BinOp::Lt, node, rhs);
                }
                TokenKind::Le => {
                    self.advance();
                    let rhs = self.shift();
                    node = Self::make_binop(BinOp::Le, node, rhs);
                }
                TokenKind::Gt => {
                    self.advance();
                    let rhs = self.shift();
                    node = Self::make_binop(BinOp::Gt, node, rhs);
                }
                TokenKind::Ge => {
                    self.advance();
                    let rhs = self.shift();
                    node = Self::make_binop(BinOp::Ge, node, rhs);
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
                    node = Self::make_binop(BinOp::Shl, node, rhs);
                }
                TokenKind::RShift => {
                    self.advance();
                    let rhs = self.add();
                    node = Self::make_binop(BinOp::Shr, node, rhs);
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
                    node = Self::make_binop(BinOp::Add, node, rhs);
                }
                TokenKind::Minus => {
                    self.advance();
                    let rhs = self.mul();
                    node = Self::make_binop(BinOp::Sub, node, rhs);
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
                    node = Self::make_binop(BinOp::Mul, node, rhs);
                }
                TokenKind::Slash => {
                    self.advance();
                    let rhs = self.unary();
                    node = Self::make_binop(BinOp::Div, node, rhs);
                }
                TokenKind::Percent => {
                    self.advance();
                    let rhs = self.unary();
                    node = Self::make_binop(BinOp::Mod, node, rhs);
                }
                _ => break,
            }
        }

        node
    }

    // unary = ("+" | "-" | "&" | "*") unary | "++" unary | "--" unary | postfix
    fn unary(&mut self) -> Expr {
        // Skip __extension__ before expressions
        while self.current().kind == TokenKind::Extension {
            self.advance();
        }
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
            TokenKind::AmpAmp => {
                // &&label — address of label (GCC extension)
                self.advance();
                let label = match &self.current().kind {
                    TokenKind::Ident(s) => s.clone(),
                    _ => {
                        self.reporter.error_at(
                            self.current().pos,
                            "expected label name after &&",
                        );
                    }
                };
                self.advance();
                Expr::LabelAddr(label)
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
                    && self.is_type_start(&self.tokens[self.pos + 1].kind)
                {
                    self.advance(); // consume "("
                    let ty = self.parse_type();
                    self.expect(TokenKind::RParen);
                    return Expr::SizeofType(ty);
                }
                let operand = self.unary();
                return Expr::SizeofExpr(Box::new(operand));
            }
            TokenKind::Alignof => {
                self.advance();
                // _Alignof(type)
                self.expect(TokenKind::LParen);
                let ty = self.parse_type();
                self.expect(TokenKind::RParen);
                return Expr::Num(ty.align() as i64);
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
                    && self.is_type_start(&self.tokens[self.pos + 1].kind)
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
                            let n = self.eval_const_expr() as usize;
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
                    // Check for function pointer call through struct member: s.fp(args)
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
                        node = Expr::FuncPtrCall {
                            fptr: Box::new(node),
                            args,
                        };
                    }
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
                    // Check for function pointer call through pointer member: p->fp(args)
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
                        node = Expr::FuncPtrCall {
                            fptr: Box::new(node),
                            args,
                        };
                    }
                }
                TokenKind::LParen => {
                    // Function pointer call through any expression: expr(args)
                    // e.g., ops[0](10, 5), (*fp)(a, b)
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
                    node = Expr::FuncPtrCall {
                        fptr: Box::new(node),
                        args,
                    };
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
            TokenKind::Generic => {
                self.advance();
                return self.parse_generic();
            }
            TokenKind::Num(val) => {
                self.advance();
                Expr::Num(val)
            }
            TokenKind::FloatNum(val) => {
                self.advance();
                Expr::FloatLit(val)
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

                // va_start(ap, last_param)
                if name == "va_start" || name == "__builtin_va_start" {
                    self.expect(TokenKind::LParen);
                    let ap = self.assign();
                    self.expect(TokenKind::Comma);
                    let last_param = match &self.current().kind {
                        TokenKind::Ident(s) => s.clone(),
                        _ => {
                            self.reporter.error_at(
                                self.current().pos,
                                "expected parameter name in va_start",
                            );
                        }
                    };
                    self.advance();
                    self.expect(TokenKind::RParen);
                    let resolved = self.resolve_var(&last_param);
                    return Expr::VaStart {
                        ap: Box::new(ap),
                        last_param: resolved,
                    };
                }

                // va_arg(ap, type)
                if name == "va_arg" || name == "__builtin_va_arg" {
                    self.expect(TokenKind::LParen);
                    let ap = self.assign();
                    self.expect(TokenKind::Comma);
                    let ty = self.parse_type();
                    self.expect(TokenKind::RParen);
                    return Expr::VaArg {
                        ap: Box::new(ap),
                        ty,
                    };
                }

                // va_end(ap) — no-op, evaluates to 0
                if name == "va_end" || name == "__builtin_va_end" {
                    self.expect(TokenKind::LParen);
                    let _ap = self.assign();
                    self.expect(TokenKind::RParen);
                    return Expr::Num(0);
                }

                // va_copy(dest, src) — no-op, evaluates to 0
                if name == "va_copy" || name == "__va_copy" || name == "__builtin_va_copy" {
                    self.expect(TokenKind::LParen);
                    let _dest = self.assign();
                    self.expect(TokenKind::Comma);
                    let _src = self.assign();
                    self.expect(TokenKind::RParen);
                    return Expr::Num(0);
                }

                // __builtin_expect(expr, expected) → returns expr
                if name == "__builtin_expect" {
                    self.expect(TokenKind::LParen);
                    let expr = self.assign();
                    self.expect(TokenKind::Comma);
                    let _expected = self.assign();
                    self.expect(TokenKind::RParen);
                    return expr;
                }

                // __builtin_constant_p(expr) → always 0 (we don't optimize)
                if name == "__builtin_constant_p" {
                    self.expect(TokenKind::LParen);
                    let _expr = self.assign();
                    self.expect(TokenKind::RParen);
                    return Expr::Num(0);
                }

                // __builtin_unreachable() → no-op
                if name == "__builtin_unreachable" {
                    self.expect(TokenKind::LParen);
                    self.expect(TokenKind::RParen);
                    return Expr::Num(0);
                }

                // __builtin_offsetof(type, member) → byte offset
                if name == "__builtin_offsetof" {
                    self.expect(TokenKind::LParen);
                    let ty = self.parse_type();
                    self.expect(TokenKind::Comma);
                    let member_name = match &self.current().kind {
                        TokenKind::Ident(s) => s.clone(),
                        _ => {
                            self.reporter.error_at(
                                self.current().pos,
                                "expected member name in __builtin_offsetof",
                            );
                        }
                    };
                    self.advance();
                    self.expect(TokenKind::RParen);
                    // Find offset in struct
                    if let crate::types::TypeKind::Struct(members) = &ty.kind {
                        for m in members {
                            if m.name == member_name {
                                return Expr::Num(m.offset as i64);
                            }
                        }
                        self.reporter.error_at(
                            self.current().pos,
                            &format!("no member '{}' in struct", member_name),
                        );
                    } else {
                        self.reporter.error_at(
                            self.current().pos,
                            "__builtin_offsetof requires a struct type",
                        );
                    }
                }

                // __builtin_types_compatible_p(type1, type2) → 1 if compatible, 0 if not
                if name == "__builtin_types_compatible_p" {
                    self.expect(TokenKind::LParen);
                    let ty1 = self.parse_type();
                    self.expect(TokenKind::Comma);
                    let ty2 = self.parse_type();
                    self.expect(TokenKind::RParen);
                    let compatible = ty1.kind == ty2.kind && ty1.is_unsigned == ty2.is_unsigned;
                    return Expr::Num(if compatible { 1 } else { 0 });
                }

                // __builtin_choose_expr(const_expr, expr1, expr2) → expr1 if const_expr != 0, else expr2
                if name == "__builtin_choose_expr" {
                    self.expect(TokenKind::LParen);
                    let cond = self.eval_const_expr();
                    self.expect(TokenKind::Comma);
                    let expr1 = self.assign();
                    self.expect(TokenKind::Comma);
                    let expr2 = self.assign();
                    self.expect(TokenKind::RParen);
                    return if cond != 0 { expr1 } else { expr2 };
                }

                // __builtin_trap() → calls abort
                if name == "__builtin_trap" {
                    self.expect(TokenKind::LParen);
                    self.expect(TokenKind::RParen);
                    return Expr::FuncCall { name: "abort".to_string(), args: vec![] };
                }

                // __builtin_clz, __builtin_ctz, __builtin_popcount, __builtin_bswap*
                // → emit as regular function calls to GCC builtins (linked via libgcc)
                if name == "__builtin_clz" || name == "__builtin_ctz"
                    || name == "__builtin_clzl" || name == "__builtin_ctzl"
                    || name == "__builtin_clzll" || name == "__builtin_ctzll"
                    || name == "__builtin_popcount" || name == "__builtin_popcountl" || name == "__builtin_popcountll"
                    || name == "__builtin_bswap16" || name == "__builtin_bswap32" || name == "__builtin_bswap64"
                    || name == "__builtin_ffs" || name == "__builtin_ffsl" || name == "__builtin_ffsll"
                    || name == "__builtin_abs"
                {
                    self.expect(TokenKind::LParen);
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

                // __builtin_classify_type(expr) → always 0
                if name == "__builtin_classify_type" {
                    self.expect(TokenKind::LParen);
                    let _expr = self.assign();
                    self.expect(TokenKind::RParen);
                    return Expr::Num(0);
                }

                // __builtin_huge_val() → 0 (simplified, not used for code gen)
                if name == "__builtin_huge_val" || name == "__builtin_inf"
                    || name == "__builtin_huge_valf" || name == "__builtin_inff"
                    || name == "__builtin_nan" || name == "__builtin_nanf"
                {
                    self.expect(TokenKind::LParen);
                    // Skip arguments if any
                    if self.current().kind != TokenKind::RParen {
                        self.assign();
                    }
                    self.expect(TokenKind::RParen);
                    return Expr::Num(0);
                }

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
                    // If the name is a declared variable, it's a function pointer call
                    if self.is_var_declared(&name) {
                        let resolved = self.resolve_var(&name);
                        return Expr::FuncPtrCall {
                            fptr: Box::new(Expr::Var(resolved)),
                            args,
                        };
                    }
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
                // Statement expression: ({ stmt1; stmt2; expr; })
                if self.current().kind == TokenKind::LBrace {
                    self.advance();
                    self.enter_scope();
                    let mut stmts = Vec::new();
                    while self.current().kind != TokenKind::RBrace {
                        stmts.push(self.stmt());
                    }
                    self.expect(TokenKind::RBrace);
                    self.leave_scope();
                    self.expect(TokenKind::RParen);
                    Expr::StmtExpr(stmts)
                } else {
                    let node = self.expr();
                    self.expect(TokenKind::RParen);
                    node
                }
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

    /// Skip __attribute__((...)) if present. May appear multiple times.
    /// Evaluate a constant expression (for enum values, array sizes, etc.)
    /// Returns the integer value of the expression.
    fn eval_const_expr(&mut self) -> i64 {
        let expr = self.ternary();
        self.eval_const(&expr)
    }

    /// Recursively evaluate a constant expression.
    fn eval_const(&self, expr: &Expr) -> i64 {
        match expr {
            Expr::Num(n) => *n,
            Expr::Cast { expr, .. } => self.eval_const(expr),
            Expr::UnaryOp { op, operand } => {
                let val = self.eval_const(operand);
                match op {
                    UnaryOp::Neg => -val,
                    UnaryOp::LogicalNot => if val == 0 { 1 } else { 0 },
                    UnaryOp::BitNot => !val,
                }
            }
            Expr::BinOp { op, lhs, rhs } => {
                let l = self.eval_const(lhs);
                let r = self.eval_const(rhs);
                match op {
                    BinOp::Add => l + r,
                    BinOp::Sub => l - r,
                    BinOp::Mul => l * r,
                    BinOp::Div => if r != 0 { l / r } else { 0 },
                    BinOp::Mod => if r != 0 { l % r } else { 0 },
                    BinOp::Eq => if l == r { 1 } else { 0 },
                    BinOp::Ne => if l != r { 1 } else { 0 },
                    BinOp::Lt => if l < r { 1 } else { 0 },
                    BinOp::Le => if l <= r { 1 } else { 0 },
                    BinOp::Gt => if l > r { 1 } else { 0 },
                    BinOp::Ge => if l >= r { 1 } else { 0 },
                    BinOp::BitAnd => l & r,
                    BinOp::BitOr => l | r,
                    BinOp::BitXor => l ^ r,
                    BinOp::Shl => l << r,
                    BinOp::Shr => l >> r,
                }
            }
            Expr::LogicalAnd(l, r) => {
                if self.eval_const(l) != 0 && self.eval_const(r) != 0 { 1 } else { 0 }
            }
            Expr::LogicalOr(l, r) => {
                if self.eval_const(l) != 0 || self.eval_const(r) != 0 { 1 } else { 0 }
            }
            Expr::Ternary { cond, then_expr, else_expr } => {
                if self.eval_const(cond) != 0 {
                    self.eval_const(then_expr)
                } else {
                    self.eval_const(else_expr)
                }
            }
            Expr::SizeofType(ty) => ty.size() as i64,
            Expr::SizeofExpr(_) => 0, // can't evaluate runtime sizeof
            Expr::Var(name) => {
                // Check enum constants
                if let Some(&val) = self.enum_values.get(name) {
                    val
                } else {
                    0
                }
            }
            _ => 0,
        }
    }

    /// Skip _Static_assert(expr, "message"); or _Static_assert(expr);
    fn skip_static_assert(&mut self) {
        self.advance(); // _Static_assert
        self.expect(TokenKind::LParen);
        // Skip until matching RParen (handles nested parens in the expression)
        let mut depth = 1;
        while depth > 0 {
            match self.current().kind {
                TokenKind::LParen => depth += 1,
                TokenKind::RParen => depth -= 1,
                TokenKind::Eof => break,
                _ => {}
            }
            self.advance();
        }
        // Skip optional semicolon
        if self.current().kind == TokenKind::Semicolon {
            self.advance();
        }
    }

    fn skip_attribute(&mut self) {
        while self.current().kind == TokenKind::Attribute {
            self.advance(); // __attribute__
            if self.current().kind == TokenKind::LParen {
                self.advance(); // outer (
                let mut depth = 1;
                while depth > 0 {
                    match self.current().kind {
                        TokenKind::LParen => depth += 1,
                        TokenKind::RParen => depth -= 1,
                        TokenKind::Eof => break,
                        _ => {}
                    }
                    self.advance();
                }
            }
        }
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

    /// Parse _Generic(control_expr, type: expr, type: expr, ..., default: expr)
    fn parse_generic(&mut self) -> Expr {
        self.expect(TokenKind::LParen);

        // Parse and evaluate the controlling expression (only need its type)
        let ctrl_expr = self.assign();
        let ctrl_ty = self.infer_type(&ctrl_expr);

        self.expect(TokenKind::Comma);

        let mut result: Option<Expr> = None;
        let mut default_expr: Option<Expr> = None;

        loop {
            if self.current().kind == TokenKind::Default {
                self.advance();
                self.expect(TokenKind::Colon);
                let expr = self.assign();
                default_expr = Some(expr);
            } else {
                let assoc_ty = self.parse_type();
                self.expect(TokenKind::Colon);
                let expr = self.assign();
                // Check if this type matches the controlling expression's type
                if result.is_none() && self.types_match(&ctrl_ty, &assoc_ty) {
                    result = Some(expr);
                }
            }
            if self.current().kind == TokenKind::Comma {
                self.advance();
            } else {
                break;
            }
        }
        self.expect(TokenKind::RParen);

        if let Some(r) = result {
            r
        } else if let Some(d) = default_expr {
            d
        } else {
            // No matching type and no default — return 0
            Expr::Num(0)
        }
    }

    /// Infer the type of an expression at parse time (best effort).
    fn infer_type(&self, expr: &Expr) -> Type {
        match expr {
            Expr::Num(_) => Type::int_type(),
            Expr::FloatLit(_) => Type::double_type(),
            Expr::StrLit(_) => Type::ptr_to(Type::char_type()),
            Expr::Var(name) => {
                // Look up variable type from scopes
                for scope in self.scopes.iter().rev() {
                    if let Some(unique) = scope.get(name) {
                        for (ty, n) in &self.locals {
                            if n == unique {
                                return ty.clone();
                            }
                        }
                    }
                }
                // Check globals
                for (ty, n, _) in &self.globals {
                    if n == name {
                        return ty.clone();
                    }
                }
                Type::int_type()
            }
            Expr::Deref(inner) => {
                let inner_ty = self.infer_type(inner);
                match inner_ty.kind {
                    crate::types::TypeKind::Ptr(base) | crate::types::TypeKind::Array(base, _) => *base,
                    _ => Type::long_type(),
                }
            }
            Expr::Addr(inner) => {
                let inner_ty = self.infer_type(inner);
                Type::ptr_to(inner_ty)
            }
            Expr::Cast { ty, .. } => ty.clone(),
            _ => Type::int_type(),
        }
    }

    /// Check if two types are compatible for _Generic matching.
    fn types_match(&self, a: &Type, b: &Type) -> bool {
        use crate::types::TypeKind;
        match (&a.kind, &b.kind) {
            (TypeKind::Void, TypeKind::Void) => true,
            (TypeKind::Bool, TypeKind::Bool) => true,
            (TypeKind::Char, TypeKind::Char) => a.is_unsigned == b.is_unsigned,
            (TypeKind::Short, TypeKind::Short) => a.is_unsigned == b.is_unsigned,
            (TypeKind::Int, TypeKind::Int) => a.is_unsigned == b.is_unsigned,
            (TypeKind::Long, TypeKind::Long) => a.is_unsigned == b.is_unsigned,
            (TypeKind::Float, TypeKind::Float) => true,
            (TypeKind::Double, TypeKind::Double) => true,
            (TypeKind::Ptr(a_base), TypeKind::Ptr(b_base)) => self.types_match(a_base, b_base),
            (TypeKind::Array(a_base, _), TypeKind::Ptr(b_base)) => self.types_match(a_base, b_base),
            (TypeKind::Ptr(a_base), TypeKind::Array(b_base, _)) => self.types_match(a_base, b_base),
            _ => false,
        }
    }

    /// Create a BinOp node, folding constants when both operands are Num.
    fn make_binop(op: BinOp, lhs: Expr, rhs: Expr) -> Expr {
        if let (Expr::Num(l), Expr::Num(r)) = (&lhs, &rhs) {
            let result = match op {
                BinOp::Add => l.wrapping_add(*r),
                BinOp::Sub => l.wrapping_sub(*r),
                BinOp::Mul => l.wrapping_mul(*r),
                BinOp::Div if *r != 0 => l.wrapping_div(*r),
                BinOp::Mod if *r != 0 => l.wrapping_rem(*r),
                BinOp::Eq => if l == r { 1 } else { 0 },
                BinOp::Ne => if l != r { 1 } else { 0 },
                BinOp::Lt => if l < r { 1 } else { 0 },
                BinOp::Le => if l <= r { 1 } else { 0 },
                BinOp::Gt => if l > r { 1 } else { 0 },
                BinOp::Ge => if l >= r { 1 } else { 0 },
                BinOp::BitAnd => l & r,
                BinOp::BitOr => l | r,
                BinOp::BitXor => l ^ r,
                BinOp::Shl => l.wrapping_shl(*r as u32),
                BinOp::Shr => l.wrapping_shr(*r as u32),
                _ => {
                    return Expr::BinOp { op, lhs: Box::new(lhs), rhs: Box::new(rhs) };
                }
            };
            return Expr::Num(result);
        }
        Expr::BinOp { op, lhs: Box::new(lhs), rhs: Box::new(rhs) }
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

    /// Check if a variable name is declared in any scope (local or global).
    /// Excludes extern-declared names (which are function prototypes, not variables).
    fn is_var_declared(&self, name: &str) -> bool {
        for scope in self.scopes.iter().rev() {
            if scope.contains_key(name) {
                return true;
            }
        }
        // Check global variables, excluding extern declarations
        if self.extern_names.contains(name) {
            return false;
        }
        self.globals.iter().any(|(_, n, _)| n == name)
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
        // Constant folding: 1 + 2 = 3
        assert_eq!(prog.functions[0].body[0], Stmt::Return(Some(Expr::Num(3))));
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
        assert_eq!(prog.globals, vec![(Type::int_type(), "g".to_string(), None)]);
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
