use regex::Regex;

pub(crate) fn extract_strings(data: &[u8]) -> Vec<String> {
    let re = Regex::new(r"[A-Za-z0-9_][A-Za-z0-9_]{3,}").unwrap();
    let mut strings = Vec::new();
    let mut i = 0;
    while i < data.len() {
        if let Some(m) = re.find(&String::from_utf8_lossy(&data[i..])) {
            let s = m.as_str().to_string();
            if !strings.contains(&s) {
                strings.push(s);
            }
            i += m.end();
        } else {
            break;
        }
    }
    strings.truncate(1000);
    strings
}
