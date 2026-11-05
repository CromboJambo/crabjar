use std::process;
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        eprintln!("Usage: hermes-struct-tok <command> [options]");
        process::exit(1);
    }
    
    match args[1].as_str() {
        "tokenize" => {
            if args.len() < 3 {
                eprintln!("Usage: hermes-struct-tok tokenize <file.rs>");
                process::exit(1);
            }
            let path = &args[2];
            match std::fs::read_to_string(path) {
                Ok(src) => {
                    let tokens = pesti_structural_tokenizer::tokenize(&src);
                    // Output as JSON array of token objects
                    let json: Vec<serde_json::Value> = tokens.iter().map(|t| {
                        serde_json::json!({
                            "kind": format!("{:?}", t.kind),
                            "text": t.text,
                            "span": [t.span.start, t.span.end]
                        })
                    }).collect();
                    println!("{}", serde_json::to_string_pretty(&json).unwrap());
                }
                Err(e) => {
                    eprintln!("Error reading {}: {}", path, e);
                    process::exit(1);
                }
            }
        }
        "encode" => {
            // Encode source to structural token IDs for LLM input
            if args.len() < 3 {
                eprintln!("Usage: hermes-struct-tok encode <file.rs>");
                process::exit(1);
            }
            let path = &args[2];
            match std::fs::read_to_string(path) {
                Ok(src) => {
                    let encoder = pesti_structural_tokenizer::StructuralEncoder::new();
                    let ids = encoder.encode(&src);
                    println!("{}", serde_json::to_string(&ids).unwrap());
                }
                Err(e) => {
                    eprintln!("Error reading {}: {}", path, e);
                    process::exit(1);
                }
            }
        }
        "decode" => {
            // Decode structural token IDs back to source (lossy)
            if args.len() < 3 {
                eprintln!("Usage: hermes-struct-tok decode <json-array-of-ids>");
                process::exit(1);
            }
            let ids: Vec<u32> = serde_json::from_str(&args[2]).unwrap();
            let decoder = pesti_structural_tokenizer::StructuralDecoder::new();
            let src = decoder.decode(&ids);
            print!("{}", src);
        }
        "vocab" => {
            // Print vocabulary size and token info
            let vocab = pesti_structural_tokenizer::VOCAB_SIZE;
            println!("Vocabulary size: {}", vocab);
        }
        _ => {
            eprintln!("Unknown command: {}", args[1]);
            process::exit(1);
        }
    }
}