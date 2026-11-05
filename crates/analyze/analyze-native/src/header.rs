use crate::types::{ElfArch, ElfClass, ElfEndian, ElfHeader, ElfOsAbi};
use analyze_common::AnalyzeError;

const EI_CLASS: usize = 4;
const EI_DATA: usize = 5;
const EI_VERSION: usize = 6;
const EI_OSABI: usize = 7;
const ELFDATA2LSB: u8 = 1;
const ELFDATA2MSB: u8 = 2;
const ELFCLASS32: u8 = 1;
const ELFCLASS64: u8 = 2;
const EV_CURRENT: u8 = 1;

pub(crate) fn parse_elf_header(data: &[u8]) -> Result<ElfHeader, AnalyzeError> {
    let class = match data[EI_CLASS] {
        ELFCLASS32 => ElfClass::Elf32,
        ELFCLASS64 => ElfClass::Elf64,
        other => {
            return Err(AnalyzeError::InvalidFormat(format!(
                "invalid ELF class: {}",
                other
            )));
        }
    };
    let endian = match data[EI_DATA] {
        ELFDATA2LSB => ElfEndian::Little,
        ELFDATA2MSB => ElfEndian::Big,
        other => {
            return Err(AnalyzeError::InvalidFormat(format!(
                "invalid ELF endianness: {}",
                other
            )));
        }
    };
    let version = data[EI_VERSION];
    if version != EV_CURRENT {
        return Err(AnalyzeError::InvalidFormat(format!(
            "invalid ELF version: {}",
            version
        )));
    }
    let abi_code = data[EI_OSABI];
    let abi = match abi_code {
        0 => ElfOsAbi::SystemV,
        1 => ElfOsAbi::Hurd,
        2 => ElfOsAbi::Solaris,
        3 => ElfOsAbi::Linux,
        other => ElfOsAbi::Other(other),
    };
    let arch_code = crate::read::read_u16(data, 18, endian)?;
    let arch = match arch_code {
        3 => ElfArch::X86,
        62 => ElfArch::X86_64,
        40 => ElfArch::Arm,
        183 => ElfArch::Aarch64,
        other => ElfArch::Other(other),
    };
    let entry_point = if class == ElfClass::Elf64 {
        crate::read::read_u64(data, 24, endian)?
    } else {
        u64::from(crate::read::read_u32(data, 24, endian)?)
    };

    Ok(ElfHeader {
        class,
        endian,
        version,
        abi,
        arch,
        entry_point,
        is_dynamic: false,
    })
}
