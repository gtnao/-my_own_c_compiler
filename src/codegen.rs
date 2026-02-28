use crate::ast::{BinOp, Expr, Function, Program, Stmt, UnaryOp};
use crate::types::{Type, TypeKind};
use std::collections::{HashMap, HashSet};

pub struct Codegen {
    output: String,
    locals: HashMap<String, usize>,
    local_types: HashMap<String, Type>,
    stack_size: usize,
    label_count: usize,
    break_labels: Vec<String>,
    continue_labels: Vec<String>,
    goto_labels: HashMap<String, String>,
    current_func_name: String,
    stack_depth: usize,
    globals: HashSet<String>,
    global_types: HashMap<String, Type>,
    string_literals: Vec<Vec<u8>>,
    va_save_area_offset: usize,
    current_func_param_count: usize,
    filename: String,
}

impl Codegen {
    pub fn new(filename: &str) -> Self {
        Self {
            output: String::new(),
            locals: HashMap::new(),
            local_types: HashMap::new(),
            stack_size: 0,
            label_count: 0,
            break_labels: Vec::new(),
            continue_labels: Vec::new(),
            goto_labels: HashMap::new(),
            current_func_name: String::new(),
            stack_depth: 0,
            globals: HashSet::new(),
            global_types: HashMap::new(),
            string_literals: Vec::new(),
            va_save_area_offset: 0,
            current_func_param_count: 0,
            filename: filename.to_string(),
        }
    }

    fn new_label(&mut self) -> String {
        let label = format!(".L{}", self.label_count);
        self.label_count += 1;
        label
    }

    pub fn generate(&mut self, program: &Program) -> String {
        // Emit debug file directive
        self.emit(&format!("  .file \"{}\"", self.filename));

        // Register global variable names and types
        for (ty, name, _) in &program.globals {
            self.globals.insert(name.clone());
            self.global_types.insert(name.clone(), ty.clone());
        }

        // Emit global variable declarations (skip extern without definition)
        let mut emitted_globals = std::collections::HashSet::new();
        for (ty, name, init) in &program.globals {
            if emitted_globals.contains(name) {
                continue;
            }
            if init.is_none() && program.extern_names.contains(name) {
                continue;
            }
            emitted_globals.insert(name.clone());
            if let Some(bytes) = init {
                // Initialized global: .data section
                self.emit("  .data");
                let align = ty.align();
                self.emit(&format!("  .align {}", align));
                self.emit(&format!("  .globl {}", name));
                self.emit(&format!("{}:", name));
                let byte_strs: Vec<String> = bytes.iter().map(|b| format!("{}", b)).collect();
                self.emit(&format!("  .byte {}", byte_strs.join(",")));
                self.emit("  .text");
            } else {
                // Uninitialized global: .bss (via .comm)
                let size = ty.size();
                let align = ty.align();
                self.emit(&format!("  .comm {}, {}, {}", name, size, align));
            }
        }

        // Generate code for functions
        for func in &program.functions {
            self.gen_function(func);
        }

        // Emit string literals in .rodata section
        let strings = self.string_literals.clone();
        if !strings.is_empty() {
            self.emit("  .section .rodata");
            for (i, s) in strings.iter().enumerate() {
                self.emit(&format!(".LC{}:", i));
                let mut bytes: Vec<String> = s.iter().map(|b| format!("{}", b)).collect();
                bytes.push("0".to_string()); // null terminator
                self.emit(&format!("  .byte {}", bytes.join(",")));
            }
        }

        self.peephole_optimize();
        self.output.clone()
    }

    /// Peephole optimization pass on the generated assembly.
    fn peephole_optimize(&mut self) {
        let lines: Vec<&str> = self.output.lines().collect();
        let mut result = Vec::new();
        let mut i = 0;
        while i < lines.len() {
            if i + 1 < lines.len() {
                let cur = lines[i].trim();
                let next = lines[i + 1].trim();

                // Pattern 1: push %rax; pop %rax → remove both
                if cur == "push %rax" && next == "pop %rax" {
                    i += 2;
                    continue;
                }

                // Pattern 2: push %rax; pop %reg → mov %rax, %reg
                if cur == "push %rax" {
                    if let Some(reg) = next.strip_prefix("pop ") {
                        result.push(format!("  mov %rax, {}", reg));
                        i += 2;
                        continue;
                    }
                }

                // Pattern 3: mov X, %rax; push %rax → push X (only for simple operands)
                // Skip this for safety — could interfere with addressing modes
            }
            result.push(lines[i].to_string());
            i += 1;
        }
        self.output = result.join("\n");
        self.output.push('\n');
    }

    fn gen_function(&mut self, func: &Function) {
        self.current_func_name = func.name.clone();
        self.stack_depth = 0;
        self.current_func_param_count = func.params.len();

        // Set up local variable offsets on stack using type sizes
        self.locals.clear();
        self.local_types.clear();
        self.goto_labels.clear();
        let mut offset = 0;
        for (ty, name) in &func.locals {
            let size = ty.size();
            let align = ty.align();
            // Align offset before placing variable
            offset = (offset + align - 1) & !(align - 1);
            offset += size;
            self.locals.insert(name.clone(), offset);
            self.local_types.insert(name.clone(), ty.clone());
        }

        // For variadic functions, allocate register save area (6 * 8 = 48 bytes)
        self.va_save_area_offset = 0;
        if func.is_variadic {
            offset = (offset + 7) & !7; // 8-byte align
            offset += 48; // 6 registers * 8 bytes
            self.va_save_area_offset = offset;
        }

        self.stack_size = offset;
        // Align stack to 16 bytes
        if self.stack_size % 16 != 0 {
            self.stack_size = (self.stack_size + 15) & !15;
        }

        self.emit(&format!("  .globl {}", func.name));
        self.emit(&format!("{}:", func.name));
        self.emit("  push %rbp");
        self.emit("  mov %rsp, %rbp");
        if self.stack_size > 0 {
            self.emit(&format!("  sub ${}, %rsp", self.stack_size));
        }

        // Store register parameters to stack (first 6)
        let arg_regs_64 = ["%rdi", "%rsi", "%rdx", "%rcx", "%r8", "%r9"];
        let arg_regs_32 = ["%edi", "%esi", "%edx", "%ecx", "%r8d", "%r9d"];
        let arg_regs_16 = ["%di", "%si", "%dx", "%cx", "%r8w", "%r9w"];
        let arg_regs_8 = ["%dil", "%sil", "%dl", "%cl", "%r8b", "%r9b"];
        for (i, (ty, param)) in func.params.iter().enumerate().take(6) {
            let offset = self.locals[param];
            match ty.kind {
                TypeKind::Bool | TypeKind::Char => {
                    self.emit(&format!("  movb {}, -{}(%rbp)", arg_regs_8[i], offset));
                }
                TypeKind::Short => {
                    self.emit(&format!("  movw {}, -{}(%rbp)", arg_regs_16[i], offset));
                }
                TypeKind::Int => {
                    self.emit(&format!("  movl {}, -{}(%rbp)", arg_regs_32[i], offset));
                }
                TypeKind::Long | TypeKind::Ptr(_) => {
                    self.emit(&format!("  mov {}, -{}(%rbp)", arg_regs_64[i], offset));
                }
                TypeKind::Array(_, _) => {
                    // Array params treated as pointers (8 bytes)
                    self.emit(&format!("  mov {}, -{}(%rbp)", arg_regs_64[i], offset));
                }
                TypeKind::Float => {
                    // Float args passed in integer registers (simplified ABI)
                    self.emit(&format!("  movl {}, -{}(%rbp)", arg_regs_32[i], offset));
                }
                TypeKind::Double => {
                    self.emit(&format!("  mov {}, -{}(%rbp)", arg_regs_64[i], offset));
                }
                TypeKind::Struct(_) => {
                    // Struct pass-by-value: register holds pointer to caller's struct,
                    // copy the struct data into local stack space
                    let size = ty.size();
                    self.emit(&format!("  mov {}, %rsi", arg_regs_64[i])); // src = caller's struct address
                    self.emit(&format!("  lea -{}(%rbp), %rdi", offset)); // dst = local struct space
                    self.emit(&format!("  mov ${}, %rcx", size));
                    self.emit("  rep movsb");
                }
                TypeKind::Void => {}
            }
        }
        // Copy stack parameters to local slots (7th and beyond)
        for (i, (_ty, param)) in func.params.iter().enumerate().skip(6) {
            let src_offset = 16 + (i - 6) * 8;
            self.emit(&format!("  mov {}(%rbp), %rax", src_offset));
            self.emit_store_var(param);
        }

        // For variadic functions, save all 6 register args to contiguous save area
        if func.is_variadic {
            let base = self.va_save_area_offset;
            for (i, reg) in arg_regs_64.iter().enumerate() {
                // Save area: slot 0 at -(base), slot 1 at -(base-8), etc.
                self.emit(&format!("  mov {}, -{}(%rbp)", reg, base - i * 8));
            }
        }

        for stmt in &func.body {
            self.gen_stmt(stmt);
        }

        // Default return 0 if no return statement reached
        self.emit("  mov $0, %rax");
        self.emit(&format!(".Lreturn.{}:", func.name));
        self.emit("  mov %rbp, %rsp");
        self.emit("  pop %rbp");
        self.emit("  ret");
    }

