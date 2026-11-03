//! JSON compression: preserve structure, truncate large values, cap array length.
//! Equivalent to Headroom's SmartCrusher.

use super::{CompressionConfig, CompressionResult};

/// Compress JSON content while preserving structure.
pub fn compress_json(text: &str, config: &CompressionConfig) -> CompressionResult {
    match serde_json::from_str::<serde_json::Value>(text.trim()) {
        Ok(value) => {
            let compressed = compress_value(&value, config);
            let output =
                serde_json::to_string_pretty(&compressed).unwrap_or_else(|_| text.to_string());
            CompressionResult::new(text, output, super::ContentType::Json)
        }
        Err(_) => CompressionResult::new(text, text.to_string(), super::ContentType::Json),
    }
}

fn compress_value(value: &serde_json::Value, config: &CompressionConfig) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => {
            let mut new_map = serde_json::Map::new();
            for (key, val) in map {
                new_map.insert(key.clone(), compress_value(val, config));
            }
            serde_json::Value::Object(new_map)
        }
        serde_json::Value::Array(arr) => {
            if arr.len() > config.json_array_limit {
                let mut truncated = Vec::new();
                for item in arr.iter().take(config.json_array_limit) {
                    truncated.push(compress_value(item, config));
                }
                // Add a marker for remaining items
                truncated.push(serde_json::Value::String(format!(
                    "... [{} more items]",
                    arr.len() - config.json_array_limit
                )));
                serde_json::Value::Array(truncated)
            } else {
                let compressed: Vec<serde_json::Value> = arr
                    .iter()
                    .map(|item| compress_value(item, config))
                    .collect();
                serde_json::Value::Array(compressed)
            }
        }
        serde_json::Value::String(s) => {
            if s.len() > config.json_string_limit {
                let truncated = &s[..config.json_string_limit];
                serde_json::Value::String(format!("{}... [truncated]", truncated))
            } else {
                value.clone()
            }
        }
        _ => value.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compress_large_array() {
        let json = r#"[1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22]"#;
        let config = CompressionConfig {
            json_array_limit: 5,
            ..Default::default()
        };
        let result = compress_json(json, &config);
        assert!(result.compressed.contains("[17 more items]"));
    }

    #[test]
    fn test_compress_long_string() {
        let long = "a".repeat(200);
        let json = format!(r#"{{"message": "{}"}}"#, long);
        let config = CompressionConfig {
            json_string_limit: 50,
            ..Default::default()
        };
        let result = compress_json(&json, &config);
        assert!(result.compressed.contains("[truncated]"));
    }
}
