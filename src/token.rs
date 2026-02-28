#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Num(i64),
    Ident(String),
    // Keywords
    Return,
    Int,
    If,
    Else,
    While,
    For,
    // Operators
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Eq,
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
