//! Image processing helpers for crabjar-vision.

use crate::error::{VisionError, VisionResult};

/// Detect MIME type from file extension.
pub(crate) fn detect_mime_from_extension(path: &str) -> String {
    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "png" => "image/png".to_string(),
        "jpg" | "jpeg" => "image/jpeg".to_string(),
        "gif" => "image/gif".to_string(),
        "webp" => "image/webp".to_string(),
        "bmp" => "image/bmp".to_string(),
        "tiff" | "tif" => "image/tiff".to_string(),
        "svg" => "image/svg+xml".to_string(),
        _ => "image/png".to_string(), // Default to PNG
    }
}

/// Crop an image to the specified region and re-encode as PNG.
///
/// # Arguments
/// * `bytes` - Raw image bytes (PNG or JPEG)
/// * `x1, y1` - Top-left corner of the crop rectangle
/// * `x2, y2` - Bottom-right corner of the crop rectangle
pub(crate) fn crop_image_region(bytes: &[u8], x1: u32, y1: u32, x2: u32, y2: u32) -> VisionResult<Vec<u8>> {
    use image::{GenericImageView, ImageEncoder, RgbaImage};

    // Decode the source image
    let img = image::load_from_memory(bytes).map_err(|e| {
        VisionError::ImageProcessingError(format!("Failed to decode image: {}", e))
    })?;

    let (width, height) = img.dimensions();

    // Validate region bounds and clamp if needed
    let x1 = x1.min(width);
    let y1 = y1.min(height);
    let x2 = x2.min(width).max(x1);
    let y2 = y2.min(height).max(y1);

    let crop_width = x2 - x1;
    let crop_height = y2 - y1;

    if crop_width == 0 || crop_height == 0 {
        return Err(VisionError::RegionError(
            "Crop region has zero area after clamping to image bounds".to_string(),
        ));
    }

    // Extract the region (RGBA for consistent output)
    let rgba = img.to_rgba8();
    let mut cropped = RgbaImage::new(crop_width, crop_height);

    for y in 0..crop_height {
        for x in 0..crop_width {
            let src_pixel = rgba.get_pixel(x1 + x, y1 + y);
            cropped.put_pixel(x, y, *src_pixel);
        }
    }

    // Encode as PNG (lossless, good for screenshots and UI elements)
    let mut png_bytes = Vec::new();
    {
        let encoder = image::codecs::png::PngEncoder::new(&mut png_bytes);
        encoder
            .write_image(cropped.as_raw(), crop_width, crop_height, image::ExtendedColorType::Rgba8)
            .map_err(|e| {
                VisionError::ImageProcessingError(format!("Failed to encode cropped PNG: {}", e))
            })?;
    }

    tracing::info!(
        original = format!("{}x{}", width, height),
        crop = format!("[{}, {}, {}, {}]", x1, y1, x2, y2),
        result = format!("{}x{}", crop_width, crop_height),
        output_bytes = png_bytes.len(),
        "Image region cropped"
    );

    Ok(png_bytes)
}
