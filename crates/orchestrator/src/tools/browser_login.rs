//! Browser login tool — handles login walls by combining vault credentials with form automation.
//!
//! Workflow:
//! 1. Look up credentials from Bitwarden/Vaultwarden for the current URL
//! 2. Detect login form fields on the page (username/password inputs)
//! 3. Fill in the fields and submit the form
//!
//! Arguments are passed as a JSON string in args[0]:
//!   {"url": "https://example.com/login", "username_field": "...", "password_field": "..."}
//!
//! Optional overrides for auto-detection:
//!   - username_field: CSS selector for username input (auto-detected if omitted)
//!   - password_field: CSS selector for password input (auto-detected if omitted)
//!   - submit_selector: CSS selector for submit button (auto-detected if omitted)

use crabjar_browser_vault::{BrowserVaultClient, VaultConfig};
use serde_json::json;
use std::sync::Arc;
use std::sync::OnceLock;
use tracing::info;

/// Cached vault client (created once per process).
static VAULT_CLIENT: OnceLock<Arc<BrowserVaultClient>> = OnceLock::new();

fn get_client() -> &'static Arc<BrowserVaultClient> {
    VAULT_CLIENT.get_or_init(|| {
        let config = VaultConfig::from_env();
        info!(cli = %config.cli_path, "Initializing browser vault client for login");
        Arc::new(BrowserVaultClient::new(config))
    })
}

/// Handle a browser_login tool call.
pub async fn handle(args: &[String]) -> Result<String, String> {
    if args.is_empty() {
        return Err("browser_login requires a JSON argument".to_string());
    }

    let request: serde_json::Value = serde_json::from_str(&args[0])
        .map_err(|e| format!("Failed to parse browser_login arguments as JSON: {}", e))?;

    let url = request
        .get("url")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing required field: 'url'".to_string())?;

    let username_field = request.get("username_field").and_then(|v| v.as_str());
    let password_field = request.get("password_field").and_then(|v| v.as_str());
    let submit_selector = request.get("submit_selector").and_then(|v| v.as_str());

    info!(url = %url, "Starting browser login flow");

    // Step 1: Look up credentials from vault
    let client = get_client();

    // Check if vault is available/unlocked
    let available = client
        .is_available()
        .await
        .map_err(|e| format!("Failed to check vault availability: {}", e))?;

    if !available {
        return Err(
            "Vault is not unlocked. Run 'bw unlock' first, then retry browser_login.".to_string(),
        );
    }

    let item = client
        .get_by_url(url)
        .await
        .map_err(|e| format!("No vault credentials found for URL '{}': {}", url, e))?;

    let username = item
        .username
        .as_ref()
        .ok_or_else(|| format!("Vault item '{}' has no username field", item.name))?;
    let password = item
        .password
        .as_ref()
        .ok_or_else(|| format!("Vault item '{}' has no password field", item.name))?;

    info!(vault_item = %item.name, "Found vault credentials for login");

    // Step 2: Detect login form fields if not provided
    let (user_sel, pass_sel, submit_sel) = detect_login_form(
        url,
        username_field,
        password_field,
        submit_selector,
    )
    .await?;

    info!(
        user_selector = %user_sel,
        pass_selector = %pass_sel,
        submit_selector = %submit_sel,
        "Login form fields identified"
    );

    // Step 3: Build and execute the login flow as browser code
    let login_code = format!(
        r#"
// Login flow for {url}
const userSelector = {user_sel_json};
const passSelector = {pass_sel_json};
const submitSelector = {submit_sel_json};

async function login() {{
    // Wait for the page to be ready
    await new Promise(r => setTimeout(r, 500));

    // Fill username
    const userField = document.querySelector(userSelector);
    if (!userField) throw new Error(`Username field not found: ${{userSelector}}`);
    userField.focus();
    userField.value = '';
    userField.dispatchEvent(new Event('input', {{ bubbles: true }}));

    // Fill password
    const passField = document.querySelector(passSelector);
    if (!passField) throw new Error(`Password field not found: ${{passSelector}}`);
    passField.focus();
    passField.value = '';
    passField.dispatchEvent(new Event('input', {{ bubbles: true }}));

    // Submit the form (or click submit button)
    const form = userField.closest('form');
    if (form) {{
        form.submit();
    }} else {{
        const submitBtn = document.querySelector(submitSelector);
        if (submitBtn) {{
            submitBtn.click();
        }} else {{
            throw new Error('No form or submit button found');
        }}
    }}

    return {{ success: true, message: "Login submitted for ${{url}}" }};
}}

login().then(r => console.log(JSON.stringify(r))).catch(e => {{
    console.error("LOGIN_ERROR:", e.message);
    throw e;
}});
"#,
        url = url,
        user_sel_json = serde_json::to_string(&user_sel).unwrap(),
        pass_sel_json = serde_json::to_string(&pass_sel).unwrap(),
        submit_sel_json = serde_json::to_string(&submit_sel).unwrap(),
    );

    // Set the credentials via environment for the browser process to use
    // We inject them directly into the JS code to avoid shell escaping issues
    let login_code_with_creds = login_code.replace(
        "userField.value = '';",
        &format!("userField.value = {};", serde_json::to_string(username).unwrap()),
    );
    let login_code_final = login_code_with_creds.replace(
        "passField.value = '';",
        &format!("passField.value = {};", serde_json::to_string(password).unwrap()),
    );

    // Execute the login flow in the browser
    match crabjar_browser::execute_code(&login_code_final).await {
        Ok(result) => {
            let response = json!({
                "status": "success",
                "url": url,
                "vault_item": item.name,
                "username": username,
                "user_selector": user_sel,
                "pass_selector": pass_sel,
                "submit_selector": submit_sel,
                "browser_result": result
            });
            info!(url = %url, "Login flow completed successfully");
            serde_json::to_string_pretty(&response)
                .map_err(|e| format!("Failed to serialize response: {}", e))
        }
        Err(e) => {
            let response = json!({
                "status": "error",
                "url": url,
                "vault_item": item.name,
                "username": username,
                "user_selector": user_sel,
                "pass_selector": pass_sel,
                "submit_selector": submit_sel,
                "error": e.to_string()
            });
            Err(format!(
                "Login flow failed for '{}': {}",
                url,
                serde_json::to_string(&response).unwrap_or_default()
            ))
        }
    }
}

