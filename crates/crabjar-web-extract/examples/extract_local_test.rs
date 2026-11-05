use crabjar_web_extract::extract;

#[tokio::main]
async fn main() {
    println!("Testing crabjar web content extraction with local server...");
    println!();

    // Create a simple HTML file to test with
    let html_content = r#"
<!DOCTYPE html>
<html>
<head><title>Test Page</title></head>
<body>
<article>
<h1>Hello World</h1>
<p>This is a test paragraph with some <strong>bold text</strong> and <em>italic text</em>.</p>
<p>Another paragraph with a <a href="https://example.com">link</a>.</p>
<ul>
<li>List item 1</li>
<li>List item 2</li>
</ul>
</article>
</body>
</html>
"#;

    std::fs::write("/tmp/test_page.html", html_content).unwrap();

    // Start a simple HTTP server in the background using tokio
    let server = tokio::spawn(async move {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:8899")
            .await
            .unwrap();

        loop {
            let (mut stream, _) = listener.accept().await.unwrap();

            // Read request (we don't care about it)
            let mut buf = [0u8; 4096];
            use tokio::io::AsyncReadExt;
            stream.read(&mut buf).await.unwrap();

            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\n\r\n{}",
                html_content.len(),
                html_content
            );
            use tokio::io::AsyncWriteExt;
            stream.write_all(response.as_bytes()).await.unwrap();
        }
    });

    // Give the server a moment to start
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let urls = vec!["http://127.0.0.1:8899/test".to_string()];

    match extract(&urls, Some(1000)).await {
        Ok(response) => {
            for result in response.results {
                println!("URL: {}", result.url);
                println!("Title: {}", result.title);
                if let Some(err) = &result.error {
                    println!("Error: {}", err);
                } else {
                    println!("Content:");
                    println!("{}", result.content);
                }
                println!();
            }

            println!("✓ Web extraction working!");
        }
        Err(e) => {
            println!("✗ Error: {}", e);
            std::process::exit(1);
        }
    }

    server.abort();
}
