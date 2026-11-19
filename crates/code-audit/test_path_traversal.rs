// Test file with intentional path traversal vulnerabilities

fn read_config(filename: &str) {
    // Vulnerable: string concatenation in file path
    let path = "/etc/config/" + filename;
}

fn write_log(message: &str, user_id: &str) {
    // Vulnerable: string concatenation with user input
    let path = "/var/log/users/" + user_id + ".log";
}

fn load_data(file: &str) {
    // Vulnerable: string concatenation in data path
    let path = "./data/" + file;
}

// Safe: Path::join (should not be flagged)
fn read_config_safe(filename: &str) {
    let path = std::path::Path::new("/etc/config").join(filename);
}
