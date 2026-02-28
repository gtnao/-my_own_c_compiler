use crate::ast::{BinOp, Expr, Function, Stmt, UnaryOp};
use std::collections::HashMap;

pub struct Codegen {
    output: String,
    locals: HashMap<String, usize>,
    stack_size: usize,
    label_count: usize,
}

impl Codegen {
    pub fn new() -> Self {
        Self {
            output: String::new(),
            locals: HashMap::new(),
            stack_size: 0,
            label_count: 0,
        }
    }

    fn new_label(&mut self) -> String {
        let label = format!(".L{}", self.label_count);
        self.label_count += 1;
        label
    }

    pub fn generate(&mut self, func: &Function, local_vars: &[String]) -> String {
        // Set up local variable offsets on stack
        self.locals.clear();
        for (i, name) in local_vars.iter().enumerate() {
            self.locals.insert(name.clone(), (i + 1) * 8);
        }
        self.stack_size = local_vars.len() * 8;
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

        for stmt in &func.body {
            self.gen_stmt(stmt);
        }

        // Default return 0 if no return statement reached
        self.emit("  mov $0, %rax");
        self.emit(&format!(".Lreturn.{}:", func.name));
        self.emit("  mov %rbp, %rsp");
        self.emit("  pop %rbp");
        self.emit("  ret");

        self.output.clone()
    }

    fn gen_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Return(expr) => {
                self.gen_expr(expr);
                self.emit("  jmp .Lreturn.main");
            }
            Stmt::ExprStmt(expr) => {
                self.gen_expr(expr);
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

                self.emit(&format!("{}:", begin_label));
                self.gen_expr(cond);
                self.emit("  cmp $0, %rax");
                self.emit(&format!("  je {}", end_label));
                self.gen_stmt(body);
                self.emit(&format!("  jmp {}", begin_label));
                self.emit(&format!("{}:", end_label));
            }
            Stmt::VarDecl { name, init } => {
                if let Some(expr) = init {
                    self.gen_expr(expr);
                    let offset = self.locals[name];
                    self.emit(&format!("  mov %rax, -{}(%rbp)", offset));
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
                let offset = self.locals[name];
                self.emit(&format!("  mov -{}(%rbp), %rax", offset));
            }
            Expr::Assign { lhs, rhs } => {
                self.gen_expr(rhs);
                match lhs.as_ref() {
                    Expr::Var(name) => {
                        let offset = self.locals[name];
                        self.emit(&format!("  mov %rax, -{}(%rbp)", offset));
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
                }
            }
            Expr::BinOp { op, lhs, rhs } => {
                // Evaluate rhs first, push it, then evaluate lhs
                self.gen_expr(rhs);
                self.emit("  push %rax");
                self.gen_expr(lhs);
                self.emit("  pop %rdi");

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
                }
            }
        }
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
        let func = Function {
            name: "main".to_string(),
            body: vec![Stmt::Return(Expr::Num(42))],
        };
        let output = codegen.generate(&func, &[]);
        assert!(output.contains("mov $42, %rax"));
        assert!(output.contains("jmp .Lreturn.main"));
    }

    #[test]
    fn test_var_decl_and_return() {
        let mut codegen = Codegen::new();
        let func = Function {
            name: "main".to_string(),
            body: vec![
                Stmt::VarDecl {
                    name: "a".to_string(),
                    init: Some(Expr::Num(5)),
                },
                Stmt::Return(Expr::Var("a".to_string())),
            ],
        };
        let output = codegen.generate(&func, &["a".to_string()]);
        assert!(output.contains("sub $16, %rsp"));
        assert!(output.contains("mov %rax, -8(%rbp)"));
        assert!(output.contains("mov -8(%rbp), %rax"));
    }
}
