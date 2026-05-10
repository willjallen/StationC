/**

- if, while
- identifier
- ;
- {, }
- int, float
- =, <, >, <=, >=, ==


*/

#[derive(Debug, Clone)]
pub(super) struct Token {
    token_kind: TokenKind,
    token_content: String,
}

#[derive(Debug, Copy, Clone)]
pub(super) enum TokenKind {
    If,
    While,
    Identifier,
    Semicolon,
    OpenCurlyBracket,
    CloseCurlyBracket,
    Int,
    Float,
    Assign,
    LessThan,
    GreaterThan,
    LessThanOrEqual,
    GreaterThanOrEqual,
    Equals,

    Invalid,
}
