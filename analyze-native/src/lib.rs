//! ELF binary analysis — header parsing, symbol extraction, string enumeration.

use analyze_common::{
    AnalysisProvenance, AnalysisResult, AnalyzeError, DoubtBlock, ANALYZE_COMMON_VERSION,
};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fs;

const ELF_MAGIC: [u8; 4] = [0x7f, b'E', b'L', b'F'];
const EI_CLASS: usize = 4;
const EI_DATA: usize = 5;
const EI_VERSION: usize = 6;
const EI_OSABI: usize = 7;
const ELFDATA2LSB: u8 = 1;
const ELFDATA2MSB: u8 = 2;
const ELFCLASS32: u8 = 1;
const ELFCLASS64: u8 = 2;
const EV_CURRENT: u8 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ElfArch { X86, X86_64, Arm, Aarch64, Other(u16) }
impl std::fmt::Display for ElfArch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ElfArch::X86 => write!(f, "x86"),
            ElfArch::X86_64 => write!(f, "x86_64"),
            ElfArch::Arm => write!(f, "arm"),
            ElfArch::Aarch64 => write!(f, "aarch64"),
            ElfArch::Other(c) => write!(f, "other({})", c),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ElfEndian { Little, Big }
impl std::fmt::Display for ElfEndian {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self { ElfEndian::Little => write!(f, "little"), ElfEndian::Big => write!(f, "big") }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ElfClass { Elf32, Elf64 }
impl std::fmt::Display for ElfClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self { ElfClass::Elf32 => write!(f, "ELF32"), ElfClass::Elf64 => write!(f, "ELF64") }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ElfOsAbi { SystemV, Hurd, Solaris, Linux, Other(u8) }
impl std::fmt::Display for ElfOsAbi {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ElfOsAbi::SystemV => write!(f, "systemv"), ElfOsAbi::Hurd => write!(f, "hurd"),
            ElfOsAbi::Solaris => write!(f, "solaris"), ElfOsAbi::Linux => write!(f, "linux"),
            ElfOsAbi::Other(c) => write!(f, "other({})", c),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElfHeader {
    pub class: ElfClass,
    pub endian: ElfEndian,
    pub version: u8,
    pub abi: ElfOsAbi,
    pub arch: ElfArch,
    pub entry_point: u64,
    pub is_dynamic: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElfSection {
    pub index: usize,
    pub name: String,
    pub r#type: u32,
    pub addr: u64,
    pub offset: u64,
    pub size: u64,
    pub link: u32,
    pub entsize: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElfSymbol {
    pub name: String,
    pub value: u64,
    pub size: u64,
    pub binding: u8,
    pub r#type: u8,
    pub section_index: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElfDependency { pub name: String }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElfAnalysis {
    pub path: String,
    pub size_bytes: u64,
    pub header: ElfHeader,
    pub sections: Vec<ElfSection>,
    pub symbols: Vec<ElfSymbol>,
    pub dynamic_symbols: Vec<ElfSymbol>,
    pub dependencies: Vec<ElfDependency>,
    pub strings: Vec<String>,
}

pub struct ElfParser;

impl ElfParser {
    pub fn analyze(path: &str) -> Result<AnalysisResult<ElfAnalysis>, AnalyzeError> {
        let data = fs::read(path).map_err(|_| AnalyzeError::FileNotFound(path.to_string()))?;
        if data.len() < 64 || &data[0..4] != ELF_MAGIC.as_slice() {
            return Err(AnalyzeError::InvalidFormat("not an ELF binary".to_string()));
        }

        let header = parse_elf_header(&data)?;
        let sections = parse_sections(&data, &header)?;
        let symbols = parse_symbols(&data, &header, &sections)?;
        let dynamic_symbols = parse_dynamic_symbols(&data, &header, &sections)?;
        let dependencies = parse_dependencies(&data, &header, &sections)?;
        let strings = extract_strings(&data);

        let analysis = ElfAnalysis {
            path: path.to_string(), size_bytes: data.len() as u64, header, sections,
            symbols, dynamic_symbols, dependencies, strings,
        };

        let provenance = AnalysisProvenance::new("analyze-native", ANALYZE_COMMON_VERSION);
        let doubt = DoubtBlock::new("30d")
            .assume("file is a valid ELF binary")
            .blind_spot("dynamic linking not resolved");

        Ok(AnalysisResult::new(
            format!("elf-{}", std::time::SystemTime::now().elapsed().unwrap().as_millis()),
            provenance, analysis, doubt,
        ))
    }
}

fn parse_elf_header(data: &[u8]) -> Result<ElfHeader, AnalyzeError> {
    let class = match data[EI_CLASS] {
        ELFCLASS32 => ElfClass::Elf32, ELFCLASS64 => ElfClass::Elf64,
        other => return Err(AnalyzeError::InvalidFormat(format!("invalid ELF class: {}", other))),
    };
    let endian = match data[EI_DATA] {
        ELFDATA2LSB => ElfEndian::Little, ELFDATA2MSB => ElfEndian::Big,
        other => return Err(AnalyzeError::InvalidFormat(format!("invalid ELF endianness: {}", other))),
    };
    let version = data[EI_VERSION];
    if version != EV_CURRENT {
        return Err(AnalyzeError::InvalidFormat(format!("invalid ELF version: {}", version)));
    }
    let abi_code = data[EI_OSABI];
    let abi = match abi_code {
        0 => ElfOsAbi::SystemV, 1 => ElfOsAbi::Hurd, 2 => ElfOsAbi::Solaris,
        3 => ElfOsAbi::Linux, other => ElfOsAbi::Other(other),
    };
    let arch_code = read_u16(data, 18, endian)?;
    let arch = match arch_code {
        3 => ElfArch::X86, 62 => ElfArch::X86_64, 40 => ElfArch::Arm,
        183 => ElfArch::Aarch64, other => ElfArch::Other(other),
    };
    let entry_point = if class == ElfClass::Elf64 {
        read_u64(data, 24, endian)?
    } else {
        u64::from(read_u32(data, 24, endian)?)
    };

    Ok(ElfHeader { class, endian, version, abi, arch, entry_point, is_dynamic: false })
}

fn parse_sections(data: &[u8], header: &ElfHeader) -> Result<Vec<ElfSection>, AnalyzeError> {
    let mut sections = Vec::new();
    if header.class == ElfClass::Elf64 {
        // ELF64 header layout: e_shoff at 0x28, e_shentsize at 0x3A, e_shnum at 0x3C, e_shstrndx at 0x3E
        let shoff = read_u64(data, 0x28, header.endian)?;
        let shentsize = read_u16(data, 0x3A, header.endian)? as usize;
        let num_sections = read_u16(data, 0x3C, header.endian)? as usize;
        if shentsize == 0 { return Ok(sections); }
        for i in 0..num_sections {
            let offset = shoff + i as u64 * shentsize as u64;
            if offset >= data.len() as u64 { break; }
            // Elf64_Shdr: name at 0, type at 4, flags at 8, addr at 16, offset at 24, size at 32, link at 40, entsize at 56
            let r#type = read_u32(data, offset as usize + 4, header.endian)?;
            let addr = read_u64(data, offset as usize + 16, header.endian)?;
            let off = read_u64(data, offset as usize + 24, header.endian)?;
            let size = read_u64(data, offset as usize + 32, header.endian)?;
            let link = read_u32(data, offset as usize + 40, header.endian)?;
            let entsize = read_u64(data, offset as usize + 56, header.endian)?;
            sections.push(ElfSection { index: i as usize, name: String::new(), r#type, addr, offset: off, size, link, entsize });
        }
        // Now resolve section names using .shstrtab
        let shstrndx = read_u16(data, 0x3E, header.endian)? as usize;
        if shstrndx < sections.len() {
            let shstrtab_offset = sections[shstrndx].offset;
            for sec in &mut sections {
                // Re-read name offset from the original data (sec.name is empty string)
                let idx = sec.index;
                let off = shoff + idx as u64 * shentsize as u64;
                let name_offset = read_u32(data, off as usize, header.endian)?;
                sec.name = get_string_at_offset(data, name_offset, shstrtab_offset);
            }
        }
    } else {
        // ELF32 header layout: e_shoff at 0x20, e_shentsize at 0x2E, e_shnum at 0x30, e_shstrndx at 0x32
        let shoff = read_u32(data, 0x20, header.endian)? as u64;
        let shentsize = read_u16(data, 0x2E, header.endian)? as usize;
        let num_sections = read_u16(data, 0x30, header.endian)? as usize;
        if shentsize == 0 { return Ok(sections); }
        for i in 0..num_sections {
            let offset = shoff + i as u64 * shentsize as u64;
            if offset >= data.len() as u64 { break; }
            // Elf32_Shdr: name at 0, type at 4, flags at 8, addr at 12, offset at 16, size at 20, link at 24, entsize at 36
            let r#type = read_u32(data, offset as usize + 4, header.endian)?;
            let addr = u64::from(read_u32(data, offset as usize + 12, header.endian)?);
            let off = u64::from(read_u32(data, offset as usize + 16, header.endian)?);
            let size = u64::from(read_u32(data, offset as usize + 20, header.endian)?);
            let link = read_u32(data, offset as usize + 24, header.endian)?;
            let entsize = u64::from(read_u32(data, offset as usize + 36, header.endian)?);
            sections.push(ElfSection { index: i as usize, name: String::new(), r#type, addr, offset: off, size, link, entsize });
        }
        // Now resolve section names using .shstrtab
        let shstrndx = read_u16(data, 0x32, header.endian)? as usize;
        if shstrndx < sections.len() {
            let shstrtab_offset = sections[shstrndx].offset;
            for sec in &mut sections {
                let idx = sec.index;
                let off = shoff + idx as u64 * shentsize as u64;
                let name_offset = read_u32(data, off as usize, header.endian)?;
                sec.name = get_string_at_offset(data, name_offset, shstrtab_offset);
            }
        }
    }
    Ok(sections)
}

fn get_string_at_offset(data: &[u8], offset: u32, strtab_offset: u64) -> String {
    let pos = (strtab_offset + offset as u64) as usize;
    if pos >= data.len() { return String::new(); }
    let mut name = String::new();
    let mut i = pos;
    while i < data.len() && data[i] != 0 {
        name.push(data[i] as char);
        i += 1;
    }
    name
}

fn parse_symbols(data: &[u8], header: &ElfHeader, sections: &[ElfSection]) -> Result<Vec<ElfSymbol>, AnalyzeError> {
    let symtab = sections.iter().find(|s| s.name == ".symtab");
    match symtab {
        Some(s) => parse_symbol_table(data, header, s),
        None => Ok(Vec::new()),
    }
}

fn parse_dynamic_symbols(data: &[u8], header: &ElfHeader, sections: &[ElfSection]) -> Result<Vec<ElfSymbol>, AnalyzeError> {
    let dynsym = sections.iter().find(|s| s.name == ".dynsym");
    match dynsym {
        Some(s) => parse_symbol_table(data, header, s),
        None => Ok(Vec::new()),
    }
}

fn parse_symbol_table(data: &[u8], header: &ElfHeader, section: &ElfSection) -> Result<Vec<ElfSymbol>, AnalyzeError> {
    let mut symbols = Vec::new();
    if section.entsize == 0 || section.size == 0 { return Ok(symbols); }
    let num_symbols = (section.size / section.entsize) as usize;
    
    // Find the linked string table section (.strtab or .dynstr)
    // The link field in the symbol table header points to it
    let strtab_offset = if section.link > 0 {
        // We need to find this section by index - search for it
        let all_sections = parse_sections(data, header)?;
        if (section.link as usize) < all_sections.len() {
            all_sections[section.link as usize].offset
        } else {
            section.offset  // fallback
        }
    } else {
        section.offset
    };
    
    for i in 0..num_symbols {
        let offset = section.offset + (i * section.entsize as usize) as u64;
        if offset >= data.len() as u64 { break; }
        let name_offset = read_u32(data, offset as usize, header.endian)?;
        let value = if header.class == ElfClass::Elf64 {
            read_u64(data, (offset + 8) as usize, header.endian)?
        } else {
            u64::from(read_u32(data, (offset + 4) as usize, header.endian)?)
        };
        let size = if header.class == ElfClass::Elf64 {
            read_u64(data, (offset + 16) as usize, header.endian)?
        } else {
            u64::from(read_u32(data, (offset + 8) as usize, header.endian)?)
        };
        let info = data[offset as usize];
        let binding = info >> 4;
        let sym_type = info & 0xf;
        let section_index = if header.class == ElfClass::Elf64 {
            read_u16(data, (offset + 28) as usize, header.endian)? as usize
        } else {
            read_u16(data, (offset + 12) as usize, header.endian)? as usize
        };
        let name = get_string_at_offset(data, name_offset, strtab_offset);
        symbols.push(ElfSymbol { name, value, size, binding, r#type: sym_type, section_index });
    }
    Ok(symbols)
}

fn parse_dependencies(_data: &[u8], _header: &ElfHeader, _sections: &[ElfSection]) -> Result<Vec<ElfDependency>, AnalyzeError> {
    Ok(Vec::new())
}

fn extract_strings(data: &[u8]) -> Vec<String> {
    let re = Regex::new(r"[A-Za-z0-9_][A-Za-z0-9_]{3,}").unwrap();
    let mut strings = Vec::new();
    let mut i = 0;
    while i < data.len() {
        if let Some(m) = re.find(&String::from_utf8_lossy(&data[i..])) {
            let s = m.as_str().to_string();
            if !strings.contains(&s) { strings.push(s); }
            i += m.end();
        } else { break; }
    }
    strings.truncate(1000);
    strings
}

fn read_u16(data: &[u8], offset: usize, endian: ElfEndian) -> Result<u16, AnalyzeError> {
    if offset + 2 > data.len() { return Err(AnalyzeError::InvalidFormat("truncated ELF".to_string())); }
    match endian {
        ElfEndian::Little => Ok(u16::from_le_bytes([data[offset], data[offset + 1]])),
        ElfEndian::Big => Ok(u16::from_be_bytes([data[offset], data[offset + 1]])),
    }
}

fn read_u32(data: &[u8], offset: usize, endian: ElfEndian) -> Result<u32, AnalyzeError> {
    if offset + 4 > data.len() { return Err(AnalyzeError::InvalidFormat("truncated ELF".to_string())); }
    match endian {
        ElfEndian::Little => Ok(u32::from_le_bytes([data[offset], data[offset + 1], data[offset + 2], data[offset + 3]])),
        ElfEndian::Big => Ok(u32::from_be_bytes([data[offset], data[offset + 1], data[offset + 2], data[offset + 3]])),
    }
}

fn read_u64(data: &[u8], offset: usize, endian: ElfEndian) -> Result<u64, AnalyzeError> {
    if offset + 8 > data.len() { return Err(AnalyzeError::InvalidFormat("truncated ELF".to_string())); }
    match endian {
        ElfEndian::Little => Ok(u64::from_le_bytes([data[offset], data[offset + 1], data[offset + 2], data[offset + 3], data[offset + 4], data[offset + 5], data[offset + 6], data[offset + 7]])),
        ElfEndian::Big => Ok(u64::from_be_bytes([data[offset], data[offset + 1], data[offset + 2], data[offset + 3], data[offset + 4], data[offset + 5], data[offset + 6], data[offset + 7]])),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_elf_header() {
        let result = ElfParser::analyze("/bin/ls");
        assert!(result.is_ok(), "Failed to parse /bin/ls: {:?}", result.err());
        let analysis = result.unwrap();
        assert_eq!(analysis.data.header.arch.to_string(), "x86_64");
        assert_eq!(analysis.data.header.endian.to_string(), "little");
    }

    #[test]
    fn test_extract_strings() {
        let data = b"hello world\x00some text\x00";
        let strings = extract_strings(data);
        assert!(!strings.is_empty());
    }
}