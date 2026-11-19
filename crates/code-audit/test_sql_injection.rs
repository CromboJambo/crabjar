// Test file with intentional SQL injection vulnerabilities

fn get_user_by_id(id: i32) {
    // Vulnerable: string concatenation in SQL query
    let query = "SELECT * FROM users WHERE id=" + &id.to_string();
}

fn search_users(name: &str) {
    // Vulnerable: string concatenation with user input
    let query = "SELECT * FROM users WHERE name LIKE '%" + name + "%'";
}

fn delete_user(id: i32) {
    // Vulnerable: string concatenation in DELETE
    let query = "DELETE FROM users WHERE id=" + &id.to_string();
}

// Safe: parameterized query (should not be flagged)
fn get_user_safe(id: i32) {
    let query = "SELECT * FROM users WHERE id = ?";
}
