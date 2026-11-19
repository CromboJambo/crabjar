#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::Path;

    use super::super::peg_analyzer::{
        PegFeature, byte_to_line, count_params_from_captures, detect_language, estimate_depth,
        truncate,
    };

    #[test]
    fn test_detect_language_rust() {
        let path = Path::new("src/main.rs");
        assert_eq!(detect_language(path), "rust");
    }

    #[test]
    fn test_detect_language_python() {
        let path = Path::new("app/main.py");
        assert_eq!(detect_language(path), "python");
    }

    #[test]
    fn test_detect_language_unknown() {
        let path = Path::new("README.md");
        assert_eq!(detect_language(path), "unknown");
    }

    #[test]
    fn test_byte_to_line() {
        let source = "line1\nline2\nline3";
        assert_eq!(byte_to_line(source, 0), 1);
        assert_eq!(byte_to_line(source, 6), 2);
        assert_eq!(byte_to_line(source, 12), 3);
    }

    #[test]
    fn test_truncate_short() {
        assert_eq!(truncate("hello", 10), "hello");
    }

    #[test]
    fn test_truncate_long() {
        let long = "a".repeat(100);
        let result = truncate(&long, 10);
        assert_eq!(result.len(), 13); // 10 chars + "..."
        assert!(result.ends_with("..."));
    }

    #[test]
    fn test_count_params() {
        let mut captures = HashMap::new();
        captures.insert("params".to_string(), "x: i32, y: String".to_string());
        assert_eq!(count_params_from_captures(&captures), 2);
    }

    #[test]
    fn test_estimate_depth() {
        let leaf = PegFeature {
            rule: "leaf".to_string(),
            start: 0,
            end: 1,
            text: "x".to_string(),
            captures: HashMap::new(),
            children: Vec::new(),
        };
        assert_eq!(estimate_depth(&[leaf.clone()]), 1);

        let nested = PegFeature {
            rule: "parent".to_string(),
            start: 0,
            end: 2,
            text: "xy".to_string(),
            captures: HashMap::new(),
            children: vec![leaf],
        };
        assert_eq!(estimate_depth(&[nested]), 2);
    }
}
