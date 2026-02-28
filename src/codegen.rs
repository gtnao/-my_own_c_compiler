use crate::ast::{BinOp, Expr, Function, Stmt, UnaryOp};

pub struct Codegen {
    output: String,
}

impl Codegen {
    pub fn new() -> Self {
        Self {
            output: String::new(),
        }
    }

    pub fn generate(&mut self, func: &Function) -> String {
        self.emit(&format!("  .globl {}", func.name));
        self.emit(&format!("{}:", func.name));
        self.emit("  push %rbp");
        self.emit("  mov %rsp, %rbp");

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
        }
    }

    fn gen_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Num(val) => {
                self.emit(&format!("  mov ${}, %rax", val));
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
        let output = codegen.generate(&func);
        assert!(output.contains("mov $42, %rax"));
        assert!(output.contains("jmp .Lreturn.main"));
    }

    #[test]
    fn test_expr_stmt_and_return() {
        let mut codegen = Codegen::new();
        let func = Function {
            name: "main".to_string(),
            body: vec![
                Stmt::ExprStmt(Expr::Num(1)),
                Stmt::Return(Expr::Num(3)),
            ],
        };
        let output = codegen.generate(&func);
        assert!(output.contains("mov $1, %rax"));
        assert!(output.contains("mov $3, %rax"));
    }
}
