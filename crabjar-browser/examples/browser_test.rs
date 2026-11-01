use crabjar_browser::navigate;

#[tokio::main]
async fn main() {
    println!("Testing crabjar browser automation with Playwright...");
    println!();

    // Test navigation to a real website
    match navigate("https://example.com").await {
        Ok(result) => {
            println!("✓ Navigation successful!");
            println!("URL: {}", result.url);
            println!("Title: {}", result.title);
            println!("Content (first 200 chars):");
            let preview: String = result.content.chars().take(200).collect();
            println!("{}", preview);
            println!();
        }
        Err(e) => {
            println!("✗ Error: {}", e);
            std::process::exit(1);
        }
    }

    println!("Browser automation test complete!");
}