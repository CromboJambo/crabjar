//! .NET assembly analysis — PE header parsing, CLR version detection, type enumeration.

use analyze_common::{AnalysisProvenance, AnalysisResult, AnalyzeError, DoubtBlock};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DotnetAnalysis {
    pub path: String,
    pub size_bytes: u64,
    pub machine_type: Option<String>,
    pub entry_point: Option<u64>,
    pub clr_version: Option<String>,
    pub types: Vec<DotnetType>,
    pub assembly_references: Vec<AssemblyReference>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DotnetType {
    pub name: String,
    pub is_class: bool,
    pub method_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssemblyReference {
    pub name: String,
    pub version: Option<String>,
}

pub fn analyze_dotnet(path: &str) -> Result<AnalysisResult<DotnetAnalysis>, AnalyzeError> {
    let path_buf = Path::new(path);
    if !path_buf.exists() {
        return Err(AnalyzeError::FileNotFound(path.to_string()));
    }

    let bytes = fs::read(path_buf).map_err(|e| AnalyzeError::Io(e))?;
    let size_bytes = bytes.len() as u64;

    // Parse PE header
    if bytes.len() < 64 {
        return Err(AnalyzeError::InvalidFormat("file too small for PE format".to_string()));
    }

    // Check DOS header signature (MZ)
    if &bytes[0..2] != [0x4D, 0x5A] {
        return Err(AnalyzeError::InvalidFormat("not a PE file (missing MZ signature)".to_string()));
    }

    // PE header offset is at DOS header +60 (e_lfanew)
    let pe_offset = u32::from_le_bytes([bytes[60], bytes[61], bytes[62], bytes[63]]) as usize;

    if bytes.len() < pe_offset + 4 {
        return Err(AnalyzeError::InvalidFormat("truncated PE file".to_string()));
    }

    // Check PE signature (PE\0\0)
    if &bytes[pe_offset..pe_offset+4] != [0x50, 0x45, 0x00, 0x00] {
        return Err(AnalyzeError::InvalidFormat("invalid PE signature".to_string()));
    }

    // Parse COFF header (starts at pe_offset + 4)
    let coff_offset = pe_offset + 4;
    let machine_type_raw = u16::from_le_bytes([bytes[coff_offset], bytes[coff_offset+1]]);
    let machine_type = match machine_type_raw {
        0x8664 => "x64",
        0x14c => "x86",
        0xAA64 => "ARM64",
        _ => "unknown",
    }.to_string();

    // Entry point RVA is at coff_offset + 16
    let entry_point_rva = u32::from_le_bytes([
        bytes[coff_offset + 16],
        bytes[coff_offset + 17],
        bytes[coff_offset + 18],
        bytes[coff_offset + 19]
    ]);

    // Try to detect CLR version by looking for .NET metadata markers
    let clr_version = detect_clr_version(&bytes);

    // Enumerate types (simplified - look for type definition patterns)
    let types = enumerate_types(&bytes);

    // Extract assembly references
    let assembly_references = extract_assembly_refs(&bytes);

    let data = DotnetAnalysis {
        path: path.to_string(),
        size_bytes,
        machine_type: Some(machine_type),
        entry_point: Some(entry_point_rva as u64),
        clr_version,
        types,
        assembly_references,
    };

    let provenance = AnalysisProvenance::new("analyze-dotnet", "0.1.0");
    let doubt = DoubtBlock::new("7d")
        .assume("file is a valid .NET assembly (PE format)")
        .blind_spot("CLR version detection is heuristic based on metadata markers")
        .blind_spot("type enumeration may be incomplete without full IL parsing");

    Ok(AnalysisResult::new(
        format!("dotnet-{}", Utc::now().timestamp_millis()),
        provenance,
        data,
        doubt,
    ))
}

fn detect_clr_version(bytes: &[u8]) -> Option<String> {
    // Look for .NET runtime version strings in the binary
    if bytes.windows(26).any(|w| w == b".NETFramework,Version=v4.8") {
        return Some("4.8".to_string());
    }
    if bytes.windows(24).any(|w| w == b".NETCoreApp,Version=v3.1") {
        return Some("3.1".to_string());
    }
    if bytes.windows(24).any(|w| w == b".NETCoreApp,Version=v5.0") {
        return Some("5.0".to_string());
    }
    if bytes.windows(24).any(|w| w == b".NETCoreApp,Version=v6.0") {
        return Some("6.0".to_string());
    }
    if bytes.windows(24).any(|w| w == b".NETCoreApp,Version=v7.0") {
        return Some("7.0".to_string());
    }
    if bytes.windows(24).any(|w| w == b".NETCoreApp,Version=v8.0") {
        return Some("8.0".to_string());
    }
    if bytes.windows(21).any(|w| w == b"Microsoft.NETCore.App") {
        return Some("core".to_string());
    }

    None
}

fn enumerate_types(bytes: &[u8]) -> Vec<DotnetType> {
    // Simplified type enumeration - look for class name patterns in strings section
    // This is a heuristic approach; full parsing requires IL metadata tables
    let mut types = Vec::new();

    // Scan for potential type names (sequences of uppercase letters followed by lowercase)
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] >= b'A' && bytes[i] <= b'Z' {
            let start = i;
            let mut j = i + 1;
            while j < bytes.len() && (bytes[j].is_ascii_alphanumeric() || bytes[j] == b'_') {
                j += 1;
            }
            if j - start > 2 && j - start < 64 {
                let name_bytes = &bytes[start..j];
                let name = String::from_utf8_lossy(name_bytes).to_string();
                // Filter to plausible type names (start with uppercase, contain at least one lowercase)
                if name.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
                    if name.chars().any(|c| c.is_lowercase()) {
                        types.push(DotnetType {
                            name,
                            is_class: true,
                            method_count: 0,
                        });
                    }
                }
            }
            i = j;
        } else {
            i += 1;
        }
    }

    // Deduplicate
    types.sort_by(|a, b| a.name.cmp(&b.name));
    types.dedup_by(|a, b| a.name == b.name);

    // Limit to reasonable number
    if types.len() > 50 {
        types.truncate(50);
    }

    types
}

fn extract_assembly_refs(bytes: &[u8]) -> Vec<AssemblyReference> {
    // Look for assembly reference patterns in the metadata
    let patterns = [
        ("mscorlib", None),
        ("System.Core", None),
        ("System.Runtime", None),
        ("Newtonsoft.Json", None),
        ("Microsoft.Extensions.DependencyInjection", None),
    ];

    let mut refs = Vec::new();
    for (name, version) in patterns {
        if bytes.windows(name.len()).any(|w| w == name.as_bytes()) {
            refs.push(AssemblyReference {
                name: name.to_string(),
                version,
            });
        }
    }

    refs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clr_version_detection() {
        let sample = b"some binary data .NETCoreApp,Version=v8.0 more data";
        assert_eq!(detect_clr_version(sample), Some("8.0".to_string()));
    }

    #[test]
    fn test_type_enumeration_basic() {
        let sample = b"MyClass SomeOtherType NotAType 123Numbers";
        let types = enumerate_types(sample);
        // Should find MyClass and SomeOtherType (start with uppercase, contain lowercase)
        assert!(types.iter().any(|t| t.name == "MyClass"));
        assert!(types.iter().any(|t| t.name == "SomeOtherType"));
    }
}
