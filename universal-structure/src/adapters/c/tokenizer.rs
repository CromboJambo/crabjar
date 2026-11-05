//! C tokenizer - converts source into simplified token stream.

/// Simplified C token kind for internal use.
#[derive(Debug, Clone, PartialEq)]
pub enum CTok {
    Keyword(String),
    Identifier(String),
    Punctuation(String),
    StringLiteral(String),
    CharLiteral(char),
    NumberLiteral(String),
    Preprocessor(String),
}

/// Tokenize C source into simplified tokens using regex patterns.
#[allow(clippy::too_many_lines)]
pub fn tokenize(source: &str) -> Vec<CTok> {
    let mut tokens = Vec::new();
    let lines: Vec<&str> = source.lines().collect();

    for line in lines {
        let trimmed = line.trim();

        // Skip empty lines and comments
        if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with("/*") {
            continue;
        }

        // Detect preprocessor directives
        if trimmed.starts_with('#') {
            tokens.push(CTok::Preprocessor(trimmed.to_string()));
            continue;
        }

        // Tokenize the line
        let mut chars = trimmed.chars().peekable();
        while chars.peek().is_some() {
            let ch = chars.next().unwrap();

            // Skip whitespace
            if ch.is_whitespace() {
                continue;
            }

            // String literals
            if ch == '"' {
                let mut content = String::new();
                while let Some(c) = chars.next() {
                    if c == '\\' {
                        // Escape sequence
                        if let Some(esc) = chars.next() {
                            content.push(esc);
                        }
                    } else if c == '"' {
                        break;
                    } else {
                        content.push(c);
                    }
                }
                tokens.push(CTok::StringLiteral(content));
                continue;
            }

            // Character literals
            if ch == '\'' {
                let mut content = String::new();
                while let Some(c) = chars.next() {
                    if c == '\\' {
                        if let Some(esc) = chars.next() {
                            content.push(esc);
                        }
                    } else if c == '\'' {
                        break;
                    } else {
                        content.push(c);
                    }
                }
                if !content.is_empty() {
                    tokens.push(CTok::CharLiteral(content.chars().next().unwrap()));
                }
                continue;
            }

            // Number literals
            if ch.is_ascii_digit()
                || (ch == '.' && chars.peek().is_some() && chars.peek().unwrap().is_ascii_digit())
            {
                let mut num = String::new();
                num.push(ch);
                while let Some(&c) = chars.peek() {
                    if c.is_ascii_alphanumeric() || c == '.' || c == 'x' || c == 'X' {
                        num.push(chars.next().unwrap());
                    } else {
                        break;
                    }
                }
                tokens.push(CTok::NumberLiteral(num));
                continue;
            }

            // Identifiers and keywords
            if ch.is_alphabetic() || ch == '_' {
                let mut word = String::new();
                word.push(ch);
                while let Some(&c) = chars.peek() {
                    if c.is_alphanumeric() || c == '_' {
                        word.push(chars.next().unwrap());
                    } else {
                        break;
                    }
                }

                // Check if it's a keyword
                let keywords = [
                    "auto",
                    "break",
                    "case",
                    "char",
                    "const",
                    "continue",
                    "default",
                    "do",
                    "double",
                    "else",
                    "enum",
                    "extern",
                    "float",
                    "for",
                    "goto",
                    "if",
                    "inline",
                    "int",
                    "long",
                    "register",
                    "restrict",
                    "return",
                    "short",
                    "signed",
                    "sizeof",
                    "static",
                    "struct",
                    "switch",
                    "typedef",
                    "union",
                    "unsigned",
                    "void",
                    "volatile",
                    "while",
                    "_Bool",
                    "_Complex",
                    "_Imaginary",
                ];

                if keywords.contains(&word.as_str()) {
                    tokens.push(CTok::Keyword(word));
                } else {
                    tokens.push(CTok::Identifier(word));
                }
                continue;
            }

            // Punctuation and operators
            let mut punct = String::new();
            punct.push(ch);
            // Handle multi-char operators
            if (ch == '=' && chars.peek() == Some(&'='))
                || (ch == '!' && chars.peek() == Some(&'='))
                || (ch == '<' && (chars.peek() == Some(&'=') || chars.peek() == Some(&'<')))
                || (ch == '>' && (chars.peek() == Some(&'=') || chars.peek() == Some(&'>')))
                || (ch == '&' && chars.peek() == Some(&'&'))
                || (ch == '|' && chars.peek() == Some(&'|'))
                || (ch == '+' && chars.peek() == Some(&'+'))
                || (ch == '-' && chars.peek() == Some(&'-'))
                || (ch == '*' && chars.peek() == Some(&'='))
                || (ch == '/' && chars.peek() == Some(&'='))
            {
                punct.push(chars.next().unwrap());
            }
            tokens.push(CTok::Punctuation(punct));
        }
    }

    tokens
}
