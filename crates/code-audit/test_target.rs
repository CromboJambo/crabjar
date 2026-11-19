// Test file with intentional vulnerabilities for bug-bounty-hunter testing

fn main() {
    let password = "super_secret_password_123";
    let api_key = "sk-1234567890abcdef";

    let query = build_query("1".to_string());
    println!("{}", query);
}

fn build_query(user_id: String) -> String {
    "SELECT * FROM users WHERE id = " + user_id
}

fn process_data(data: Vec<u8>) {
    let obj = serde_json::from_slice(&data).unwrap();
    // No validation of deserialized object
}