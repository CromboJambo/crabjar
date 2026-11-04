use crate::types::ObfuscationReport;
use regex::Regex;

/// Detect obfuscation patterns in JavaScript source.
pub fn detect_obfuscation(source: &str) -> ObfuscationReport {
    let mut suspicious = Vec::new();

    // eval() usage
    let re_eval = Regex::new(r"\beval\s*\(").unwrap();
    let eval_usage = re_eval.is_match(source);
    if eval_usage {
        suspicious.push("eval() used".to_string());
    }

    // base64 decoding
    let re_b64 = Regex::new(r"atob|btoa|Buffer\.from\([^)]+,'base64'\)").unwrap();
    let base64_decoding = re_b64.is_match(source);
    if base64_decoding {
        suspicious.push("Base64 encoding/decoding detected".to_string());
    }

    // String concatenation (potential obfuscation)
    let re_concat = Regex::new(r#"['"][^'"]*['"]\s*\+\s*['"]"#).unwrap();
    let string_concatenation = re_concat.is_match(source);
    if string_concatenation {
        suspicious.push("String concatenation detected".to_string());
    }

    // Function constructor
    let re_fn_constructor = Regex::new(r"new\s+Function\s*\(").unwrap();
    if re_fn_constructor.is_match(source) {
        suspicious.push("Function() constructor used".to_string());
    }

    ObfuscationReport {
        eval_usage,
        base64_decoding,
        string_concatenation,
        suspicious_patterns: suspicious,
    }
}
