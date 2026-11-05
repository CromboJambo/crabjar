//! C++ tokenizer: lexes source into simplified tokens using regex patterns.

use super::CppTok;

/// Tokenize C++ source into simplified tokens using regex patterns.
pub fn tokenize(source: &str) -> Vec<CppTok> {
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
            tokens.push(CppTok::Preprocessor(trimmed.to_string()));
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
                        if let Some(esc) = chars.next() {
                            content.push(esc);
                        }
                    } else if c == '"' {
                        break;
                    } else {
                        content.push(c);
                    }
                }
                tokens.push(CppTok::StringLiteral(content));
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
                    tokens.push(CppTok::CharLiteral(content.chars().next().unwrap()));
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
                tokens.push(CppTok::NumberLiteral(num));
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

                // Check if it's a keyword (C++ keywords include C keywords plus more)
                let keywords = [
                    "alignas",
                    "alignof",
                    "and",
                    "and_eq",
                    "asm",
                    "auto",
                    "bitand",
                    "bitor",
                    "bool",
                    "break",
                    "case",
                    "catch",
                    "char",
                    "char8_t",
                    "char16_t",
                    "char32_t",
                    "class",
                    "compl",
                    "concept",
                    "const",
                    "consteval",
                    "constexpr",
                    "constinit",
                    "const_cast",
                    "continue",
                    "co_await",
                    "co_return",
                    "co_yield",
                    "decltype",
                    "default",
                    "delete",
                    "do",
                    "double",
                    "dynamic_cast",
                    "else",
                    "enum",
                    "explicit",
                    "export",
                    "extern",
                    "false",
                    "float",
                    "for",
                    "friend",
                    "goto",
                    "if",
                    "import",
                    "inline",
                    "int",
                    "long",
                    "module",
                    "mutable",
                    "namespace",
                    "new",
                    "noexcept",
                    "not",
                    "not_eq",
                    "nullptr",
                    "operator",
                    "or",
                    "or_eq",
                    "private",
                    "protected",
                    "public",
                    "register",
                    "reinterpret_cast",
                    "requires",
                    "return",
                    "short",
                    "signed",
                    "sizeof",
                    "static",
                    "static_assert",
                    "static_cast",
                    "struct",
                    "switch",
                    "template",
                    "this",
                    "thread_local",
                    "throw",
                    "true",
                    "try",
                    "typedef",
                    "typeid",
                    "typename",
                    "union",
                    "unsigned",
                    "using",
                    "virtual",
                    "void",
                    "volatile",
                    "while",
                    "xor",
                    "xor_eq",
                ];

                if keywords.contains(&word.as_str()) {
                    tokens.push(CppTok::Keyword(word));
                } else {
                    tokens.push(CppTok::Identifier(word));
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
            } else if ch == '*' && chars.peek() == Some(&'=') {
                punct.push(chars.next().unwrap());
            } else if ch == '/' && chars.peek() == Some(&'=') {
                punct.push(chars.next().unwrap());
            }
            tokens.push(CppTok::Punctuation(punct));
        }
    }

    tokens
}
