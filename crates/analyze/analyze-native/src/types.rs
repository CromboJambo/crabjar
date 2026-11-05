use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ElfArch {
    X86,
    X86_64,
    Arm,
    Aarch64,
    Other(u16),
}
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
pub enum ElfEndian {
    Little,
    Big,
}
impl std::fmt::Display for ElfEndian {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ElfEndian::Little => write!(f, "little"),
            ElfEndian::Big => write!(f, "big"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ElfClass {
    Elf32,
    Elf64,
}
impl std::fmt::Display for ElfClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ElfClass::Elf32 => write!(f, "ELF32"),
            ElfClass::Elf64 => write!(f, "ELF64"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ElfOsAbi {
    SystemV,
    Hurd,
    Solaris,
    Linux,
    Other(u8),
}
impl std::fmt::Display for ElfOsAbi {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ElfOsAbi::SystemV => write!(f, "systemv"),
            ElfOsAbi::Hurd => write!(f, "hurd"),
            ElfOsAbi::Solaris => write!(f, "solaris"),
            ElfOsAbi::Linux => write!(f, "linux"),
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
pub struct ElfDependency {
    pub name: String,
}

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
