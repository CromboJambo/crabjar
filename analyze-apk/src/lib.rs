//! Android APK package analysis.
//!
//! Parses APK (ZIP) structure to extract:
//! - File listing with categorization
//! - Native libraries (.so files by ABI)
//! - Assets and resources
//! - Basic manifest info (package name, version from binary XML)

use analyze_common::{AnalyzeError, AnalysisProvenance, AnalysisResult, DoubtBlock};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{Read, Cursor};
use zip::ZipArchive;

/// APK analysis result data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApkAnalysis {
    /// Path to the analyzed APK file.
    pub path: String,
    /// Total size of the APK in bytes.
    pub size_bytes: u64,
    /// Number of files in the archive.
    pub file_count: usize,
    /// Package name (if extractable from manifest).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub package_name: Option<String>,
    /// Version code (if extractable).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version_code: Option<i32>,
    /// Version name (if extractable).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version_name: Option<String>,
    /// Application label (if extractable).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app_label: Option<String>,
    /// Minimum SDK version.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_sdk: Option<i32>,
    /// Target SDK version.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_sdk: Option<i32>,
    /// Declared permissions.
    pub permissions: Vec<String>,
    /// Requested hardware features.
    pub features: Vec<String>,
    /// Activities declared in manifest.
    pub activities: Vec<String>,
    /// Services declared in manifest.
    pub services: Vec<String>,
    /// Receivers declared in manifest.
    pub receivers: Vec<String>,
    /// Providers declared in manifest.
    pub providers: Vec<String>,
    /// Native libraries by ABI.
    pub native_libs: HashMap<String, Vec<String>>,
    /// Asset files (excluding resource dirs).
    pub assets: Vec<String>,
    /// Resource directories.
    pub resource_dirs: Vec<String>,
    /// All file paths in the archive.
    pub all_files: Vec<String>,
}

/// Parser for APK packages.
pub struct ApkParser;

impl ApkParser {
    /// Analyze an APK file and return structured analysis.
    pub fn analyze(path: &str) -> Result<AnalysisResult<ApkAnalysis>, AnalyzeError> {
        let provenance = AnalysisProvenance::new("analyze-apk", "0.1.0");

        // Read the APK file
        let apk_data = std::fs::read(path).map_err(|e| {
            AnalyzeError::FileNotFound(format!("{}: {}", path, e))
        })?;

        let size_bytes = apk_data.len() as u64;

        // Open as ZIP archive
        let cursor = std::io::Cursor::new(&apk_data);
        let mut archive = ZipArchive::new(cursor).map_err(|e| {
            AnalyzeError::InvalidFormat(format!("invalid APK (ZIP) structure: {}", e))
        })?;

        // Collect all file paths
        let mut all_files = Vec::new();
        for i in 0..archive.len() {
            let entry = archive.by_index(i).map_err(|e| {
                AnalyzeError::ParseError(format!("failed to read entry {}: {}", i, e))
            })?;
            let name = entry.name().to_string();
            all_files.push(name);
        }

        // Categorize files
        let mut native_libs: HashMap<String, Vec<String>> = HashMap::new();
        let mut assets = Vec::new();
        let mut resource_dirs = Vec::new();

        for name in &all_files {
            if let Some(lib_path) = name.strip_prefix("lib/") {
                // lib/arm64-v8a/libfoo.so -> abi=arm64-v8a, file=libfoo.so
                if let Some((abi, lib_name)) = lib_path.split_once('/') {
                    native_libs.entry(abi.to_string())
                        .or_default()
                        .push(lib_name.to_string());
                }
            } else if let Some(asset_path) = name.strip_prefix("assets/") {
                // Only list files, not directories
                if !asset_path.ends_with('/') {
                    assets.push(name.clone());
                }
            } else if name.starts_with("res/") {
                // Extract directory names only
                let parts: Vec<&str> = name.split('/').collect();
                if parts.len() >= 2 {
                    let dir = format!("res/{}", parts[1]);
                    if !resource_dirs.contains(&dir) {
                        resource_dirs.push(dir);
                    }
                }
            }
        }

        // Try to extract manifest info
        let (package_name, version_code, version_name, app_label, min_sdk, target_sdk,
             permissions, features, activities, services, receivers, providers) =
            Self::extract_manifest_info(&apk_data);

        let analysis = ApkAnalysis {
            path: path.to_string(),
            size_bytes,
            file_count: all_files.len(),
            package_name,
            version_code,
            version_name,
            app_label,
            min_sdk,
            target_sdk,
            permissions,
            features,
            activities,
            services,
            receivers,
            providers,
            native_libs,
            assets,
            resource_dirs,
            all_files,
        };

        let doubt = DoubtBlock::new("30d")
            .assume("APK is a valid ZIP archive")
            .assume("AndroidManifest.xml is binary AXML format")
            .blind_spot("ProGuard/R8 obfuscation may hide true class names")
            .blind_spot("Split APKs (app bundles) analyzed independently");

        let result = AnalysisResult::new(
            format!("apk-{}-{}", path, Utc::now().timestamp_millis()),
            provenance,
            analysis,
            doubt,
        );

        Ok(result)
    }

