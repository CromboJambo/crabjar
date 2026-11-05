use crabjar_web_search::search;

#[tokio::main]
async fn main() {
    // Set SearXNG URL from environment or use default
    unsafe {
        std::env::set_var("SEARXNG_URL", "http://ftw3:8080");
    }

    println!("Testing crabjar web search with self-hosted SearXNG...");
    println!();

    match search("rust programming language", Some(3)).await {
        Ok(response) => {
            println!("Query: {}", response.query);
            println!("Results found: {}", response.total_results);
            println!();

            for (i, result) in response.results.iter().enumerate() {
                println!("{}. {}", i + 1, result.title);
                println!("   URL: {}", result.url);
                if let Some(desc) = &result.description {
                    let truncated: String = desc.chars().take(150).collect();
                    println!("   Description: {}...", truncated);
                }
                println!();
            }

            println!("✓ Web search integration working!");
        }
        Err(e) => {
            println!("✗ Error: {}", e);
            std::process::exit(1);
        }
    }
}
