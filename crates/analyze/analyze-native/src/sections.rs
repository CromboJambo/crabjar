use crate::read::*;
use crate::types::{ElfClass, ElfSection};
use analyze_common::AnalyzeError;

pub(crate) fn parse_sections(
    data: &[u8],
    header: &crate::types::ElfHeader,
) -> Result<Vec<ElfSection>, AnalyzeError> {
    let mut sections = Vec::new();
    if header.class == ElfClass::Elf64 {
        // ELF64 header layout: e_shoff at 0x28, e_shentsize at 0x3A, e_shnum at 0x3C, e_shstrndx at 0x3E
        let shoff = read_u64(data, 0x28, header.endian)?;
        let shentsize = read_u16(data, 0x3A, header.endian)? as usize;
        let num_sections = read_u16(data, 0x3C, header.endian)? as usize;
        if shentsize == 0 {
            return Ok(sections);
        }
        for i in 0..num_sections {
            let offset = shoff + i as u64 * shentsize as u64;
            if offset >= data.len() as u64 {
                break;
            }
            // Elf64_Shdr: name at 0, type at 4, flags at 8, addr at 16, offset at 24, size at 32, link at 40, entsize at 56
            let r#type = read_u32(data, offset as usize + 4, header.endian)?;
            let addr = read_u64(data, offset as usize + 16, header.endian)?;
            let off = read_u64(data, offset as usize + 24, header.endian)?;
            let size = read_u64(data, offset as usize + 32, header.endian)?;
            let link = read_u32(data, offset as usize + 40, header.endian)?;
            let entsize = read_u64(data, offset as usize + 56, header.endian)?;
            sections.push(ElfSection {
                index: i,
                name: String::new(),
                r#type,
                addr,
                offset: off,
                size,
                link,
                entsize,
            });
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
        if shentsize == 0 {
            return Ok(sections);
        }
        for i in 0..num_sections {
            let offset = shoff + i as u64 * shentsize as u64;
            if offset >= data.len() as u64 {
                break;
            }
            // Elf32_Shdr: name at 0, type at 4, flags at 8, addr at 12, offset at 16, size at 20, link at 24, entsize at 36
            let r#type = read_u32(data, offset as usize + 4, header.endian)?;
            let addr = u64::from(read_u32(data, offset as usize + 12, header.endian)?);
            let off = u64::from(read_u32(data, offset as usize + 16, header.endian)?);
            let size = u64::from(read_u32(data, offset as usize + 20, header.endian)?);
            let link = read_u32(data, offset as usize + 24, header.endian)?;
            let entsize = u64::from(read_u32(data, offset as usize + 36, header.endian)?);
            sections.push(ElfSection {
                index: i,
                name: String::new(),
                r#type,
                addr,
                offset: off,
                size,
                link,
                entsize,
            });
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
    if pos >= data.len() {
        return String::new();
    }
    let mut name = String::new();
    let mut i = pos;
    while i < data.len() && data[i] != 0 {
        name.push(data[i] as char);
        i += 1;
    }
    name
}
