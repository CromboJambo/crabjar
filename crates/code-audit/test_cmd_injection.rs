// Test file with intentional command injection vulnerabilities

fn list_files(directory: &str) {
    // Vulnerable: string concatenation in shell command
    let cmd = "ls -la " + directory;
}

fn search_files(pattern: &str, path: &str) {
    // Vulnerable: string concatenation with user input
    let cmd = "grep -r \"" + pattern + "\" " + path;
}

fn delete_file(filename: &str) {
    // Vulnerable: string concatenation in rm command
    let cmd = "rm -f " + filename;
}

// Safe: argument array (should not be flagged)
fn list_files_safe(directory: &str) {
    let args = ["ls", "-la", directory];
}