    fn gen_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Return(expr) => {
                if let Some(e) = expr {
                    self.gen_expr(e);
                }
                let func_name = self.current_func_name.clone();
                self.emit(&format!("  jmp .Lreturn.{}", func_name));
            }
            Stmt::ExprStmt(expr) => {
                self.gen_expr(expr);
            }
            Stmt::Block(stmts) => {
                for s in stmts {
                    self.gen_stmt(s);
                }
            }
            Stmt::If { cond, then_stmt, else_stmt } => {
                let else_label = self.new_label();
                let end_label = self.new_label();

                self.gen_expr(cond);
                self.emit("  cmp $0, %rax");
                self.emit(&format!("  je {}", else_label));
                self.gen_stmt(then_stmt);
                self.emit(&format!("  jmp {}", end_label));
                self.emit(&format!("{}:", else_label));
                if let Some(else_s) = else_stmt {
                    self.gen_stmt(else_s);
                }
                self.emit(&format!("{}:", end_label));
            }
            Stmt::While { cond, body } => {
                let begin_label = self.new_label();
                let end_label = self.new_label();

                self.break_labels.push(end_label.clone());
                self.continue_labels.push(begin_label.clone());
                self.emit(&format!("{}:", begin_label));
                self.gen_expr(cond);
                self.emit("  cmp $0, %rax");
                self.emit(&format!("  je {}", end_label));
                self.gen_stmt(body);
                self.emit(&format!("  jmp {}", begin_label));
                self.emit(&format!("{}:", end_label));
                self.continue_labels.pop();
                self.break_labels.pop();
            }
            Stmt::DoWhile { body, cond } => {
                let begin_label = self.new_label();
                let continue_label = self.new_label();
                let end_label = self.new_label();

                self.break_labels.push(end_label.clone());
                self.continue_labels.push(continue_label.clone());
                self.emit(&format!("{}:", begin_label));
                self.gen_stmt(body);
                self.emit(&format!("{}:", continue_label));
                self.gen_expr(cond);
                self.emit("  cmp $0, %rax");
                self.emit(&format!("  jne {}", begin_label));
                self.emit(&format!("{}:", end_label));
                self.continue_labels.pop();
                self.break_labels.pop();
            }
            Stmt::Switch { cond, cases, default } => {
                let end_label = self.new_label();
                self.break_labels.push(end_label.clone());

                self.gen_expr(cond);

                // Generate comparisons and jumps to case labels
                let mut case_labels = Vec::new();
                for (val, _) in cases {
                    let label = self.new_label();
                    self.emit(&format!("  cmp ${}, %rax", val));
                    self.emit(&format!("  je {}", label));
                    case_labels.push(label);
                }

                // Jump to default or end
                let default_label = if default.is_some() {
                    let label = self.new_label();
                    self.emit(&format!("  jmp {}", label));
                    Some(label)
                } else {
                    self.emit(&format!("  jmp {}", end_label));
                    None
                };

                // Generate case bodies
                for (i, (_, stmts)) in cases.iter().enumerate() {
                    self.emit(&format!("{}:", case_labels[i]));
                    for s in stmts {
                        self.gen_stmt(s);
                    }
                }

                // Generate default body
                if let Some(stmts) = default {
                    if let Some(label) = default_label {
                        self.emit(&format!("{}:", label));
                    }
                    for s in stmts {
                        self.gen_stmt(s);
                    }
                }

                self.emit(&format!("{}:", end_label));
                self.break_labels.pop();
            }
            Stmt::Break => {
                if let Some(label) = self.break_labels.last() {
                    self.emit(&format!("  jmp {}", label));
                }
            }
            Stmt::Continue => {
                if let Some(label) = self.continue_labels.last() {
                    self.emit(&format!("  jmp {}", label));
                }
            }
            Stmt::Goto(name) => {
                let label = self.get_or_create_goto_label(name);
                self.emit(&format!("  jmp {}", label));
            }
            Stmt::GotoExpr(expr) => {
                // Computed goto: goto *expr — jump to address in expr
                self.gen_expr(expr);
                self.emit("  jmp *%rax");
            }
            Stmt::Label { name, stmt } => {
                let label = self.get_or_create_goto_label(name);
                self.emit(&format!("{}:", label));
                self.gen_stmt(stmt);
            }
            Stmt::For { init, cond, inc, body } => {
                let begin_label = self.new_label();
                let continue_label = self.new_label();
                let end_label = self.new_label();

                self.break_labels.push(end_label.clone());
                self.continue_labels.push(continue_label.clone());
                if let Some(init_stmt) = init {
                    self.gen_stmt(init_stmt);
                }
                self.emit(&format!("{}:", begin_label));
                if let Some(cond_expr) = cond {
                    self.gen_expr(cond_expr);
                    self.emit("  cmp $0, %rax");
                    self.emit(&format!("  je {}", end_label));
                }
                self.gen_stmt(body);
                self.emit(&format!("{}:", continue_label));
                if let Some(inc_expr) = inc {
                    self.gen_expr(inc_expr);
                }
                self.emit(&format!("  jmp {}", begin_label));
                self.emit(&format!("{}:", end_label));
                self.continue_labels.pop();
                self.break_labels.pop();
            }
            Stmt::VarDecl { name, ty: _, init } => {
                if let Some(expr) = init {
                    let var_ty = self.get_var_type(name);
                    let expr_ty = self.expr_type(expr);
                    self.gen_expr(expr);
                    // Type conversion for float/int mismatch
                    let var_is_float = Self::is_float_type(&var_ty);
                    let expr_is_float = Self::is_float_type(&expr_ty);
                    if var_is_float != expr_is_float || (var_is_float && expr_is_float && var_ty.kind != expr_ty.kind) {
                        self.emit_type_convert(&expr_ty, &var_ty);
                    }
                    self.emit_store_var(name);
                }
            }
        }
    }

    fn gen_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Num(val) => {
                self.emit(&format!("  mov ${}, %rax", val));
            }
            Expr::FloatLit(val) => {
                // Load double literal via integer register
                let bits = val.to_bits();
                self.emit(&format!("  movabs ${}, %rax", bits as i64));
                self.emit("  movq %rax, %xmm0");
            }
            Expr::Var(name) => {
                self.emit_load_var(name);
            }
            Expr::Assign { lhs, rhs } => {
                let lhs_ty = self.expr_type(lhs);
                if let TypeKind::Struct(_) = &lhs_ty.kind {
                    // Struct assignment: get addresses of both sides and memcpy
                    self.gen_addr(rhs);
                    self.push(); // save rhs address
                    self.gen_addr(lhs);
                    self.emit("  mov %rax, %rdi"); // dst address
                    self.pop("%rsi"); // src address
                    self.emit(&format!("  mov ${}, %rcx", lhs_ty.size()));
                    self.emit("  rep movsb");
                    // Leave dst address in %rax for chained assignment
                    self.gen_addr(lhs);
                } else {
                    // Check for bit-field assignment
                    let bf_info = if let Expr::Member(base, name) = lhs.as_ref() {
                        let base_ty = self.expr_type(base);
                        if let TypeKind::Struct(members) = &base_ty.kind {
                            members.iter().find(|m| m.name == *name)
                                .filter(|m| m.bit_width > 0)
                                .map(|m| (m.bit_width, m.bit_offset))
                        } else {
                            None
                        }
                    } else {
                        None
                    };

                    if let Some((bit_width, bit_off)) = bf_info {
                        // Bit-field assignment: read-modify-write
                        self.gen_expr(rhs);
                        let mask = (1u64 << bit_width) - 1;
                        self.emit(&format!("  and ${}, %rax", mask)); // mask new value
                        if bit_off > 0 {
                            self.emit(&format!("  shl ${}, %rax", bit_off)); // shift to position
                        }
                        self.push(); // save shifted new value
                        self.gen_addr(lhs);
                        self.emit("  mov %rax, %rdi"); // address of storage unit
                        let ty = self.expr_type(lhs);
                        // Load current storage unit value
                        self.emit_load_indirect(&ty);
                        let clear_mask = !((mask) << bit_off) as i64;
                        self.emit(&format!("  mov ${}, %rcx", clear_mask));
                        self.emit("  and %rcx, %rax"); // clear old bits
                        self.pop("%rcx"); // get shifted new value
                        self.emit("  or %rcx, %rax"); // set new bits
                        // Store back
                        self.emit_store_indirect(&ty);
                    } else {
                        let rhs_ty = self.expr_type(rhs);
                        self.gen_expr(rhs);
                        // Type conversion between int and float
                        let lhs_is_float = Self::is_float_type(&lhs_ty);
                        let rhs_is_float = Self::is_float_type(&rhs_ty);
                        if lhs_is_float && !rhs_is_float {
                            // int -> float/double: convert %rax to %xmm0
                            self.emit_type_convert(&rhs_ty, &lhs_ty);
                        } else if !lhs_is_float && rhs_is_float {
                            // float/double -> int: convert %xmm0 to %rax
                            self.emit_type_convert(&rhs_ty, &lhs_ty);
                        } else if lhs_is_float && rhs_is_float {
                            // float <-> double conversion
                            self.emit_type_convert(&rhs_ty, &lhs_ty);
                        }
                        match lhs.as_ref() {
                            Expr::Var(name) => {
                                self.emit_store_var(name);
                            }
                            Expr::Deref(_) | Expr::Member(_, _) => {
                                if lhs_is_float {
                                    self.push_float();
                                    self.gen_addr(lhs);
                                    self.emit("  mov %rax, %rdi");
                                    self.pop_float("%xmm0");
                                    let ty = self.expr_type(lhs);
                                    self.emit_store_indirect(&ty);
                                } else {
                                    self.push(); // save rhs value
                                    self.gen_addr(lhs);
                                    self.emit("  mov %rax, %rdi"); // address in %rdi
                                    self.pop("%rax"); // value in %rax
                                    let ty = self.expr_type(lhs);
                                    self.emit_store_indirect(&ty);
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            Expr::Addr(inner) => {
                self.gen_addr(inner);
            }
            Expr::Deref(inner) => {
                self.gen_expr(inner);
                let ty = self.expr_type(expr);
                if let TypeKind::Struct(_) = &ty.kind {
                    // Struct deref: leave address in %rax
                } else {
                    self.emit_load_indirect(&ty);
                }
            }
            Expr::UnaryOp { op, operand } => {
                self.gen_expr(operand);
                match op {
                    UnaryOp::Neg => {
                        self.emit("  neg %rax");
                    }
                    UnaryOp::LogicalNot => {
                        self.emit("  cmp $0, %rax");
                        self.emit("  sete %al");
                        self.emit("  movzb %al, %rax");
                    }
                    UnaryOp::BitNot => {
                        self.emit("  not %rax");
                    }
                }
            }
            Expr::Comma(lhs, rhs) => {
                self.gen_expr(lhs);
                self.gen_expr(rhs);
            }
            Expr::Ternary { cond, then_expr, else_expr } => {
                let else_label = self.new_label();
                let end_label = self.new_label();

                self.gen_expr(cond);
                self.emit("  cmp $0, %rax");
                self.emit(&format!("  je {}", else_label));
                self.gen_expr(then_expr);
                self.emit(&format!("  jmp {}", end_label));
                self.emit(&format!("{}:", else_label));
                self.gen_expr(else_expr);
                self.emit(&format!("{}:", end_label));
            }
            Expr::LogicalAnd(lhs, rhs) => {
                let false_label = self.new_label();
                let end_label = self.new_label();

                self.gen_expr(lhs);
                self.emit("  cmp $0, %rax");
                self.emit(&format!("  je {}", false_label));
                self.gen_expr(rhs);
                self.emit("  cmp $0, %rax");
                self.emit(&format!("  je {}", false_label));
                self.emit("  mov $1, %rax");
                self.emit(&format!("  jmp {}", end_label));
                self.emit(&format!("{}:", false_label));
                self.emit("  mov $0, %rax");
                self.emit(&format!("{}:", end_label));
            }
            Expr::LogicalOr(lhs, rhs) => {
                let true_label = self.new_label();
                let end_label = self.new_label();

                self.gen_expr(lhs);
                self.emit("  cmp $0, %rax");
                self.emit(&format!("  jne {}", true_label));
                self.gen_expr(rhs);
                self.emit("  cmp $0, %rax");
                self.emit(&format!("  jne {}", true_label));
                self.emit("  mov $0, %rax");
                self.emit(&format!("  jmp {}", end_label));
                self.emit(&format!("{}:", true_label));
                self.emit("  mov $1, %rax");
                self.emit(&format!("{}:", end_label));
            }
            Expr::PreInc(operand) => {
                // addr → load → add 1 → store → result = new value
                self.gen_addr(operand);
                self.push(); // save addr
                let ty = self.expr_type(operand);
                self.emit_load_indirect(&ty);
                self.emit("  add $1, %rax");
                self.pop("%rdi"); // addr in %rdi
                self.emit_store_indirect(&ty); // store new value
                // %rax still has new value
            }
            Expr::PreDec(operand) => {
                self.gen_addr(operand);
                self.push();
                let ty = self.expr_type(operand);
                self.emit_load_indirect(&ty);
                self.emit("  sub $1, %rax");
                self.pop("%rdi");
                self.emit_store_indirect(&ty);
            }
            Expr::PostInc(operand) => {
                // addr → load old → save old → inc → store new → return old
                self.gen_addr(operand);
                self.push(); // save addr (%rax still = addr)
                let ty = self.expr_type(operand);
                self.emit_load_indirect(&ty); // old value in %rax
                self.emit("  mov %rax, %rcx"); // old value in %rcx
                self.emit("  add $1, %rax"); // new value in %rax
                self.pop("%rdi"); // addr in %rdi
                self.emit_store_indirect(&ty); // store new value
                self.emit("  mov %rcx, %rax"); // return old value
            }
            Expr::PostDec(operand) => {
                self.gen_addr(operand);
                self.push();
                let ty = self.expr_type(operand);
                self.emit_load_indirect(&ty);
                self.emit("  mov %rax, %rcx"); // old value
                self.emit("  sub $1, %rax"); // new value
                self.pop("%rdi");
                self.emit_store_indirect(&ty);
                self.emit("  mov %rcx, %rax"); // return old value
            }
            Expr::FuncCall { name, args } => {
                self.gen_call(args, |codegen| {
                    codegen.emit(&format!("  call {}", name));
                });
            }
            Expr::FuncPtrCall { fptr, args } => {
                // Evaluate function pointer first
                self.gen_expr(fptr);
                // Save to %r10 (caller-saved, not used by argument passing)
                self.emit("  mov %rax, %r10");

                let num_stack_args = if args.len() > 6 { args.len() - 6 } else { 0 };
                let needs_align = (self.stack_depth + num_stack_args) % 2 != 0;
                if needs_align {
                    self.emit("  sub $8, %rsp");
                    self.stack_depth += 1;
                }

                // Push stack arguments (7th and beyond) in reverse order
                for i in (6..args.len()).rev() {
                    self.gen_expr(&args[i]);
                    self.push();
                }

                // Evaluate first 6 register arguments
                let reg_count = std::cmp::min(args.len(), 6);
                for i in 0..reg_count {
                    self.gen_expr(&args[i]);
                    self.push();
                }
                let arg_regs = ["%rdi", "%rsi", "%rdx", "%rcx", "%r8", "%r9"];
                for i in (0..reg_count).rev() {
                    self.pop(arg_regs[i]);
                }

                self.emit("  mov $0, %al");
                self.emit("  call *%r10");

                if num_stack_args > 0 {
                    self.emit(&format!("  add ${}, %rsp", num_stack_args * 8));
                    self.stack_depth -= num_stack_args;
                }
                if needs_align {
                    self.emit("  add $8, %rsp");
                    self.stack_depth -= 1;
                }
            }
            Expr::SizeofType(ty) => {
                self.emit(&format!("  mov ${}, %rax", ty.size()));
            }
            Expr::SizeofExpr(expr) => {
                let ty = self.expr_type(expr);
                self.emit(&format!("  mov ${}, %rax", ty.size()));
            }
            Expr::Member(base, name) => {
                // Check if this is a bit-field member
                let base_ty = self.expr_type(base);
                let bf_info = if let TypeKind::Struct(members) = &base_ty.kind {
                    members.iter().find(|m| m.name == *name)
                        .filter(|m| m.bit_width > 0)
                        .map(|m| (m.bit_width, m.bit_offset))
                } else {
                    None
                };

                self.gen_addr(expr);
                let ty = self.expr_type(expr);
                if let TypeKind::Struct(_) = &ty.kind {
                    // Struct member: leave address in %rax (like array decay)
                } else if let Some((bit_width, bit_off)) = bf_info {
                    // Bit-field: load storage unit, extract bits
                    self.emit_load_indirect(&ty);
                    if bit_off > 0 {
                        self.emit(&format!("  shr ${}, %rax", bit_off));
                    }
                    let mask = (1u64 << bit_width) - 1;
                    self.emit(&format!("  and ${}, %rax", mask));
                } else {
                    self.emit_load_indirect(&ty);
                }
            }
            Expr::StrLit(s) => {
                let idx = self.string_literals.len();
                self.string_literals.push(s.clone());
                self.emit(&format!("  lea .LC{}(%rip), %rax", idx));
            }
            Expr::Cast { ty, expr } => {
                let src_ty = self.expr_type(expr);
                self.gen_expr(expr);
                // Handle float/int conversions
                let src_float = Self::is_float_type(&src_ty);
                let dst_float = Self::is_float_type(ty);
                if src_float || dst_float {
                    self.emit_type_convert(&src_ty, ty);
                    // If converting float to int, also apply integer truncation
                    if src_float && !dst_float {
                        match ty.kind {
                            TypeKind::Bool => {
                                self.emit("  cmp $0, %rax");
                                self.emit("  setne %al");
                                self.emit("  movzbl %al, %eax");
                            }
                            TypeKind::Char if ty.is_unsigned => self.emit("  movzbl %al, %eax"),
                            TypeKind::Char => self.emit("  movsbq %al, %rax"),
                            TypeKind::Short if ty.is_unsigned => self.emit("  movzwl %ax, %eax"),
                            TypeKind::Short => self.emit("  movswq %ax, %rax"),
                            TypeKind::Int if ty.is_unsigned => self.emit("  movl %eax, %eax"),
                            TypeKind::Int => self.emit("  movslq %eax, %rax"),
                            _ => {}
                        }
                    }
                } else {
                    // Integer-to-integer cast (existing code)
                    match ty.kind {
                        TypeKind::Bool => {
                            self.emit("  cmp $0, %rax");
                            self.emit("  setne %al");
                            self.emit("  movzbl %al, %eax");
                        }
                        TypeKind::Char if ty.is_unsigned => self.emit("  movzbl %al, %eax"),
                        TypeKind::Char => self.emit("  movsbq %al, %rax"),
                        TypeKind::Short if ty.is_unsigned => self.emit("  movzwl %ax, %eax"),
                        TypeKind::Short => self.emit("  movswq %ax, %rax"),
                        TypeKind::Int if ty.is_unsigned => self.emit("  movl %eax, %eax"),
                        TypeKind::Int => self.emit("  movslq %eax, %rax"),
                        TypeKind::Long | TypeKind::Void | TypeKind::Ptr(_) | TypeKind::Array(_, _) | TypeKind::Struct(_) | TypeKind::Float | TypeKind::Double => {}
                    }
                }
            }
            Expr::VaStart { ap, last_param: _ } => {
                // Compute address of first unnamed arg in register save area.
                // Register save area layout (highest offset = reg 0):
                //   -(va_save_area_offset)(%rbp) = reg 0 (rdi)
                //   -(va_save_area_offset - 8)(%rbp) = reg 1 (rsi)
                //   ...
                // Find the index of last_param among function params
                let param_idx = self.current_func_param_count;
                // First unnamed arg is at index param_idx in the save area
                let base = self.va_save_area_offset;
                let first_unnamed_offset = base - param_idx * 8;
                self.emit(&format!("  lea -{}(%rbp), %rax", first_unnamed_offset));
                // Store the address into ap variable
                self.push(); // save computed address
                self.gen_addr(ap); // get address of ap
                self.emit("  mov %rax, %rdi"); // %rdi = address of ap
                self.pop("%rax"); // %rax = computed address
                self.emit("  mov %rax, (%rdi)"); // *ap = address
            }
            Expr::VaArg { ap, ty } => {
                // 1. Get address of ap variable
                self.gen_addr(ap);
                self.emit("  mov %rax, %rcx"); // %rcx = &ap
                // 2. Load current ap value (pointer)
                self.emit("  mov (%rcx), %rdi"); // %rdi = ap (current pointer)
                // 3. Load value from current pointer based on requested type
                match ty.kind {
                    TypeKind::Bool | TypeKind::Char if ty.is_unsigned => {
                        self.emit("  movzbl (%rdi), %eax");
                    }
                    TypeKind::Char => {
                        self.emit("  movsbl (%rdi), %eax");
                    }
                    TypeKind::Short if ty.is_unsigned => {
                        self.emit("  movzwl (%rdi), %eax");
                    }
                    TypeKind::Short => {
                        self.emit("  movswl (%rdi), %eax");
                    }
                    TypeKind::Int if ty.is_unsigned => {
                        self.emit("  movl (%rdi), %eax");
                    }
                    TypeKind::Int => {
                        self.emit("  movslq (%rdi), %rax");
                    }
                    _ => {
                        self.emit("  mov (%rdi), %rax");
                    }
                }
                self.push(); // save loaded value
                // 4. Advance ap by 8 bytes (ascending: next arg at higher address)
                self.emit("  add $8, %rdi");
                self.emit("  mov %rdi, (%rcx)"); // store updated ap
                self.pop("%rax"); // restore loaded value
            }
            Expr::BinOp { op, lhs, rhs } => {
                // Check if this is a float/double operation
                let lhs_ty = self.expr_type(lhs);
                let rhs_ty = self.expr_type(rhs);
                if Self::is_float_type(&lhs_ty) || Self::is_float_type(&rhs_ty) {
                    self.gen_float_binop(op, lhs, rhs);
                    return;
                }

                self.gen_expr(rhs);
                self.push();
                self.gen_expr(lhs);
                self.pop("%rdi");

                // After evaluation: %rax = lhs, %rdi = rhs
                let lhs_ty = self.expr_type(lhs);
                let rhs_ty = self.expr_type(rhs);

                match op {
                    BinOp::Add => {
                        if lhs_ty.is_pointer() {
                            // ptr + int: scale rhs by sizeof(*ptr)
                            let size = lhs_ty.base_type().unwrap().size();
                            if size > 1 {
                                self.emit(&format!("  imul ${}, %rdi", size));
                            }
                        } else if rhs_ty.is_pointer() {
                            // int + ptr: scale lhs by sizeof(*ptr)
                            let size = rhs_ty.base_type().unwrap().size();
                            if size > 1 {
                                self.emit(&format!("  imul ${}, %rax", size));
                            }
                        }
                        self.emit("  add %rdi, %rax");
                    }
                    BinOp::Sub => {
                        if lhs_ty.is_pointer() && rhs_ty.is_pointer() {
                            // ptr - ptr: result is element count
                            self.emit("  sub %rdi, %rax");
                            let size = lhs_ty.base_type().unwrap().size();
                            if size > 1 {
                                self.emit(&format!("  mov ${}, %rdi", size));
                                self.emit("  cqto");
                                self.emit("  idiv %rdi");
                            }
                        } else if lhs_ty.is_pointer() {
                            // ptr - int: scale rhs by sizeof(*ptr)
                            let size = lhs_ty.base_type().unwrap().size();
                            if size > 1 {
                                self.emit(&format!("  imul ${}, %rdi", size));
                            }
                            self.emit("  sub %rdi, %rax");
                        } else {
                            self.emit("  sub %rdi, %rax");
                        }
                    }
                    BinOp::Mul => {
                        self.emit("  imul %rdi, %rax");
                    }
                    BinOp::Div => {
                        self.emit("  cqto");
                        self.emit("  idiv %rdi");
                    }
                    BinOp::Mod => {
                        self.emit("  cqto");
                        self.emit("  idiv %rdi");
                        self.emit("  mov %rdx, %rax");
                    }
                    BinOp::Eq => {
                        self.emit("  cmp %rdi, %rax");
                        self.emit("  sete %al");
                        self.emit("  movzb %al, %rax");
                    }
                    BinOp::Ne => {
                        self.emit("  cmp %rdi, %rax");
                        self.emit("  setne %al");
                        self.emit("  movzb %al, %rax");
                    }
                    BinOp::Lt => {
                        self.emit("  cmp %rdi, %rax");
                        self.emit("  setl %al");
                        self.emit("  movzb %al, %rax");
                    }
                    BinOp::Le => {
                        self.emit("  cmp %rdi, %rax");
                        self.emit("  setle %al");
                        self.emit("  movzb %al, %rax");
                    }
                    BinOp::Gt => {
                        self.emit("  cmp %rdi, %rax");
                        self.emit("  setg %al");
                        self.emit("  movzb %al, %rax");
                    }
                    BinOp::Ge => {
                        self.emit("  cmp %rdi, %rax");
                        self.emit("  setge %al");
                        self.emit("  movzb %al, %rax");
                    }
                    BinOp::BitAnd => {
                        self.emit("  and %rdi, %rax");
                    }
                    BinOp::BitOr => {
                        self.emit("  or %rdi, %rax");
                    }
                    BinOp::BitXor => {
                        self.emit("  xor %rdi, %rax");
                    }
                    BinOp::Shl => {
                        self.emit("  mov %rdi, %rcx");
                        self.emit("  sal %cl, %rax");
                    }
                    BinOp::Shr => {
                        self.emit("  mov %rdi, %rcx");
                        self.emit("  sar %cl, %rax");
                    }
                }
            }
            Expr::StmtExpr(stmts) => {
                // Evaluate all statements; the last expression statement's
                // value remains in %rax
                for s in stmts {
                    self.gen_stmt(s);
                }
            }
            Expr::LabelAddr(label) => {
                // &&label — load address of label into %rax
                let asm_label = self.get_or_create_goto_label(label);
                self.emit(&format!("  lea {}(%rip), %rax", asm_label));
            }
        }
    }

    /// Compute the address of an lvalue expression into %rax.
    fn gen_addr(&mut self, expr: &Expr) {
        match expr {
            Expr::Var(name) => {
                if self.globals.contains(name) {
                    self.emit(&format!("  lea {}(%rip), %rax", name));
                } else {
                    let offset = self.locals[name];
                    self.emit(&format!("  lea -{}(%rbp), %rax", offset));
                }
            }
            Expr::Deref(inner) => {
                // Address of *p is just the value of p
                self.gen_expr(inner);
            }
            Expr::Member(base, name) => {
                self.gen_addr(base);
                let base_ty = self.expr_type(base);
                if let TypeKind::Struct(members) = &base_ty.kind {
                    let member = members.iter().find(|m| m.name == *name).unwrap();
                    if member.offset > 0 {
                        self.emit(&format!("  add ${}, %rax", member.offset));
                    }
                }
            }
            Expr::Comma(lhs, rhs) => {
                // Evaluate lhs for side effects, then get address of rhs
                self.gen_expr(lhs);
                self.gen_addr(rhs);
            }
            _ => {}
        }
    }

    /// Infer the type of an expression (best effort).
    fn expr_type(&self, expr: &Expr) -> Type {
        match expr {
            Expr::FloatLit(_) => Type::double_type(),
            Expr::Var(name) => self.get_var_type(name),
            Expr::Deref(inner) => {
                let inner_ty = self.expr_type(inner);
                match inner_ty.kind {
                    TypeKind::Ptr(base) | TypeKind::Array(base, _) => *base,
                    _ => Type::long_type(),
                }
            }
            Expr::Addr(inner) => {
                let inner_ty = self.expr_type(inner);
                Type::ptr_to(inner_ty)
            }
            Expr::StrLit(_) => Type::ptr_to(Type::char_type()),
            Expr::Member(base, name) => {
                let base_ty = self.expr_type(base);
                if let TypeKind::Struct(members) = &base_ty.kind {
                    members.iter().find(|m| m.name == *name)
                        .map(|m| m.ty.clone())
                        .unwrap_or(Type::int_type())
                } else {
                    Type::int_type()
                }
            }
            Expr::BinOp { op, lhs, rhs } => {
                let lhs_ty = self.expr_type(lhs);
                let rhs_ty = self.expr_type(rhs);
                // Comparison operators always return int
                match op {
                    BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                        return Type::int_type();
                    }
                    _ => {}
                }
                // Float/double type promotion
                if Self::is_float_type(&lhs_ty) || Self::is_float_type(&rhs_ty) {
                    if matches!(lhs_ty.kind, TypeKind::Double) || matches!(rhs_ty.kind, TypeKind::Double) {
                        return Type::double_type();
                    }
                    return Type::float_type();
                }
                match op {
                    BinOp::Add => {
                        if lhs_ty.is_pointer() {
                            Type::ptr_to(lhs_ty.base_type().unwrap().clone())
                        } else if rhs_ty.is_pointer() {
                            Type::ptr_to(rhs_ty.base_type().unwrap().clone())
                        } else {
                            Type::long_type()
                        }
                    }
                    BinOp::Sub => {
                        if lhs_ty.is_pointer() && rhs_ty.is_pointer() {
                            Type::long_type()
                        } else if lhs_ty.is_pointer() {
                            Type::ptr_to(lhs_ty.base_type().unwrap().clone())
                        } else {
                            Type::long_type()
                        }
                    }
                    _ => Type::long_type(),
                }
            }
            Expr::Comma(_, rhs) => self.expr_type(rhs),
            Expr::Cast { ty, .. } => ty.clone(),
            Expr::VaArg { ty, .. } => ty.clone(),
            Expr::VaStart { .. } => Type::void(),
            Expr::FuncPtrCall { .. } => Type::long_type(),
            Expr::StmtExpr(stmts) => {
                // Type of statement expression is the type of the last expression statement
                if let Some(last) = stmts.last() {
                    if let Stmt::ExprStmt(expr) = last {
                        return self.expr_type(expr);
                    }
                }
                Type::int_type()
            }
            _ => Type::long_type(),
        }
    }

    /// Generate a function call with args setup, alignment, and cleanup.
    /// The `emit_call` closure should emit the actual call instruction.
    fn gen_call<F>(&mut self, args: &[Expr], emit_call: F)
    where
        F: FnOnce(&mut Self),
    {
        let num_stack_args = if args.len() > 6 { args.len() - 6 } else { 0 };

        let needs_align = (self.stack_depth + num_stack_args) % 2 != 0;
        if needs_align {
            self.emit("  sub $8, %rsp");
            self.stack_depth += 1;
        }

        // Push stack arguments (7th and beyond) in reverse order
        for i in (6..args.len()).rev() {
            self.gen_expr(&args[i]);
            self.push();
        }

        // Evaluate first 6 register arguments
        let reg_count = std::cmp::min(args.len(), 6);
        for i in 0..reg_count {
            self.gen_expr(&args[i]);
            self.push();
        }
        let arg_regs = ["%rdi", "%rsi", "%rdx", "%rcx", "%r8", "%r9"];
        for i in (0..reg_count).rev() {
            self.pop(arg_regs[i]);
        }

        // Set %al to 0 for variadic functions (number of vector registers used)
        self.emit("  mov $0, %al");
        emit_call(self);

        if num_stack_args > 0 {
            self.emit(&format!("  add ${}, %rsp", num_stack_args * 8));
            self.stack_depth -= num_stack_args;
        }

        if needs_align {
            self.emit("  add $8, %rsp");
            self.stack_depth -= 1;
        }
    }

    /// Load a value from the address in %rax, based on the given type.
    fn emit_load_indirect(&mut self, ty: &Type) {
        match ty.kind {
            TypeKind::Bool => self.emit("  movzbl (%rax), %eax"),
            TypeKind::Char if ty.is_unsigned => self.emit("  movzbl (%rax), %eax"),
            TypeKind::Char => self.emit("  movsbq (%rax), %rax"),
            TypeKind::Short if ty.is_unsigned => self.emit("  movzwl (%rax), %eax"),
            TypeKind::Short => self.emit("  movswq (%rax), %rax"),
            TypeKind::Int if ty.is_unsigned => self.emit("  movl (%rax), %eax"),
            TypeKind::Int => self.emit("  movslq (%rax), %rax"),
            TypeKind::Float => {
                self.emit("  movss (%rax), %xmm0");
            }
            TypeKind::Double => {
                self.emit("  movsd (%rax), %xmm0");
            }
            TypeKind::Array(_, _) => {} // array-to-pointer decay: address is the value
            _ => self.emit("  mov (%rax), %rax"), // long, ptr
        }
    }

    /// Store %rax to the address in %rdi, based on the given type.
    /// For float/double, stores from %xmm0.
    fn emit_store_indirect(&mut self, ty: &Type) {
        if let TypeKind::Struct(_) = &ty.kind {
            self.emit("  mov %rax, %rsi"); // src
            let size = ty.size();
            self.emit(&format!("  mov ${}, %rcx", size));
            self.emit("  rep movsb");
            return;
        }
        if matches!(ty.kind, TypeKind::Float) {
            self.emit("  movss %xmm0, (%rdi)");
            return;
        }
        if matches!(ty.kind, TypeKind::Double) {
            self.emit("  movsd %xmm0, (%rdi)");
            return;
        }
        if ty.kind == TypeKind::Bool {
            self.emit("  cmp $0, %rax");
            self.emit("  setne %al");
        }
        match ty.kind {
            TypeKind::Bool | TypeKind::Char => self.emit("  movb %al, (%rdi)"),
            TypeKind::Short => self.emit("  movw %ax, (%rdi)"),
            TypeKind::Int => self.emit("  movl %eax, (%rdi)"),
            _ => self.emit("  mov %rax, (%rdi)"), // long, ptr
        }
    }

    fn get_or_create_goto_label(&mut self, name: &str) -> String {
        if let Some(label) = self.goto_labels.get(name) {
            label.clone()
        } else {
            let label = self.new_label();
            self.goto_labels.insert(name.to_string(), label.clone());
            label
        }
    }

    /// Get the type of a variable (local or global).
    fn get_var_type(&self, name: &str) -> Type {
        if let Some(ty) = self.local_types.get(name) {
            return ty.clone();
        }
        if let Some(ty) = self.global_types.get(name) {
            return ty.clone();
        }
        Type::int_type()
    }

    fn emit_load_var(&mut self, name: &str) {
        // If name is not a declared variable, treat it as a function name (function-to-pointer decay)
        if !self.locals.contains_key(name) && !self.globals.contains(name) {
            self.emit(&format!("  lea {}(%rip), %rax", name));
            return;
        }
        let ty = self.get_var_type(name);
        if self.globals.contains(name) {
            match ty.kind {
                TypeKind::Bool => self.emit(&format!("  movzbl {}(%rip), %eax", name)),
                TypeKind::Char if ty.is_unsigned => self.emit(&format!("  movzbl {}(%rip), %eax", name)),
                TypeKind::Char => self.emit(&format!("  movsbq {}(%rip), %rax", name)),
                TypeKind::Short if ty.is_unsigned => self.emit(&format!("  movzwl {}(%rip), %eax", name)),
                TypeKind::Short => self.emit(&format!("  movswq {}(%rip), %rax", name)),
                TypeKind::Int if ty.is_unsigned => self.emit(&format!("  movl {}(%rip), %eax", name)),
                TypeKind::Int => self.emit(&format!("  movslq {}(%rip), %rax", name)),
                TypeKind::Long | TypeKind::Ptr(_) => self.emit(&format!("  mov {}(%rip), %rax", name)),
                TypeKind::Float => self.emit(&format!("  movss {}(%rip), %xmm0", name)),
                TypeKind::Double => self.emit(&format!("  movsd {}(%rip), %xmm0", name)),
                TypeKind::Array(_, _) | TypeKind::Struct(_) => {
                    self.emit(&format!("  lea {}(%rip), %rax", name));
                }
                TypeKind::Void => {}
            }
        } else {
            let offset = self.locals[name];
            match ty.kind {
                TypeKind::Bool => self.emit(&format!("  movzbl -{}(%rbp), %eax", offset)),
                TypeKind::Char if ty.is_unsigned => self.emit(&format!("  movzbl -{}(%rbp), %eax", offset)),
                TypeKind::Char => self.emit(&format!("  movsbq -{}(%rbp), %rax", offset)),
                TypeKind::Short if ty.is_unsigned => self.emit(&format!("  movzwl -{}(%rbp), %eax", offset)),
                TypeKind::Short => self.emit(&format!("  movswq -{}(%rbp), %rax", offset)),
                TypeKind::Int if ty.is_unsigned => self.emit(&format!("  movl -{}(%rbp), %eax", offset)),
                TypeKind::Int => self.emit(&format!("  movslq -{}(%rbp), %rax", offset)),
                TypeKind::Long | TypeKind::Ptr(_) => self.emit(&format!("  mov -{}(%rbp), %rax", offset)),
                TypeKind::Float => self.emit(&format!("  movss -{}(%rbp), %xmm0", offset)),
                TypeKind::Double => self.emit(&format!("  movsd -{}(%rbp), %xmm0", offset)),
                TypeKind::Array(_, _) | TypeKind::Struct(_) => {
                    self.emit(&format!("  lea -{}(%rbp), %rax", offset));
                }
                TypeKind::Void => {}
            }
        }
    }

    fn emit_store_var(&mut self, name: &str) {
        let ty = self.get_var_type(name);
        // Struct copy: %rax = source address, copy to variable location
        if let TypeKind::Struct(_) = &ty.kind {
            let size = ty.size();
            self.emit("  mov %rax, %rsi"); // src address
            if self.globals.contains(name) {
                self.emit(&format!("  lea {}(%rip), %rdi", name));
            } else {
                let offset = self.locals[name];
                self.emit(&format!("  lea -{}(%rbp), %rdi", offset));
            }
            self.emit(&format!("  mov ${}, %rcx", size));
            self.emit("  rep movsb");
            return;
        }
        // Float/double: store from %xmm0
        if matches!(ty.kind, TypeKind::Float) {
            if self.globals.contains(name) {
                self.emit(&format!("  movss %xmm0, {}(%rip)", name));
            } else {
                let offset = self.locals[name];
                self.emit(&format!("  movss %xmm0, -{}(%rbp)", offset));
            }
            return;
        }
        if matches!(ty.kind, TypeKind::Double) {
            if self.globals.contains(name) {
                self.emit(&format!("  movsd %xmm0, {}(%rip)", name));
            } else {
                let offset = self.locals[name];
                self.emit(&format!("  movsd %xmm0, -{}(%rbp)", offset));
            }
            return;
        }
        if ty.kind == TypeKind::Bool {
            self.emit("  cmp $0, %rax");
            self.emit("  setne %al");
        }
        if self.globals.contains(name) {
            match ty.kind {
                TypeKind::Bool | TypeKind::Char => self.emit(&format!("  movb %al, {}(%rip)", name)),
                TypeKind::Short => self.emit(&format!("  movw %ax, {}(%rip)", name)),
                TypeKind::Int => self.emit(&format!("  movl %eax, {}(%rip)", name)),
                TypeKind::Long | TypeKind::Ptr(_) => self.emit(&format!("  mov %rax, {}(%rip)", name)),
                TypeKind::Array(_, _) | TypeKind::Struct(_) | TypeKind::Void | TypeKind::Float | TypeKind::Double => {}
            }
        } else {
            let offset = self.locals[name];
            match ty.kind {
                TypeKind::Bool | TypeKind::Char => self.emit(&format!("  movb %al, -{}(%rbp)", offset)),
                TypeKind::Short => self.emit(&format!("  movw %ax, -{}(%rbp)", offset)),
                TypeKind::Int => self.emit(&format!("  movl %eax, -{}(%rbp)", offset)),
                TypeKind::Long | TypeKind::Ptr(_) => self.emit(&format!("  mov %rax, -{}(%rbp)", offset)),
                TypeKind::Array(_, _) | TypeKind::Struct(_) | TypeKind::Void | TypeKind::Float | TypeKind::Double => {}
            }
        }
    }

    fn emit_store_var_from_rdi(&mut self, name: &str) {
        let ty = self.get_var_type(name);
        if ty.kind == TypeKind::Bool {
            self.emit("  cmp $0, %rdi");
            self.emit("  setne %dil");
        }
        if self.globals.contains(name) {
            match ty.kind {
                TypeKind::Bool | TypeKind::Char => self.emit(&format!("  movb %dil, {}(%rip)", name)),
                TypeKind::Short => self.emit(&format!("  movw %di, {}(%rip)", name)),
                TypeKind::Int => self.emit(&format!("  movl %edi, {}(%rip)", name)),
                TypeKind::Long | TypeKind::Ptr(_) => self.emit(&format!("  mov %rdi, {}(%rip)", name)),
                TypeKind::Array(_, _) | TypeKind::Struct(_) | TypeKind::Void | TypeKind::Float | TypeKind::Double => {}
            }
        } else {
            let offset = self.locals[name];
            match ty.kind {
                TypeKind::Bool | TypeKind::Char => self.emit(&format!("  movb %dil, -{}(%rbp)", offset)),
                TypeKind::Short => self.emit(&format!("  movw %di, -{}(%rbp)", offset)),
                TypeKind::Int => self.emit(&format!("  movl %edi, -{}(%rbp)", offset)),
                TypeKind::Long | TypeKind::Ptr(_) => self.emit(&format!("  mov %rdi, -{}(%rbp)", offset)),
                TypeKind::Array(_, _) | TypeKind::Struct(_) | TypeKind::Void | TypeKind::Float | TypeKind::Double => {}
            }
        }
    }

    fn push(&mut self) {
        self.emit("  push %rax");
        self.stack_depth += 1;
    }

    fn pop(&mut self, reg: &str) {
        self.emit(&format!("  pop {}", reg));
        self.stack_depth -= 1;
    }

    fn push_float(&mut self) {
        self.emit("  sub $8, %rsp");
        self.emit("  movsd %xmm0, (%rsp)");
        self.stack_depth += 1;
    }

    fn pop_float(&mut self, reg: &str) {
        self.emit(&format!("  movsd (%rsp), {}", reg));
        self.emit("  add $8, %rsp");
        self.stack_depth -= 1;
    }

    fn emit(&mut self, s: &str) {
        self.output.push_str(s);
        self.output.push('\n');
    }

    /// Check if a type is floating-point.
    fn is_float_type(ty: &Type) -> bool {
        matches!(ty.kind, TypeKind::Float | TypeKind::Double)
    }

    /// Emit conversion instructions from src type to dst type.
    /// Assumes src result is in the appropriate register (%rax for int, %xmm0 for float).
    fn emit_type_convert(&mut self, src: &Type, dst: &Type) {
        let src_float = Self::is_float_type(src);
        let dst_float = Self::is_float_type(dst);

        if src_float && dst_float {
            // float <-> double
            if matches!(src.kind, TypeKind::Float) && matches!(dst.kind, TypeKind::Double) {
                self.emit("  cvtss2sd %xmm0, %xmm0");
            } else if matches!(src.kind, TypeKind::Double) && matches!(dst.kind, TypeKind::Float) {
                self.emit("  cvtsd2ss %xmm0, %xmm0");
            }
        } else if !src_float && dst_float {
            // int -> float/double
            match dst.kind {
                TypeKind::Float => self.emit("  cvtsi2ss %rax, %xmm0"),
                TypeKind::Double => self.emit("  cvtsi2sd %rax, %xmm0"),
                _ => {}
            }
        } else if src_float && !dst_float {
            // float/double -> int (truncate toward zero, as C requires)
            match src.kind {
                TypeKind::Float => self.emit("  cvttss2si %xmm0, %rax"),
                TypeKind::Double => self.emit("  cvttsd2si %xmm0, %rax"),
                _ => {}
            }
        }
    }

    /// Generate code for a float/double binary operation.
    fn gen_float_binop(&mut self, op: &BinOp, lhs: &Expr, rhs: &Expr) {
        let lhs_ty = self.expr_type(lhs);
        let rhs_ty = self.expr_type(rhs);
        // Use double precision if either side is double
        let is_double = matches!(lhs_ty.kind, TypeKind::Double) || matches!(rhs_ty.kind, TypeKind::Double);

        // Evaluate rhs, convert to float if needed
        self.gen_expr(rhs);
        if !Self::is_float_type(&rhs_ty) {
            if is_double {
                self.emit("  cvtsi2sd %rax, %xmm0");
            } else {
                self.emit("  cvtsi2ss %rax, %xmm0");
            }
        } else if is_double && matches!(rhs_ty.kind, TypeKind::Float) {
            self.emit("  cvtss2sd %xmm0, %xmm0");
        }
        self.push_float(); // save rhs on stack

        // Evaluate lhs, convert to float if needed
        self.gen_expr(lhs);
        if !Self::is_float_type(&lhs_ty) {
            if is_double {
                self.emit("  cvtsi2sd %rax, %xmm0");
            } else {
                self.emit("  cvtsi2ss %rax, %xmm0");
            }
        } else if is_double && matches!(lhs_ty.kind, TypeKind::Float) {
            self.emit("  cvtss2sd %xmm0, %xmm0");
        }

        self.pop_float("%xmm1"); // rhs in %xmm1, lhs in %xmm0

        let suffix = if is_double { "sd" } else { "ss" };

        match op {
            BinOp::Add => self.emit(&format!("  add{} %xmm1, %xmm0", suffix)),
            BinOp::Sub => self.emit(&format!("  sub{} %xmm1, %xmm0", suffix)),
            BinOp::Mul => self.emit(&format!("  mul{} %xmm1, %xmm0", suffix)),
            BinOp::Div => self.emit(&format!("  div{} %xmm1, %xmm0", suffix)),
            BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                self.emit(&format!("  ucomi{} %xmm1, %xmm0", suffix));
                match op {
                    BinOp::Eq => {
                        self.emit("  sete %al");
                        self.emit("  setnp %cl");
                        self.emit("  and %cl, %al");
                    }
                    BinOp::Ne => {
                        self.emit("  setne %al");
                        self.emit("  setp %cl");
                        self.emit("  or %cl, %al");
                    }
                    BinOp::Lt => self.emit("  setb %al"),
                    BinOp::Le => self.emit("  setbe %al"),
                    BinOp::Gt => self.emit("  seta %al"),
                    BinOp::Ge => self.emit("  setae %al"),
                    _ => {}
                }
                self.emit("  movzb %al, %rax");
                // Result is in %rax (integer), not %xmm0
                return;
            }
            _ => {} // bitwise ops not applicable to float
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_return_number() {
        let mut codegen = Codegen::new("test.c");
        let program = Program {
            globals: vec![],
            extern_names: std::collections::HashSet::new(),
            functions: vec![Function {
                name: "main".to_string(),
                return_ty: Type::int_type(),
                params: vec![],
                is_variadic: false,
                body: vec![Stmt::Return(Some(Expr::Num(42)))],
                locals: vec![],
            }],
        };
        let output = codegen.generate(&program);
        assert!(output.contains("mov $42, %rax"));
        assert!(output.contains("jmp .Lreturn.main"));
    }

    #[test]
    fn test_var_decl_and_return() {
        let mut codegen = Codegen::new("test.c");
        let program = Program {
            globals: vec![],
            extern_names: std::collections::HashSet::new(),
            functions: vec![Function {
                name: "main".to_string(),
                return_ty: Type::int_type(),
                params: vec![],
                is_variadic: false,
                body: vec![
                    Stmt::VarDecl {
                        name: "a".to_string(),
                        ty: Type::int_type(),
                        init: Some(Expr::Num(5)),
                    },
                    Stmt::Return(Some(Expr::Var("a".to_string()))),
                ],
                locals: vec![(Type::int_type(), "a".to_string())],
            }],
        };
        let output = codegen.generate(&program);
        assert!(output.contains("sub $16, %rsp"));
        assert!(output.contains("movl %eax, -4(%rbp)"));
        assert!(output.contains("movslq -4(%rbp), %rax"));
    }

    #[test]
    fn test_char_var() {
        let mut codegen = Codegen::new("test.c");
        let program = Program {
            globals: vec![],
            extern_names: std::collections::HashSet::new(),
            functions: vec![Function {
                name: "main".to_string(),
                return_ty: Type::int_type(),
                params: vec![],
                is_variadic: false,
                body: vec![
                    Stmt::VarDecl {
                        name: "a".to_string(),
                        ty: Type::char_type(),
                        init: Some(Expr::Num(65)),
                    },
                    Stmt::Return(Some(Expr::Var("a".to_string()))),
                ],
                locals: vec![(Type::char_type(), "a".to_string())],
            }],
        };
        let output = codegen.generate(&program);
        // char uses movb for store and movsbq for load
        assert!(output.contains("movb %al, -1(%rbp)"));
        assert!(output.contains("movsbq -1(%rbp), %rax"));
    }

    #[test]
    fn test_unsigned_char_var() {
        let mut codegen = Codegen::new("test.c");
        let program = Program {
            globals: vec![],
            extern_names: std::collections::HashSet::new(),
            functions: vec![Function {
                name: "main".to_string(),
                return_ty: Type::int_type(),
                params: vec![],
                is_variadic: false,
                body: vec![
                    Stmt::VarDecl {
                        name: "a".to_string(),
                        ty: Type::uchar(),
                        init: Some(Expr::Num(200)),
                    },
                    Stmt::Return(Some(Expr::Var("a".to_string()))),
                ],
                locals: vec![(Type::uchar(), "a".to_string())],
            }],
        };
        let output = codegen.generate(&program);
        // unsigned char uses movb for store and movzbl for load
        assert!(output.contains("movb %al, -1(%rbp)"));
        assert!(output.contains("movzbl -1(%rbp), %eax"));
    }
}
