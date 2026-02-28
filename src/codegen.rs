use crate::ast::{BinOp, Expr, Function, Program, Stmt, UnaryOp};
use crate::types::Type;
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
}

impl Codegen {
    pub fn new() -> Self {
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
        }
    }

    fn new_label(&mut self) -> String {
        let label = format!(".L{}", self.label_count);
        self.label_count += 1;
        label
    }

    pub fn generate(&mut self, program: &Program) -> String {
        // Register global variable names and types
        for (ty, name) in &program.globals {
            self.globals.insert(name.clone());
            self.global_types.insert(name.clone(), ty.clone());
        }

        // Emit global variable declarations in .bss section
        if !program.globals.is_empty() {
            for (ty, name) in &program.globals {
                let size = ty.size();
                let align = ty.align();
                self.emit(&format!("  .comm {}, {}, {}", name, size, align));
            }
        }

        // Generate code for functions
        for func in &program.functions {
            self.gen_function(func);
        }
        self.output.clone()
    }

    fn gen_function(&mut self, func: &Function) {
        self.current_func_name = func.name.clone();
        self.stack_depth = 0;

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
            match ty {
                Type::Char => {
                    self.emit(&format!("  movb {}, -{}(%rbp)", arg_regs_8[i], offset));
                }
                Type::Short => {
                    self.emit(&format!("  movw {}, -{}(%rbp)", arg_regs_16[i], offset));
                }
                Type::Int => {
                    self.emit(&format!("  movl {}, -{}(%rbp)", arg_regs_32[i], offset));
                }
                Type::Long => {
                    self.emit(&format!("  mov {}, -{}(%rbp)", arg_regs_64[i], offset));
                }
                Type::Void => {}
            }
        }
        // Copy stack parameters to local slots (7th and beyond)
        for (i, (_ty, param)) in func.params.iter().enumerate().skip(6) {
            let src_offset = 16 + (i - 6) * 8;
            self.emit(&format!("  mov {}(%rbp), %rax", src_offset));
            self.emit_store_var(param);
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
                    self.gen_expr(expr);
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
            Expr::Var(name) => {
                self.emit_load_var(name);
            }
            Expr::Assign { lhs, rhs } => {
                self.gen_expr(rhs);
                match lhs.as_ref() {
                    Expr::Var(name) => {
                        self.emit_store_var(name);
                    }
                    _ => {}
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
                if let Expr::Var(name) = operand.as_ref() {
                    self.emit_load_var(name);
                    self.emit("  add $1, %rax");
                    self.emit_store_var(name);
                }
            }
            Expr::PreDec(operand) => {
                if let Expr::Var(name) = operand.as_ref() {
                    self.emit_load_var(name);
                    self.emit("  sub $1, %rax");
                    self.emit_store_var(name);
                }
            }
            Expr::PostInc(operand) => {
                if let Expr::Var(name) = operand.as_ref() {
                    self.emit_load_var(name);
                    self.emit("  mov %rax, %rdi");
                    self.emit("  add $1, %rdi");
                    self.emit_store_var_from_rdi(name);
                }
            }
            Expr::PostDec(operand) => {
                if let Expr::Var(name) = operand.as_ref() {
                    self.emit_load_var(name);
                    self.emit("  mov %rax, %rdi");
                    self.emit("  sub $1, %rdi");
                    self.emit_store_var_from_rdi(name);
                }
            }
            Expr::FuncCall { name, args } => {
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

                self.emit(&format!("  call {}", name));

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
            Expr::Cast { ty, expr } => {
                self.gen_expr(expr);
                // Truncate and re-extend to target type
                match ty {
                    Type::Char => self.emit("  movsbq %al, %rax"),
                    Type::Short => self.emit("  movswq %ax, %rax"),
                    Type::Int => self.emit("  movslq %eax, %rax"),
                    Type::Long | Type::Void => {} // no-op
                }
            }
            Expr::BinOp { op, lhs, rhs } => {
                self.gen_expr(rhs);
                self.push();
                self.gen_expr(lhs);
                self.pop("%rdi");

                match op {
                    BinOp::Add => {
                        self.emit("  add %rdi, %rax");
                    }
                    BinOp::Sub => {
                        self.emit("  sub %rdi, %rax");
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
    fn get_var_type(&self, name: &str) -> &Type {
        if let Some(ty) = self.local_types.get(name) {
            return ty;
        }
        if let Some(ty) = self.global_types.get(name) {
            return ty;
        }
        &Type::Int
    }

    fn emit_load_var(&mut self, name: &str) {
        let ty = self.get_var_type(name).clone();
        if self.globals.contains(name) {
            match ty {
                Type::Char => self.emit(&format!("  movsbq {}(%rip), %rax", name)),
                Type::Short => self.emit(&format!("  movswq {}(%rip), %rax", name)),
                Type::Int => self.emit(&format!("  movslq {}(%rip), %rax", name)),
                Type::Long => self.emit(&format!("  mov {}(%rip), %rax", name)),
                Type::Void => {}
            }
        } else {
            let offset = self.locals[name];
            match ty {
                Type::Char => self.emit(&format!("  movsbq -{}(%rbp), %rax", offset)),
                Type::Short => self.emit(&format!("  movswq -{}(%rbp), %rax", offset)),
                Type::Int => self.emit(&format!("  movslq -{}(%rbp), %rax", offset)),
                Type::Long => self.emit(&format!("  mov -{}(%rbp), %rax", offset)),
                Type::Void => {}
            }
        }
    }

    fn emit_store_var(&mut self, name: &str) {
        let ty = self.get_var_type(name).clone();
        if self.globals.contains(name) {
            match ty {
                Type::Char => self.emit(&format!("  movb %al, {}(%rip)", name)),
                Type::Short => self.emit(&format!("  movw %ax, {}(%rip)", name)),
                Type::Int => self.emit(&format!("  movl %eax, {}(%rip)", name)),
                Type::Long => self.emit(&format!("  mov %rax, {}(%rip)", name)),
                Type::Void => {}
            }
        } else {
            let offset = self.locals[name];
            match ty {
                Type::Char => self.emit(&format!("  movb %al, -{}(%rbp)", offset)),
                Type::Short => self.emit(&format!("  movw %ax, -{}(%rbp)", offset)),
                Type::Int => self.emit(&format!("  movl %eax, -{}(%rbp)", offset)),
                Type::Long => self.emit(&format!("  mov %rax, -{}(%rbp)", offset)),
                Type::Void => {}
            }
        }
    }

    fn emit_store_var_from_rdi(&mut self, name: &str) {
        let ty = self.get_var_type(name).clone();
        if self.globals.contains(name) {
            match ty {
                Type::Char => self.emit(&format!("  movb %dil, {}(%rip)", name)),
                Type::Short => self.emit(&format!("  movw %di, {}(%rip)", name)),
                Type::Int => self.emit(&format!("  movl %edi, {}(%rip)", name)),
                Type::Long => self.emit(&format!("  mov %rdi, {}(%rip)", name)),
                Type::Void => {}
            }
        } else {
            let offset = self.locals[name];
            match ty {
                Type::Char => self.emit(&format!("  movb %dil, -{}(%rbp)", offset)),
                Type::Short => self.emit(&format!("  movw %di, -{}(%rbp)", offset)),
                Type::Int => self.emit(&format!("  movl %edi, -{}(%rbp)", offset)),
                Type::Long => self.emit(&format!("  mov %rdi, -{}(%rbp)", offset)),
                Type::Void => {}
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

    fn emit(&mut self, s: &str) {
        self.output.push_str(s);
        self.output.push('\n');
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_return_number() {
        let mut codegen = Codegen::new();
        let program = Program {
            globals: vec![],
            functions: vec![Function {
                name: "main".to_string(),
                return_ty: Type::Int,
                params: vec![],
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
        let mut codegen = Codegen::new();
        let program = Program {
            globals: vec![],
            functions: vec![Function {
                name: "main".to_string(),
                return_ty: Type::Int,
                params: vec![],
                body: vec![
                    Stmt::VarDecl {
                        name: "a".to_string(),
                        ty: Type::Int,
                        init: Some(Expr::Num(5)),
                    },
                    Stmt::Return(Some(Expr::Var("a".to_string()))),
                ],
                locals: vec![(Type::Int, "a".to_string())],
            }],
        };
        let output = codegen.generate(&program);
        assert!(output.contains("sub $16, %rsp"));
        assert!(output.contains("movl %eax, -4(%rbp)"));
        assert!(output.contains("movslq -4(%rbp), %rax"));
    }

    #[test]
    fn test_char_var() {
        let mut codegen = Codegen::new();
        let program = Program {
            globals: vec![],
            functions: vec![Function {
                name: "main".to_string(),
                return_ty: Type::Int,
                params: vec![],
                body: vec![
                    Stmt::VarDecl {
                        name: "a".to_string(),
                        ty: Type::Char,
                        init: Some(Expr::Num(65)),
                    },
                    Stmt::Return(Some(Expr::Var("a".to_string()))),
                ],
                locals: vec![(Type::Char, "a".to_string())],
            }],
        };
        let output = codegen.generate(&program);
        // char uses movb for store and movsbq for load
        assert!(output.contains("movb %al, -1(%rbp)"));
        assert!(output.contains("movsbq -1(%rbp), %rax"));
    }
}
