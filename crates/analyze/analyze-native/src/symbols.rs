use crate::read::*;
use crate::types::{ElfClass, ElfSection, ElfSymbol};
use analyze_common::AnalyzeError;

pub(crate) fn parse_symbols(
    data: &[u8],
    header: &crate::types::ElfHeader,
    sections: &[ElfSection],
) -> Result<Vec<ElfSymbol>, AnalyzeError> {
    let symtab = sections.iter().find(|s| s.name == ".symtab");
    match symtab {
        Some(s) => parse_symbol_table(data, header, s),
        None => Ok(Vec::new()),
    }
}

pub(crate) fn parse_dynamic_symbols(
    data: &[u8],
    header: &crate::types::ElfHeader,
    sections: &[ElfSection],
) -> Result<Vec<ElfSymbol>, AnalyzeError> {
    let dynsym = sections.iter().find(|s| s.name == ".dynsym");
    match dynsym {
        Some(s) => parse_symbol_table(data, header, s),
        None => Ok(Vec::new()),
    }
}

fn parse_symbol_table(
    data: &[u8],
    header: &crate::types::ElfHeader,
    section: &ElfSection,
) -> Result<Vec<ElfSymbol>, AnalyzeError> {
    let mut symbols = Vec::new();
    if section.entsize == 0 || section.size == 0 {
        return Ok(symbols);
    }
    let num_symbols = (section.size / section.entsize) as usize;

    // Find the linked string table section (.strtab or .dynstr)
    // The link field in the symbol table header points to it
    let strtab_offset = if section.link > 0 {
        // We need to find this section by index - search for it
        let all_sections = crate::sections::parse_sections(data, header)?;
        if (section.link as usize) < all_sections.len() {
            all_sections[section.link as usize].offset
        } else {
            section.offset // fallback
        }
    } else {
        section.offset
    };

    for i in 0..num_symbols {
        let offset = section.offset + (i * section.entsize as usize) as u64;
        if offset >= data.len() as u64 {
            break;
        }
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
        symbols.push(ElfSymbol {
            name,
            value,
            size,
            binding,
            r#type: sym_type,
            section_index,
        });
    }
    Ok(symbols)
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
