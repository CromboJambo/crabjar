//! Thumbnail generation from captured frames.
//!
//! Converts RGBA8 frame data to JPEG for efficient transport over HTTP.
//! Uses `image` crate's JPEG encoder with quality tuning for small previews.

use anyhow::{Context, Result};
use image::codecs::jpeg::JpegEncoder;
use std::io::Cursor;

use super::vnc_client::Frame;

/// Generate a JPEG thumbnail from a frame.
///
/// * `frame` - The source frame (will be downscaled if larger than `max_dim`)
/// * `max_dim` - Maximum width or height of the output (preserves aspect ratio)
/// * `quality` - JPEG quality 1-100 (75 is a good balance for previews)
pub fn generate_thumbnail(frame: &Frame, max_dim: u32, quality: u8) -> Result<Vec<u8>> {
    let scaled = frame.downscale(max_dim);

    // JPEG doesn't support alpha — composite onto white background.
    let mut rgb = Vec::with_capacity((scaled.width * scaled.height * 3) as usize);
    for chunk in scaled.pixels.chunks(4) {
        if chunk.len() < 4 {
            break;
        }
        let a = chunk[3] as f64 / 255.0;
        // Premultiplied composite over white.
        rgb.push((chunk[0] as f64 * a + 255.0 * (1.0 - a)) as u8);
        rgb.push((chunk[1] as f64 * a + 255.0 * (1.0 - a)) as u8);
        rgb.push((chunk[2] as f64 * a + 255.0 * (1.0 - a)) as u8);
    }

    let mut output = Cursor::new(Vec::new());
    let mut encoder = JpegEncoder::new_with_quality(&mut output, quality.clamp(1, 100));
    encoder
        .encode(
            &rgb,
            scaled.width,
            scaled.height,
            image::ExtendedColorType::Rgb8,
        )
        .context("JPEG encode failed")?;

    Ok(output.into_inner())
}

/// Generate a small preview thumbnail (160px max dim, quality 60).
/// Suitable for UI list views and status displays.
#[allow(dead_code)]
pub fn generate_preview(frame: &Frame) -> Result<Vec<u8>> {
    generate_thumbnail(frame, 160, 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thumbnail_of_small_frame() {
        // 4x4 solid red frame (16 pixels * 4 bytes = 64 bytes).
        let mut pixels = vec![0u8; 64];
        for i in (0..64).step_by(4) {
            pixels[i] = 255; // R
            pixels[i + 3] = 255; // A
        }
        let frame = Frame {
            width: 4,
            height: 4,
            pixels,
        };

        let jpeg = generate_thumbnail(&frame, 160, 75).unwrap();
        assert!(!jpeg.is_empty());
        // JPEG magic bytes.
        assert_eq!(&jpeg[0..2], &[0xFF, 0xD8]);
    }

    #[test]
    fn downscale_preserves_aspect_ratio() {
        let frame = Frame {
            width: 1920,
            height: 1080,
            pixels: vec![0u8; 1920 * 1080 * 4],
        };

        let scaled = frame.downscale(320);
        assert_eq!(scaled.width, 320);
        assert_eq!(scaled.height, 180); // 16:9 preserved
    }

    #[test]
    fn downscale_noop_when_small() {
        let frame = Frame {
            width: 100,
            height: 50,
            pixels: vec![0u8; 100 * 50 * 4],
        };

        let scaled = frame.downscale(320);
        assert_eq!(scaled.width, 100);
        assert_eq!(scaled.height, 50);
    }
}
