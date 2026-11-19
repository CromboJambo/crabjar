#[cfg(test)]
mod tests {
#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageEncoder, RgbaImage};

    use std::sync::{Mutex, OnceLock};

    /// Static mutex to serialize env-var tests (they modify global state).
    static ENV_TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    fn env_test_lock() -> std::sync::MutexGuard<'static, ()> {
        ENV_TEST_LOCK.get_or_init(Mutex::default).lock().unwrap()
    }

    #[test]
    fn test_detect_mime_from_extension() {
        assert_eq!(detect_mime_from_extension("/path/to/image.png"), "image/png");
        assert_eq!(detect_mime_from_extension("/path/to/photo.jpg"), "image/jpeg");
        assert_eq!(detect_mime_from_extension("/path/to/pic.jpeg"), "image/jpeg");
        assert_eq!(detect_mime_from_extension("/path/to/anim.gif"), "image/gif");
        assert_eq!(detect_mime_from_extension("/path/to/img.webp"), "image/webp");
        assert_eq!(detect_mime_from_extension("/path/to/file"), "image/png"); // Default
    }

    #[test]
    fn test_vision_config_from_env_defaults() {
        let _lock = env_test_lock();
        // Clear any existing env vars
        unsafe {
            std::env::remove_var("VISION_API_URL");
            std::env::remove_var("UNSLOTH_STUDIO_URL");
            std::env::remove_var("VISION_MODEL");
            std::env::remove_var("UNSLOTH_STUDIO_MODEL");
        }

        let config = VisionConfig::from_env().unwrap();
        assert_eq!(config.base_url, "http://localhost:8081/v1");
        assert_eq!(config.model, "llama-3.2-vision");
        assert!(config.api_key.is_none());
    }

    #[test]
    fn test_vision_config_with_custom_values() {
        let _lock = env_test_lock();
        unsafe {
            std::env::set_var("VISION_API_URL", "http://custom-host:8080/v1");
            std::env::set_var("VISION_MODEL", "my-vision-model");
            std::env::set_var("VISION_API_KEY", "test-key-123");
        }

        let config = VisionConfig::from_env().unwrap();
        assert_eq!(config.base_url, "http://custom-host:8080/v1");
        assert_eq!(config.model, "my-vision-model");
        assert_eq!(config.api_key, Some("test-key-123".to_string()));

        // Cleanup
        unsafe {
            std::env::remove_var("VISION_API_URL");
            std::env::remove_var("VISION_MODEL");
            std::env::remove_var("VISION_API_KEY");
        }
    }

    #[test]
    fn test_vision_config_uses_unsloth_studio_fallback() {
        let _lock = env_test_lock();
        // Clear VISION_* vars, set UNSLOTH_STUDIO_* vars
        unsafe {
            std::env::remove_var("VISION_API_URL");
            std::env::remove_var("VISION_MODEL");
            std::env::remove_var("VISION_API_KEY");

            std::env::set_var("UNSLOTH_STUDIO_URL", "http://ftw3:8081/v1");
            std::env::set_var("UNSLOTH_STUDIO_MODEL", "unsloth-vision-model");
            std::env::set_var("UNSLOTH_STUDIO_API_KEY", "unsloth-key");
        }

        let config = VisionConfig::from_env().unwrap();
        assert_eq!(config.base_url, "http://ftw3:8081/v1");
        assert_eq!(config.model, "unsloth-vision-model");
        assert_eq!(config.api_key, Some("unsloth-key".to_string()));

        // Cleanup
        unsafe {
            std::env::remove_var("UNSLOTH_STUDIO_URL");
            std::env::remove_var("UNSLOTH_STUDIO_MODEL");
            std::env::remove_var("UNSLOTH_STUDIO_API_KEY");
        }
    }

    #[test]
    fn test_build_vision_request() {
        let config = VisionConfig {
            base_url: "http://localhost:1234/v1".to_string(),
            model: "test-model".to_string(),
            api_key: None,
            timeout_secs: 60,
            max_tokens: 1024,
        };
        let client = VisionClient {
            config,
            http_client: reqwest::Client::new(),
        };

        let request = client
            .build_vision_request(
                "data:image/png;base64,TESTDATA",
                "image/png",
                "What is in this image?",
            )
            .unwrap();

        assert_eq!(request["model"], "test-model");
        assert_eq!(request["max_tokens"], 1024);
        assert_eq!(request["messages"].as_array().unwrap().len(), 2);

        // Verify user message has both text and image parts
        let user_msg = &request["messages"][1];
        let content = user_msg["content"].as_array().unwrap();
        assert_eq!(content.len(), 2);
        assert_eq!(content[0]["type"], "text");
        assert_eq!(content[1]["type"], "image_url");
    }

    #[test]
    fn test_parse_response() {
        let config = VisionConfig {
            base_url: "http://localhost:1234/v1".to_string(),
            model: "test-model".to_string(),
            api_key: None,
            timeout_secs: 60,
            max_tokens: 1024,
        };
        let client = VisionClient {
            config,
            http_client: reqwest::Client::new(),
        };

        let response = serde_json::json!({
            "choices": [
                {
                    "message": {
                        "content": "The image shows a cat sitting on a chair."
                    }
                }
            ],
            "usage": {
                "completion_tokens": 42
            }
        });

        let analysis = client.parse_response(&response, "/test/image.png").unwrap();
        assert_eq!(analysis.analysis, "The image shows a cat sitting on a chair.");
        assert_eq!(analysis.model, "test-model");
        assert_eq!(analysis.source, "/test/image.png");
        assert_eq!(analysis.tokens_used, Some(42));
    }

    #[tokio::test]
    async fn test_analyze_missing_file() {
        let config = VisionConfig {
            base_url: "http://localhost:1234/v1".to_string(),
            model: "test-model".to_string(),
            api_key: None,
            timeout_secs: 60,
            max_tokens: 1024,
        };
        let client = VisionClient::new(config);

        let result = client.analyze("/nonexistent/image.png", "What is this?").await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            VisionError::ImageLoadError { .. }
        ));
    }

    #[test]
    fn test_crop_image_region_valid() {
        // Create a simple 10x10 red PNG image
        let mut img = image::RgbaImage::new(10, 10);
        for y in 0..10 {
            for x in 0..10 {
                img.put_pixel(x, y, image::Rgba([255, 0, 0, 255])); // Red
            }
        }

        let mut png_bytes = Vec::new();
        {
            let encoder = image::codecs::png::PngEncoder::new(&mut png_bytes);
            encoder.write_image(
                img.as_raw(),
                10,
                10,
                image::ExtendedColorType::Rgba8,
            ).unwrap();
        }

        // Crop a 5x5 region from the center
        let cropped = crop_image_region(&png_bytes, 2, 2, 7, 7).unwrap();
        assert!(!cropped.is_empty());

        // Verify the cropped image is valid PNG with correct dimensions
        let decoded = image::load_from_memory(&cropped).unwrap();
        assert_eq!(decoded.width(), 5);
        assert_eq!(decoded.height(), 5);

        // Verify pixel color is preserved (red)
        let rgba = decoded.to_rgba8();
        let pixel = rgba.get_pixel(0, 0);
        assert_eq!(pixel[0], 255); // R
        assert_eq!(pixel[1], 0);   // G
        assert_eq!(pixel[2], 0);   // B
    }

    #[test]
    fn test_crop_image_region_clamps_to_bounds() {
        // Create a 10x10 image
        let mut img = image::RgbaImage::new(10, 10);
        for y in 0..10 {
            for x in 0..10 {
                img.put_pixel(x, y, image::Rgba([0, 255, 0, 255])); // Green
            }
        }

        let mut png_bytes = Vec::new();
        {
            let encoder = image::codecs::png::PngEncoder::new(&mut png_bytes);
            encoder.write_image(
                img.as_raw(),
                10,
                10,
                image::ExtendedColorType::Rgba8,
            ).unwrap();
        }

        // Request a region that extends beyond bounds — should clamp
        let cropped = crop_image_region(&png_bytes, 5, 5, 20, 20).unwrap();
        let decoded = image::load_from_memory(&cropped).unwrap();
        assert_eq!(decoded.width(), 5);  // Clamped to 10-5=5
        assert_eq!(decoded.height(), 5); // Clamped to 10-5=5
    }

    #[test]
    fn test_crop_image_region_invalid_coords() {
        // Create a simple image
        let mut img = image::RgbaImage::new(10, 10);
        for y in 0..10 {
            for x in 0..10 {
                img.put_pixel(x, y, image::Rgba([0, 0, 255, 255])); // Blue
            }
        }

        let mut png_bytes = Vec::new();
        {
            let encoder = image::codecs::png::PngEncoder::new(&mut png_bytes);
            encoder.write_image(
                img.as_raw(),
                10,
                10,
                image::ExtendedColorType::Rgba8,
            ).unwrap();
        }

        // x1 >= x2 after clamping should produce zero-width crop -> RegionError
        let result = crop_image_region(&png_bytes, 7, 2, 5, 7);
        assert!(matches!(result, Err(VisionError::RegionError(_))));
    }

    #[test]
    fn test_crop_image_region_invalid_input() {
        let result = crop_image_region(&[0xFF, 0xD8, 0xFF], 0, 0, 5, 5);
        assert!(matches!(result, Err(VisionError::ImageProcessingError(_))));
    }

    #[tokio::test]
    async fn test_analyze_with_region_invalid_coords() {
        let config = VisionConfig {
            base_url: "http://localhost:1234/v1".to_string(),
            model: "test-model".to_string(),
            api_key: None,
            timeout_secs: 60,
            max_tokens: 1024,
        };
        let client = VisionClient::new(config);

        // Create a temp image file with .png extension
        let mut img = image::RgbaImage::new(100, 100);
        for y in 0..100 {
            for x in 0..100 {
                img.put_pixel(x, y, image::Rgba([255, 255, 255, 255]));
            }
        }

        let tmp_dir = tempfile::tempdir().unwrap();
        let path = tmp_dir.path().join("test_image.png");
        img.save(&path).unwrap();
        let path_str = path.to_str().unwrap();

        // Invalid region: x1 >= x2
        let result = client.analyze_with_region(path_str, "What is this?", &[50, 0, 30, 100]).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), VisionError::RegionError(_)));
    }
}
