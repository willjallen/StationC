use super::tokens::TokenKind;

#[must_use]
pub(super) fn lex_string(input: String) -> Vec<TokenKind> {
    let mut tokens: Vec<TokenKind> = Vec::new();

    let mut curr_lexeme: String = String::new();

    for ch in input.chars() {
        // = and == are ambiguous
        // whitespace separation identifies tokens
        // and newlines
        if !(ch == ' ' || ch == '\n' || ch == '\t') {
            curr_lexeme.push(ch);
            continue;
        }

        if !curr_lexeme.is_empty() {
            tokens.push(match_lexeme(&curr_lexeme));
            curr_lexeme.clear();
        }
    }

    if !curr_lexeme.is_empty() {
        tokens.push(match_lexeme(&curr_lexeme));
    }

    tokens
}

fn match_lexeme(lexeme: &str) -> TokenKind {
    match lexeme {
        "if" => TokenKind::If,
        "while" => TokenKind::While,
        ";" => TokenKind::Semicolon,
        "{" => TokenKind::OpenCurlyBracket,
        "}" => TokenKind::CloseCurlyBracket,
        "int" => TokenKind::Int,
        "float" => TokenKind::Float,
        "=" => TokenKind::Assign,
        "<" => TokenKind::LessThan,
        ">" => TokenKind::GreaterThan,
        "<=" => TokenKind::LessThanOrEqual,
        ">=" => TokenKind::GreaterThanOrEqual,
        "==" => TokenKind::Equals,
        _ => TokenKind::Identifier,
    }
}