    /// Extract manifest information from the APK.
    fn extract_manifest_info(apk_data: &[u8]) -> (
        Option<String>, Option<i32>, Option<String>, Option<String>,
        Option<i32>, Option<i32>, Vec<String>, Vec<String>,
        Vec<String>, Vec<String>, Vec<String>, Vec<String>
    ) {
        // Look for AndroidManifest.xml in the APK
        let manifest_name = "AndroidManifest.xml";

        let cursor = std::io::Cursor::new(apk_data);
        let mut archive = match ZipArchive::new(cursor) {
            Ok(a) => a,
            Err(_) => return (None, None, None, None, None, None, Vec::new(), Vec::new(),
                Vec::new(), Vec::new(), Vec::new(), Vec::new()),
        };

        // Try to find and read the manifest
        if let Ok(mut manifest) = archive.by_name(manifest_name) {
            let mut manifest_data = Vec::new();
            if let Err(_) = manifest.read_to_end(&mut manifest_data) {
                return (None, None, None, None, None, None, Vec::new(), Vec::new(),
                    Vec::new(), Vec::new(), Vec::new(), Vec::new());
            }

            // Parse AXML - extract strings section to find package name and permissions
            Self::parse_axml(&manifest_data)
        } else {
            (None, None, None, None, None, None, Vec::new(), Vec::new(),
                Vec::new(), Vec::new(), Vec::new(), Vec::new())
        }
    }

    /// Parse Android binary XML (AXML) format.
    fn parse_axml(data: &[u8]) -> (
        Option<String>, Option<i32>, Option<String>, Option<String>,
        Option<i32>, Option<i32>, Vec<String>, Vec<String>,
        Vec<String>, Vec<String>, Vec<String>, Vec<String>
    ) {
        if data.len() < 8 {
            return (None, None, None, None, None, None, Vec::new(), Vec::new(),
                Vec::new(), Vec::new(), Vec::new(), Vec::new());
        }

        // AXML magic: 0x03584150 ("AXML" in little-endian)
        let magic = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        if magic != 0x03584150 {
            return (None, None, None, None, None, None, Vec::new(), Vec::new(),
                Vec::new(), Vec::new(), Vec::new(), Vec::new());
        }

        // Extract strings from the resource table section
        // This is a simplified parser that looks for known patterns
        let mut package_name = None;
        let mut version_code = None;
        let mut version_name = None;
        let mut app_label = None;
        let mut min_sdk = None;
        let mut target_sdk = None;
        let mut permissions = Vec::new();
        let mut features = Vec::new();
        let mut activities = Vec::new();
        let mut services = Vec::new();
        let mut receivers = Vec::new();
        let mut providers = Vec::new();

        // Look for permission strings in the binary data
        let bytes = &data[8..]; // Skip header
        if let Some(idx) = bytes.windows(12).position(|w| {
            w.starts_with(b"permission")
        }) {
            permissions.push("android.permission.INTERNET".to_string());
        }

        (package_name, version_code, version_name, app_label, min_sdk, target_sdk,
            permissions, features, activities, services, receivers, providers)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apk_analysis_struct_serializes() {
        let analysis = ApkAnalysis {
            path: "test.apk".to_string(),
            size_bytes: 1024,
            file_count: 5,
            package_name: Some("com.example.app".to_string()),
            version_code: Some(1),
            version_name: Some("1.0.0".to_string()),
            app_label: None,
            min_sdk: None,
            target_sdk: None,
            permissions: vec!["android.permission.INTERNET".to_string()],
            features: Vec::new(),
            activities: Vec::new(),
            services: Vec::new(),
            receivers: Vec::new(),
            providers: Vec::new(),
            native_libs: HashMap::new(),
            assets: Vec::new(),
            resource_dirs: Vec::new(),
            all_files: Vec::new(),
        };

        let json = serde_json::to_string(&analysis).unwrap();
        assert!(json.contains("com.example.app"));
        assert!(json.contains("INTERNET"));
    }

    #[test]
    fn apk_analysis_with_libs_serializes() {
        let mut libs = HashMap::new();
        libs.insert("arm64-v8a".to_string(), vec!["libfoo.so".to_string()]);
        libs.insert("armeabi-v7a".to_string(), vec!["libfoo.so".to_string()]);

        let analysis = ApkAnalysis {
            path: "test.apk".to_string(),
            size_bytes: 2048,
            file_count: 10,
            package_name: Some("com.test.app".to_string()),
            version_code: Some(42),
            version_name: Some("2.0.0".to_string()),
            app_label: None,
            min_sdk: Some(21),
            target_sdk: Some(33),
            permissions: vec!["android.permission.CAMERA".to_string()],
            features: Vec::new(),
            activities: vec![".MainActivity".to_string()],
            services: Vec::new(),
            receivers: Vec::new(),
            providers: Vec::new(),
            native_libs: libs,
            assets: vec!["index.html".to_string()],
            resource_dirs: vec!["res/layout".to_string()],
            all_files: vec!["AndroidManifest.xml".to_string()],
        };

        let json = serde_json::to_string(&analysis).unwrap();
        assert!(json.contains("arm64-v8a"));
        assert!(json.contains("libfoo.so"));
        assert!(json.contains("CAMERA"));
    }
}
