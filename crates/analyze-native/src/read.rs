use crate::types::ElfEndian;
use analyze_common::AnalyzeError;

pub(crate) fn read_u16(data: &[u8], offset: usize, endian: ElfEndian) -> Result<u16, AnalyzeError> {
    if offset + 2 > data.len() {
        return Err(AnalyzeError::InvalidFormat("truncated ELF".to_string()));
    }
    match endian {
        ElfEndian::Little => Ok(u16::from_le_bytes([data[offset], data[offset + 1]])),
        ElfEndian::Big => Ok(u16::from_be_bytes([data[offset], data[offset + 1]])),
    }
}

pub(crate) fn read_u32(data: &[u8], offset: usize, endian: ElfEndian) -> Result<u32, AnalyzeError> {
    if offset + 4 > data.len() {
        return Err(AnalyzeError::InvalidFormat("truncated ELF".to_string()));
    }
    match endian {
        ElfEndian::Little => Ok(u32::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ])),
        ElfEndian::Big => Ok(u32::from_be_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ])),
    }
}

pub(crate) fn read_u64(data: &[u8], offset: usize, endian: ElfEndian) -> Result<u64, AnalyzeError> {
    if offset + 8 > data.len() {
        return Err(AnalyzeError::InvalidFormat("truncated ELF".to_string()));
    }
    match endian {
        ElfEndian::Little => Ok(u64::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
            data[offset + 4],
            data[offset + 5],
            data[offset + 6],
            data[offset + 7],
        ])),
        ElfEndian::Big => Ok(u64::from_be_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
            data[offset + 4],
            data[offset + 5],
            data[offset + 6],
            data[offset + 7],
        ])),
    }
}
