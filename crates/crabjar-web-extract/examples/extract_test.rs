use crabjar_web_extract::extract;

#[tokio::main]
async fn main() {
    println!("Testing crabjar web content extraction...");
    println!();

    let urls = vec!["https://en.wikipedia.org/wiki/Rust_(programming_language)".to_string()];

    match extract(&urls, Some(500)).await {
        Ok(response) => {
            for result in response.results {
                println!("URL: {}", result.url);
                println!("Title: {}", result.title);
                if let Some(err) = &result.error {
                    println!("Error: {err}");
                } else {
                    println!("Content (first 500 chars):");
                    println!(
                        "{}",
                        &result.content[..std::cmp::min(500, result.content.len())]
                    );
                }
                println!();
            }

            println!("✓ Web extraction working!");
        }
        Err(e) => {
            println!("✗ Error: {e}");
            std::process::exit(1);
        }
    }
}