/// Detect login form fields on the page.
///
/// Uses browser JS to find username/password inputs and submit buttons.
/// Returns CSS selectors for each field.
async fn detect_login_form(
    url: &str,
    username_field: Option<&str>,
    password_field: Option<&str>,
    submit_selector: Option<&str>,
) -> Result<(String, String, String), String> {
    // Use provided selectors if available
    let user_sel = username_field
        .map(|s| s.to_string())
        .unwrap_or_else(|| "input[name='username'], input[name='email'], input[type='text'][name*='user'], #username, #email".to_string());

    let pass_sel = password_field
        .map(|s| s.to_string())
        .unwrap_or_else(|| "input[type='password'], input[name='password'], #password".to_string());

    let submit_sel = submit_selector
        .map(|s| s.to_string())
        .unwrap_or_else(|| "button[type='submit'], input[type='submit'], button:contains('Login'), button:contains('Sign in')".to_string());

    // Verify the selectors work on the current page via browser JS
    let user_sel_json = serde_json::to_string(&user_sel).unwrap();
    let pass_sel_json = serde_json::to_string(&pass_sel).unwrap();
    let submit_sel_json = serde_json::to_string(&submit_sel).unwrap();

    let detect_code = format!(
        r#"
async function detectLoginForm() {{
    const userSelector = {user_sel_json};
    const passSelector = {pass_sel_json};
    const submitSelector = {submit_sel_json};

    // Find username field
    const userField = document.querySelector(userSelector);
    if (!userField) {{
        // Try broader search
        const allInputs = Array.from(document.querySelectorAll('input'));
        const likelyUser = allInputs.find(i =>
            i.type === 'text' || i.type === 'email' ||
            (i.name && (i.name.includes('user') || i.name.includes('email')))
        );
        if (!likelyUser) throw new Error("No username field detected on page");
        // Generate a selector for it
        const id = likelyUser.id ? `#${{likelyUser.id}}` : null;
        const name = likelyUser.name ? `[name='${{likelyUser.name}}']` : null;
        if (id) return JSON.stringify({{ user: id, pass: "input[type='password']", submit: "button[type='submit'], input[type='submit']" }});
        if (name) return JSON.stringify({{ user: name, pass: "input[type='password']", submit: "button[type='submit'], input[type='submit']" }});
        throw new Error("Could not generate selector for username field");
    }}

    // Find password field
    const passField = document.querySelector(passSelector);
    if (!passField) {{
        throw new Error("No password field detected on page");
    }}

    return JSON.stringify({{
        user: userSelector,
        pass: passSelector,
        submit: submitSelector
    }});
}}

detectLoginForm().then(r => console.log(r)).catch(e => {{
    console.error("DETECT_ERROR:", e.message);
    throw e;
}});
"#
    );

    match crabjar_browser::execute_code(&detect_code).await {
        Ok(result) => {
            // Parse the detection result
            let result_str = serde_json::to_string(&result).unwrap_or_default();
            if let Some(detection) = extract_detection_result(&result_str) {
                Ok((detection.user, detection.pass, detection.submit))
            } else {
                // Fall back to provided/default selectors
                Ok((user_sel, pass_sel, submit_sel))
            }
        }
        Err(e) => {
            // Detection failed — fall back to provided/default selectors
            tracing::warn!(error = %e, "Login form detection failed, using default selectors");
            Ok((user_sel, pass_sel, submit_sel))
        }
    }
}

/// Extract detection result from browser output.
fn extract_detection_result(output: &str) -> Option<DetectionResult> {
    // Look for JSON in the output
    let json_start = output.find('{')?;
    let json_end = output.rfind('}')? + 1;
    if json_end <= json_start {
        return None;
    }

    let json_str = &output[json_start..json_end];
    serde_json::from_str(json_str).ok()
}

#[derive(serde::Deserialize)]
struct DetectionResult {
    user: String,
    pass: String,
    submit: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_detection_result_valid() {
        let output = "DETECT result: {\"user\": \"#username\", \"pass\": \"input[type='password']\", \"submit\": \"button[type='submit']\"}";
        let result = extract_detection_result(output);
        assert!(result.is_some());
        let det = result.unwrap();
        assert_eq!(det.user, "#username");
        assert_eq!(det.pass, "input[type='password']");
        assert_eq!(det.submit, "button[type='submit']");
    }

    #[test]
    fn test_extract_detection_result_no_json() {
        let output = "No JSON here";
        assert!(extract_detection_result(output).is_none());
    }

    #[test]
    fn test_extract_detection_result_empty() {
        assert!(extract_detection_result("").is_none());
    }

    #[test]
    fn test_default_selectors_are_reasonable() {
        let (user, pass, submit) = (
            "input[name='username'], input[name='email'], input[type='text'][name*='user'], #username, #email",
            "input[type='password'], input[name='password'], #password",
            "button[type='submit'], input[type='submit']",
        );
        assert!(user.contains("username"));
        assert!(pass.contains("password"));
        assert!(submit.contains("submit"));
    }
}
