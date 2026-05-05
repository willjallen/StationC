use super::tokens::Token;

#[must_use]
pub(super) fn lex_string(input: String) -> Vec<Token>{
    let mut tokens: Vec<Token> = Vec::new();

    let mut curr_lexeme: String = String::new();

    for ch in input.chars() {

        // = and == are ambiguous
        // whitespace separation identifies tokens
        // and newlines
        if !(ch == ' ' || ch == '\n' || ch == '\t') {
            curr_lexeme.push(ch);
            continue;
        }

        let start = curr_lexeme.len() - curr_lexeme.trim_start().len();
        curr_lexeme.replace_range(..start, "");

        let mut lexeme: Option<Token>;

        let lexeme = match curr_lexeme.as_str() {
            "if" => Some(Token::If),
            "while" => Some(Token::While),
            ";" => Some(Token::While),
            "{" => Some(Token::OpenCurlyBracket),
            "}" => Some(Token::CloseCurlyBracket),
            "int" => Some(Token::Int),
            "float" => Some(Token::Float),
            "=" => Some(Token::Equals),
            "<" => Some(Token::LessThan),
            ">" => Some(Token::GreaterThan),
            "<=" => Some(Token::LessThanOrEqual),
            ">=" => Some(Token::GreaterThanOrEqual),
            "==" => Some(Token::Equals),
            _ => None
        };

    }

    tokens
}

fn match_lexeme(lexeme: &str) {

}