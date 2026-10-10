//! Firmware image analysis — file magic detection, region identification, binwalk integration.

use analyze_common::{AnalysisProvenance, AnalysisResult, AnalyzeError, DoubtBlock};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirmwareAnalysis {
    pub path: String,
    pub size_bytes: u64,
    pub format_detected: Option<String>,
    pub compression_detected: Option<String>,
    pub regions: Vec<FirmwareRegion>,
    pub binwalk_output: Option<BinwalkOutput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirmwareRegion {
    pub name: String,
    pub offset: u64,
    pub size: u64,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinwalkOutput {
    pub found: bool,
    pub output: String,
}

pub fn analyze_firmware(path: &str) -> Result<AnalysisResult<FirmwareAnalysis>, AnalyzeError> {
    let path_buf = Path::new(path);
    if !path_buf.exists() {
        return Err(AnalyzeError::FileNotFound(path.to_string()));
    }

    let bytes = fs::read(path_buf).map_err(|e| AnalyzeError::Io(e))?;
    let size_bytes = bytes.len() as u64;

    // Detect format by file magic
    let format_detected = detect_format(&bytes);

    // Detect compression
    let compression_detected = detect_compression(&bytes);

    // Identify regions (bootloader, kernel, rootfs)
    let regions = identify_regions(&bytes, size_bytes);

    // Try binwalk for deep analysis
    let binwalk_output = run_binwalk(path);

    let data = FirmwareAnalysis {
        path: path.to_string(),
        size_bytes,
        format_detected,
        compression_detected,
        regions,
        binwalk_output,
    };

    let provenance = AnalysisProvenance::new("analyze-firmware", "0.1.0");
    let doubt = DoubtBlock::new("7d")
        .assume("file is a valid firmware image")
        .blind_spot("region identification is heuristic based on file magic and offsets")
        .blind_spot("binwalk may not be installed or available");

    Ok(AnalysisResult::new(
        format!("firmware-{}", Utc::now().timestamp_millis()),
        provenance,
        data,
        doubt,
    ))
}

fn detect_format(bytes: &[u8]) -> Option<String> {
    // FIT image (Flattened Image Tree) — ARM firmware
    if bytes.len() >= 4 && &bytes[0..4] == [0x27, 0x05, 0x19, 0x56] {
        return Some("FIT image".to_string());
    }

    // U-Boot header
    if bytes.len() >= 64 && &bytes[0..4] == [0x27, 0x05, 0x19, 0x56] {
        return Some("U-Boot image".to_string());
    }

    // Raw kernel (zImage) — x86 boot sector magic at end of first sector
    if bytes.len() >= 512 && bytes[510] == 0x55 && bytes[511] == 0xAA {
        return Some("raw disk image or MBR".to_string());
    }

    // SquashFS (common rootfs)
    if bytes.len() >= 4 && (&bytes[0..4] == b"hsqs" || &bytes[0..4] == b"sqsh") {
        return Some("SquashFS".to_string());
    }

    // JFFS2 (embedded filesystem)
    if bytes.len() >= 4 && (&bytes[0..4] == [0x85, 0xEC, 0x13, 0x37] || &bytes[0..4] == [0x1E, 0x90, 0x82, 0xA6]) {
        return Some("JFFS2 filesystem".to_string());
    }

    // CramFS
    if bytes.len() >= 4 && (&bytes[0..4] == [0x27, 0x05, 0x19, 0x56]) {
        return Some("CramFS filesystem".to_string());
    }

    None
}

fn detect_compression(bytes: &[u8]) -> Option<String> {
    // gzip
    if bytes.len() >= 2 && bytes[0] == 0x1f && bytes[1] == 0x8b {
        return Some("gzip".to_string());
    }

    // xz/lzma
    if bytes.len() >= 6 && &bytes[0..6] == [0xFD, 0x37, 0x7A, 0x58, 0x5A, 0x00] {
        return Some("xz/lzma".to_string());
    }

    // lzop
    if bytes.len() >= 2 && bytes[0] == 0x89 && bytes[1] == 0x4C {
        return Some("lzop".to_string());
    }

    // lzo (old format)
    if bytes.len() >= 3 && bytes[0] == 0xD0 && (bytes[1] == 0x0B || bytes[1] == 0x1B) {
        return Some("lzo".to_string());
    }

    None
}

fn identify_regions(bytes: &[u8], size_bytes: u64) -> Vec<FirmwareRegion> {
    let mut regions = Vec::new();

    // Look for kernel images (zImage magic or ARM boot header)
    let mut offset: usize = 0;
    while offset + 512 < bytes.len() {
        // x86 zImage has boot sector magic at end of first sector
        if bytes[offset + 510] == 0x55 && bytes[offset + 511] == 0xAA {
            regions.push(FirmwareRegion {
                name: "kernel".to_string(),
                offset: offset as u64,
                size: (size_bytes as usize - offset) as u64,
                description: Some("Linux kernel image (zImage)".to_string()),
            });
        }

        // Look for SquashFS within the image
        if bytes.len() > offset + 4 && (&bytes[offset..offset+4] == b"hsqs" || &bytes[offset..offset+4] == b"sqsh") {
            regions.push(FirmwareRegion {
                name: "rootfs".to_string(),
                offset: offset as u64,
                size: (size_bytes as usize - offset) as u64,
                description: Some("SquashFS root filesystem".to_string()),
            });
        }

        // Look for JFFS2 magic
        if bytes.len() > offset + 4 && (&bytes[offset..offset+4] == [0x85, 0xEC, 0x13, 0x37] || &bytes[offset..offset+4] == [0x1E, 0x90, 0x82, 0xA6]) {
            regions.push(FirmwareRegion {
                name: "rootfs".to_string(),
                offset: offset as u64,
                size: (size_bytes as usize - offset) as u64,
                description: Some("JFFS2 root filesystem".to_string()),
            });
        }

        offset += 1024; // Scan in 1KB steps
    }

    if regions.is_empty() {
        regions.push(FirmwareRegion {
            name: "unknown".to_string(),
            offset: 0,
            size: size_bytes,
            description: Some("Could not identify firmware regions".to_string()),
        });
    }

    regions
}

fn run_binwalk(path: &str) -> Option<BinwalkOutput> {
    let result = Command::new("which")
        .arg("binwalk")
        .output();

    if result.is_err() {
        return None;
    }

    let which_result = result.unwrap();
    if !which_result.status.success() || which_result.stdout.is_empty() {
        return None;
    }

    // Run binwalk with timeout (30s)
    let output = Command::new("timeout")
        .arg("30")
        .arg("binwalk")
        .arg("-Me")
        .arg(path)
        .output();

    match output {
        Ok(out) => Some(BinwalkOutput {
            found: true,
            output: String::from_utf8_lossy(&out.stdout).to_string(),
        }),
        Err(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gzip_detection() {
        let gzip_magic = [0x1f, 0x8b, 0x08, 0x00];
        assert_eq!(detect_compression(&gzip_magic), Some("gzip".to_string()));
    }

    #[test]
    fn test_xz_detection() {
        let xz_magic = [0xFD, 0x37, 0x7A, 0x58, 0x5A, 0x00];
        assert_eq!(detect_compression(&xz_magic), Some("xz/lzma".to_string()));
    }

    #[test]
    fn test_squashfs_detection() {
        let squashfs = [b'h', b's', b'q', b's'];
        assert_eq!(detect_format(&squashfs), Some("SquashFS".to_string()));
    }
}
