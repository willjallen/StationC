
/**

- if, while
- identifier
- ;
- {, }
- int, float
- =, <, >, <=, >=, ==


*/
#[derive(Debug, Copy, Clone)]
pub(super) enum Token {
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

    Invalid
}

pub(super) struct IdentifierName {
    name: str
}