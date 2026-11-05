//! Go tokenizer: lexes source into simplified tokens using regex patterns.

use super::GoTok;

/// Tokenize Go source into simplified tokens using regex patterns.
pub fn tokenize(source: &str) -> Vec<GoTok> {
    let mut tokens = Vec::new();
    let lines: Vec<&str> = source.lines().collect();

    for line in lines {
        let trimmed = line.trim();

        // Skip empty lines
        if trimmed.is_empty() {
            continue;
        }

        // Detect comments
        if trimmed.starts_with("//") {
            tokens.push(GoTok::Comment(trimmed.to_string()));
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

            // String literals (double quotes)
            if ch == '"' {
                let mut content = String::new();
                while let Some(c) = chars.next() {
                    if c == '\\' {
                        if let Some(esc) = chars.next() {
                            content.push(esc);
                        }
                    } else if c == '"' {
                        break;
                    } else {
                        content.push(c);
                    }
                }
                tokens.push(GoTok::StringLiteral(content));
                continue;
            }

            // Character literals (single quotes)
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
                    tokens.push(GoTok::CharLiteral(content.chars().next().unwrap()));
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
                tokens.push(GoTok::NumberLiteral(num));
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

                // Go keywords
                let keywords = [
                    "break",
                    "case",
                    "chan",
                    "const",
                    "continue",
                    "default",
                    "defer",
                    "else",
                    "fallthrough",
                    "for",
                    "func",
                    "go",
                    "goto",
                    "if",
                    "import",
                    "interface",
                    "map",
                    "package",
                    "range",
                    "return",
                    "select",
                    "struct",
                    "switch",
                    "type",
                    "var",
                ];

                if keywords.contains(&word.as_str()) {
                    tokens.push(GoTok::Keyword(word));
                } else {
                    tokens.push(GoTok::Identifier(word));
                }
                continue;
            }

            // Punctuation and operators
            let mut punct = String::new();
            punct.push(ch);
            // Handle multi-char operators
            if ch == '=' && chars.peek() == Some(&'=') {
                punct.push(chars.next().unwrap());
            } else if ch == '!' && chars.peek() == Some(&'=') {
                punct.push(chars.next().unwrap());
            } else if ch == '<' && (chars.peek() == Some(&'=') || chars.peek() == Some(&'<')) {
                punct.push(chars.next().unwrap());
            } else if ch == '>' && (chars.peek() == Some(&'=') || chars.peek() == Some(&'>')) {
                punct.push(chars.next().unwrap());
            } else if ch == '&' && chars.peek() == Some(&'&') {
                punct.push(chars.next().unwrap());
            } else if ch == '|' && chars.peek() == Some(&'|') {
                punct.push(chars.next().unwrap());
            } else if ch == '+' && chars.peek() == Some(&'+') {
                punct.push(chars.next().unwrap());
            } else if ch == '-' && chars.peek() == Some(&'-') {
                punct.push(chars.next().unwrap());
            }
            tokens.push(GoTok::Punctuation(punct));
        }
    }

    tokens
}
