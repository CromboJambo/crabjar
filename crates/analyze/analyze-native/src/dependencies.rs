use crate::types::{ElfDependency, ElfHeader, ElfSection};
use analyze_common::AnalyzeError;

pub(crate) fn parse_dependencies(
    _data: &[u8],
    _header: &ElfHeader,
    _sections: &[ElfSection],
) -> Result<Vec<ElfDependency>, AnalyzeError> {
    Ok(Vec::new())
}
