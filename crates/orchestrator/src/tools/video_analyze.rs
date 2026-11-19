//! video_analyze tool — video frame analysis (P2 parity).
//!
//! Extracts frames from a video using ffmpeg, then analyzes each frame
//! with crabjar-vision. Returns aggregated results for the whole video.

use crabjar_vision::{VisionClient, VisionConfig};
use reqwest::Client;
use serde_json::{Value, json};
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, OnceLock};
use tempfile::TempDir;
use tracing::{debug, warn};

static VISION_CLIENT: OnceLock<Arc<VisionClient>> = OnceLock::new();

fn get_vision_client() -> Result<&'static VisionClient, String> {
    if let Some(client) = VISION_CLIENT.get() {
        return Ok(client.as_ref());
    }
    let config = VisionConfig::from_env().map_err(|e| e.to_string())?;
    let client = Arc::new(VisionClient::new(config));
    VISION_CLIENT.get_or_init(|| client.clone());
    Ok(VISION_CLIENT.get().unwrap())
}

/// Handle video_analyze tool invocation.
///
/// Arguments:
/// - args[0]: JSON string with fields:
///   - `video` (required): URL or local path to the video file
///   - `question` (required): What to analyze in the video
///   - `max_frames` (optional): Maximum frames to extract (default 8)
///   - `interval` (optional): Seconds between frames (default: auto based on duration)
pub async fn handle(args: &[String]) -> Result<String, String> {
    if args.is_empty() || args[0].is_empty() {
        return Err(
            "video_analyze requires a JSON argument with 'video' and 'question' fields".to_string(),
        );
    }

    let request: Value = serde_json::from_str(&args[0])
        .map_err(|e| format!("Failed to parse video_analyze arguments: {}", e))?;

    let video_source = request["video"]
        .as_str()
        .ok_or("video_analyze requires 'video' field (URL or local path)")?;

    let question = request["question"]
        .as_str()
        .ok_or("video_analyze requires 'question' field")?;

    let max_frames = request["max_frames"].as_u64().unwrap_or(8) as usize;
    let interval = request["interval"].as_f64(); // seconds between frames (optional)

    // Step 1: Get the video file (download if URL, use path if local)
    debug!(video_source, "Starting video analysis");
    let temp_dir = TempDir::new().map_err(|e| format!("Failed to create temp directory: {}", e))?;

    let video_path = if video_source.starts_with("http://") || video_source.starts_with("https://")
    {
        debug!(url = video_source, "Downloading video");
        download_video(video_source, &temp_dir).await?
    } else {
        let path = PathBuf::from(video_source);
        if !path.exists() {
            return Err(format!("Video file not found: {}", video_source));
        }
        path
    };

    // Step 2: Get video duration using ffprobe
    let duration = get_video_duration(&video_path)?;
    debug!(duration_secs = duration, "Video duration obtained");

    // Step 3: Calculate frame intervals
    let effective_interval = interval.unwrap_or_else(|| {
        if max_frames <= 1 {
            0.0
        } else {
            duration / (max_frames as f64 - 1.0)
        }
    });

    // Step 4: Extract frames using ffmpeg
    debug!(
        max_frames,
        interval = effective_interval,
        "Extracting frames"
    );
    let frame_paths = extract_frames(&video_path, &temp_dir, max_frames, effective_interval)?;

    if frame_paths.is_empty() {
        return Err("No frames could be extracted from the video".to_string());
    }

    debug!(count = frame_paths.len(), "Frames extracted");

    // Step 5: Analyze each frame with vision model
    let vision_client = get_vision_client()?;
    let mut frame_analyses = Vec::new();
    let mut errors = Vec::new();

    for (i, frame_path) in frame_paths.iter().enumerate() {
        debug!(frame = i + 1, total = frame_paths.len(), "Analyzing frame");
        match vision_client
            .analyze(frame_path.to_str().unwrap(), question)
            .await
        {
            Ok(analysis) => {
                frame_analyses.push(json!({
                    "frame": i + 1,
                    "timestamp": format!("{:.1}s", effective_interval * i as f64),
                    "analysis": analysis.analysis,
                    "model": analysis.model
                }));
            }
            Err(e) => {
                warn!(frame = i + 1, error = %e, "Frame analysis failed");
                errors.push(format!("Frame {}: {}", i + 1, e));
            }
        }
    }

    // Step 6: Aggregate results
    let result = json!({
        "status": if frame_analyses.is_empty() { "failed" } else { "ok" },
        "video": video_source,
        "duration_secs": duration,
        "frames_analyzed": frame_analyses.len(),
        "total_frames_extracted": frame_paths.len(),
        "question": question,
        "analyses": frame_analyses,
        "errors": if errors.is_empty() { Value::Null } else { json!(errors) },
        "summary": generate_summary(&frame_analyses, question)
    });

    Ok(serde_json::to_string_pretty(&result).map_err(|e| e.to_string())?)
}

async fn download_video(url: &str, temp_dir: &TempDir) -> Result<PathBuf, String> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Failed to download video: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "HTTP {} downloading video",
            response.status().as_u16()
        ));
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Failed to read video bytes: {}", e))?;

    // Determine extension from URL or default to mp4
    let ext = url.rsplit('.').next().unwrap_or("mp4");
    let safe_ext = if ext.len() <= 5 && ext.chars().all(|c| c.is_ascii_alphanumeric()) {
        ext.to_string()
    } else {
        "mp4".to_string()
    };

    let path = temp_dir.path().join(format!("video.{}", safe_ext));
    std::fs::write(&path, &bytes).map_err(|e| format!("Failed to write video to disk: {}", e))?;

    Ok(path)
}

