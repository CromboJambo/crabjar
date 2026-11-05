use super::compressor::ContextCompressor;
use super::config::CompressionConfig;
use chrono::Utc;
use crabjar_host_core::work_item::Observation;

fn make_obs(stage: &str, kind: &str, details: &str) -> Observation {
    Observation {
        id: uuid::Uuid::new_v4(),
        stage: stage.to_string(),
        kind: kind.to_string(),
        details: details.to_string(),
        timestamp: Utc::now(),
    }
}

#[test]
fn test_compress_disabled() {
    let config = CompressionConfig::disabled();
    let compressor = ContextCompressor::new(config);

    let obs: Vec<_> = (0..5)
        .map(|i| make_obs("observe", "state", &format!("observation {}", i)))
        .collect();

    let compressed = compressor.compress(&obs);
    assert!(!compressed.is_empty());
}

#[test]
fn test_compress_small_input() {
    let config = CompressionConfig::default();
    let compressor = ContextCompressor::new(config);

    // Fewer observations than recent_count — should not compress
    let obs: Vec<_> = (0..3)
        .map(|i| make_obs("observe", "state", &format!("observation {}", i)))
        .collect();

    let compressed = compressor.compress(&obs);
    assert!(compressed.contains("observation 0"));
    assert!(compressed.contains("observation 1"));
    assert!(compressed.contains("observation 2"));
}

#[test]
fn test_compress_large_input() {
    let config = CompressionConfig {
        recent_count: 3,
        max_summaries: 2,
        ..CompressionConfig::default()
    };
    let compressor = ContextCompressor::new(config);

    // Many observations — should compress
    let obs: Vec<_> = (0..20)
        .map(|i| make_obs("observe", "state", &format!("observation {}", i)))
        .collect();

    let compressed = compressor.compress(&obs);
    assert!(compressed.contains("Summary of earlier"));
    assert!(compressed.contains("Recent observations"));
}

#[test]
fn test_compress_empty() {
    let compressor = ContextCompressor::default_compressor();
    let result = compressor.compress(&[]);
    assert_eq!(result, "");
}

#[test]
fn test_compressor_default() {
    let compressor = ContextCompressor::default_compressor();
    assert!(compressor.compress(&[]) == compressor.compress(&[]));
}

#[test]
fn test_summary_grouping() {
    let config = CompressionConfig {
        recent_count: 2,
        max_summaries: 5,
        ..CompressionConfig::default()
    };
    let compressor = ContextCompressor::new(config);

    let obs: Vec<_> = (0..10)
        .map(|i| {
            let stage = if i < 5 { "plan" } else { "execute" };
            let kind = if i % 2 == 0 { "tool" } else { "result" };
            make_obs(stage, kind, &format!("detail {}", i))
        })
        .collect();

    let compressed = compressor.compress(&obs);
    assert!(compressed.contains("Summary of earlier"));
    assert!(compressed.contains("plan"));
    assert!(compressed.contains("execute"));
}

#[test]
fn test_token_budget_enforcement() {
    let config = CompressionConfig {
        max_tokens: 100, // ~400 chars
        recent_count: 3,
        ..CompressionConfig::default()
    };
    let compressor = ContextCompressor::new(config.clone());

    let obs: Vec<_> = (0..50)
        .map(|i| make_obs("observe", "state", &format!("observation {}", i)))
        .collect();

    let compressed = compressor.compress(&obs);
    // Should be truncated to fit budget
    assert!(compressed.len() <= config.max_tokens * 4 + 50); // +50 for truncation message
}

#[test]
fn test_config_short_conversation() {
    let config = CompressionConfig::for_short_conversation();
    assert_eq!(config.max_tokens, 2048);
    assert_eq!(config.recent_count, 5);
    assert_eq!(config.max_summaries, 3);
    assert!(config.enabled);
}

#[test]
fn test_config_long_conversation() {
    let config = CompressionConfig::for_long_conversation();
    assert_eq!(config.max_tokens, 8192);
    assert_eq!(config.recent_count, 20);
    assert_eq!(config.max_summaries, 10);
    assert!(config.enabled);
}

#[test]
fn test_compression_disabled_config() {
    let config = CompressionConfig::disabled();
    assert!(!config.enabled);
}

#[test]
fn test_structural_compression_for_code_observations() {
    let config = CompressionConfig {
        recent_count: 3,
        max_summaries: 5,
        ..CompressionConfig::default()
    };
    let compressor = ContextCompressor::new(config);

    // Create observations with code-heavy content that should trigger structural compression
    let large_function = "fn process_large_data(items: Vec<i32>) -> i32 {\n\
        let mut sum = 0;\n\
        for item in items {\n\
            sum += item;\n\
        }\n\
        return sum * 2;\n\
    }";

    // Place code observation early enough to be in "older" bucket (indices 0-11 with recent_count=3)
    let obs: Vec<_> = (0..15)
        .map(|i| {
            if i == 5 {
                make_obs("execute", "tool", large_function)
            } else {
                make_obs(
                    "observe",
                    "state",
                    &format!("observation {} — some regular text", i),
                )
            }
        })
        .collect();

    let compressed = compressor.compress(&obs);

    // Should have summarized older observations including the code-heavy one
    assert!(compressed.contains("Summary of earlier"));
    // The code observation should have been structurally compressed
    // (not just truncated to first 200 chars)
    assert!(!compressed.contains("let mut sum = 0"));
}

#[test]
fn test_non_code_observations_use_simple_truncation() {
    let config = CompressionConfig {
        recent_count: 3,
        max_summaries: 5,
        ..CompressionConfig::default()
    };
    let compressor = ContextCompressor::new(config);

    // Long non-code text
    let long_text = "This is a long observation with no code patterns. It's just regular text that goes on and on without any Rust-like keywords that would trigger structural compression.";

    let obs_code = make_obs("execute", "tool", long_text);

    let obs: Vec<_> = (0..15)
        .map(|i| {
            if i == 12 {
                return obs_code.clone();
            }
            make_obs(
                "observe",
                "state",
                &format!("observation {} — some regular text", i),
            )
        })
        .collect();

    let compressed = compressor.compress(&obs);

    // Non-code observations use simple truncation
    assert!(compressed.contains("Summary of earlier"));
    // Should truncate but not apply structural compression markers
    assert!(!compressed.contains("[compressed code:"));
}
