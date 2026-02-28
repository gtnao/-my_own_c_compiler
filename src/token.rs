#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Num(i64),
    Ident(String),
    // Keywords
    Return,
    Int,
    // Operators
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    EqEq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    LParen,
    RParen,
    LBrace,
    RBrace,
    Semicolon,
    Eof,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub pos: usize,
}