fn get_video_duration(video_path: &PathBuf) -> Result<f64, String> {
    let output = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
        ])
        .arg(video_path)
        .output()
        .map_err(|e| format!("Failed to run ffprobe: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "ffprobe failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let duration_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
    duration_str
        .parse::<f64>()
        .map_err(|e| format!("Failed to parse video duration '{}': {}", duration_str, e))
}

fn extract_frames(
    video_path: &PathBuf,
    temp_dir: &TempDir,
    max_frames: usize,
    interval: f64,
) -> Result<Vec<PathBuf>, String> {
    if interval <= 0.0 {
        // Single frame at start
        let output_path = temp_dir.path().join("frame_%03d.png");
        let output = Command::new("ffmpeg")
            .arg("-y")
            .arg("-i")
            .arg(video_path.to_str().unwrap())
            .arg("-ss")
            .arg("0")
            .arg("-vframes")
            .arg("1")
            .arg("-f")
            .arg("image2")
            .arg(output_path)
            .output()
            .map_err(|e| format!("Failed to run ffmpeg: {}", e))?;

        if !output.status.success() {
            return Err(format!(
                "ffmpeg failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }
    } else {
        // Extract frames at regular intervals
        let output_path = temp_dir.path().join("frame_%03d.png");
        let vf_arg = format!("fps=1/{}", interval);
        let frames_arg = max_frames.to_string();
        let output = Command::new("ffmpeg")
            .arg("-y")
            .arg("-i")
            .arg(video_path.to_str().unwrap())
            .arg("-vf")
            .arg(&vf_arg)
            .arg("-frames:v")
            .arg(&frames_arg)
            .arg("-f")
            .arg("image2")
            .arg(output_path)
            .output()
            .map_err(|e| format!("Failed to run ffmpeg: {}", e))?;

        if !output.status.success() {
            return Err(format!(
                "ffmpeg failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }
    }

    // Collect extracted frame paths
    let mut frames = Vec::new();
    for entry in
        std::fs::read_dir(temp_dir.path()).map_err(|e| format!("Failed to read temp dir: {}", e))?
    {
        let entry = entry.map_err(|e| format!("Failed to read dir entry: {}", e))?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("png") {
            frames.push(path);
        }
    }

    frames.sort();
    Ok(frames)
}

fn generate_summary(analyses: &[Value], question: &str) -> String {
    if analyses.is_empty() {
        return "No frames could be analyzed.".to_string();
    }

    let count = analyses.len();
    let frame_texts: Vec<&str> = analyses
        .iter()
        .filter_map(|a| a["analysis"].as_str())
        .collect();

    if count == 1 {
        return format!("Analyzed 1 frame. {}", frame_texts[0]);
    }

    // Simple aggregation: note that multiple frames were analyzed
    let unique_observations = frame_texts
        .iter()
        .take(3)
        .enumerate()
        .map(|(i, text)| format!("Frame {}: {}", i + 1, &text[..text.len().min(100)]))
        .collect::<Vec<_>>()
        .join(" | ");

    format!(
        "Analyzed {} frames for: '{}'. Key observations: {}",
        count, question, unique_observations
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_video_analyze_missing_video() {
        let args = vec![json!({"question": "what is this?"}).to_string()];
        let result = handle(&args).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("requires 'video'"));
    }

    #[tokio::test]
    async fn test_video_analyze_missing_question() {
        let args = vec![json!({"video": "/tmp/test.mp4"}).to_string()];
        let result = handle(&args).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("requires 'question'"));
    }

    #[tokio::test]
    async fn test_video_analyze_nonexistent_file() {
        let args = vec![
            json!({
                "video": "/tmp/definitely_does_not_exist_12345.mp4",
                "question": "what is this?"
            })
            .to_string(),
        ];
        let result = handle(&args).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[tokio::test]
    async fn test_generate_summary_empty() {
        let summary = generate_summary(&[], "test question");
        assert_eq!(summary, "No frames could be analyzed.");
    }

    #[tokio::test]
    async fn test_generate_summary_single_frame() {
        let analyses = vec![json!({
            "frame": 1,
            "analysis": "A cat sitting on a table",
            "confidence": 0.95
        })];
        let summary = generate_summary(&analyses, "what animal is this?");
        assert!(summary.contains("Analyzed 1 frame"));
        assert!(summary.contains("A cat sitting on a table"));
    }

    #[tokio::test]
    async fn test_generate_summary_multiple_frames() {
        let analyses = vec![
            json!({"frame": 1, "analysis": "Cat enters frame", "confidence": 0.9}),
            json!({"frame": 2, "analysis": "Cat jumps on table", "confidence": 0.85}),
            json!({"frame": 3, "analysis": "Cat knocks over glass", "confidence": 0.92}),
        ];
        let summary = generate_summary(&analyses, "what is the cat doing?");
        assert!(summary.contains("Analyzed 3 frames"));
        assert!(summary.contains("Frame 1"));
    }

    #[tokio::test]
    async fn test_video_analyze_empty_args() {
        let args: Vec<String> = vec![];
        let result = handle(&args).await;
        assert!(result.is_err());
    }
}
