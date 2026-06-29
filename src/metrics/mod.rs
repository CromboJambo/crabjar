/// Workspace metrics module.
///
/// Provides dynamic queries for test counts, module sizes, LoC totals,
/// and clippy status — replacing stale snapshot values in project_map.md
/// and ROADMAP.md.

mod test_count;
mod module_sizes;

pub use test_count::run_test_count;
pub use module_sizes::run_module_sizes;
