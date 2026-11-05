//! ELF binary analysis — header parsing, symbol extraction, string enumeration.

pub mod dependencies;
pub mod header;
pub mod read;
pub mod sections;
pub mod strings;
pub mod symbols;
pub mod types;

use analyze_common::{
    ANALYZE_COMMON_VERSION, AnalysisProvenance, AnalysisResult, AnalyzeError, DoubtBlock,
};
use std::fs;

const ELF_MAGIC: [u8; 4] = [0x7f, b'E', b'L', b'F'];

pub use types::*;

pub struct ElfParser;

impl ElfParser {
    pub fn analyze(path: &str) -> Result<AnalysisResult<ElfAnalysis>, AnalyzeError> {
        let data = fs::read(path).map_err(|_| AnalyzeError::FileNotFound(path.to_string()))?;
        if data.len() < 64 || &data[0..4] != ELF_MAGIC.as_slice() {
            return Err(AnalyzeError::InvalidFormat("not an ELF binary".to_string()));
        }

        let header = header::parse_elf_header(&data)?;
        let sections = sections::parse_sections(&data, &header)?;
        let symbols = symbols::parse_symbols(&data, &header, &sections)?;
        let dynamic_symbols = symbols::parse_dynamic_symbols(&data, &header, &sections)?;
        let dependencies = dependencies::parse_dependencies(&data, &header, &sections)?;
        let strings = strings::extract_strings(&data);

        let analysis = ElfAnalysis {
            path: path.to_string(),
            size_bytes: data.len() as u64,
            header,
            sections,
            symbols,
            dynamic_symbols,
            dependencies,
            strings,
        };

        let provenance = AnalysisProvenance::new("analyze-native", ANALYZE_COMMON_VERSION);
        let doubt = DoubtBlock::new("30d")
            .assume("file is a valid ELF binary")
            .blind_spot("dynamic linking not resolved");

        Ok(AnalysisResult::new(
            format!(
                "elf-{}",
                std::time::SystemTime::now().elapsed().unwrap().as_millis()
            ),
            provenance,
            analysis,
            doubt,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_elf_header() {
        let result = ElfParser::analyze("/bin/ls");
        assert!(
            result.is_ok(),
            "Failed to parse /bin/ls: {:?}",
            result.err()
        );
        let analysis = result.unwrap();
        assert_eq!(analysis.data.header.arch.to_string(), "x86_64");
        assert_eq!(analysis.data.header.endian.to_string(), "little");
    }

    #[test]
    fn test_extract_strings() {
        let data = b"hello world\x00some text\x00";
        let strings = strings::extract_strings(data);
        assert!(!strings.is_empty());
    }
}
