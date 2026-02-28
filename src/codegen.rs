use crate::ast::{BinOp, Expr, UnaryOp};

pub struct Codegen {
    output: String,
}

impl Codegen {
    pub fn new() -> Self {
        Self {
            output: String::new(),
        }
    }

    pub fn generate(&mut self, expr: &Expr) -> String {
        self.emit("  .globl main");
        self.emit("main:");
        self.emit("  push %rbp");
        self.emit("  mov %rsp, %rbp");

        self.gen_expr(expr);

        self.emit("  mov %rbp, %rsp");
        self.emit("  pop %rbp");
        self.emit("  ret");

        self.output.clone()
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
                        // idiv: quotient -> %rax, remainder -> %rdx
                        self.emit("  cqto");
                        self.emit("  idiv %rdi");
                        self.emit("  mov %rdx, %rax");
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
    fn test_single_number() {
        let mut codegen = Codegen::new();
        let output = codegen.generate(&Expr::Num(42));
        assert!(output.contains("mov $42, %rax"));
    }

    #[test]
    fn test_addition() {
        let mut codegen = Codegen::new();
        let expr = Expr::BinOp {
            op: BinOp::Add,
            lhs: Box::new(Expr::Num(1)),
            rhs: Box::new(Expr::Num(2)),
        };
        let output = codegen.generate(&expr);
        assert!(output.contains("add %rdi, %rax"));
    }

    #[test]
    fn test_negation() {
        let mut codegen = Codegen::new();
        let expr = Expr::UnaryOp {
            op: UnaryOp::Neg,
            operand: Box::new(Expr::Num(10)),
        };
        let output = codegen.generate(&expr);
        assert!(output.contains("neg %rax"));
    }
}
