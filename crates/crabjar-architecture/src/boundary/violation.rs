/// A single dependency violation found during boundary checking.
#[derive(Debug, Clone)]
pub struct Violation {
    /// The crate that has the invalid dependency.
    pub from_crate: String,
    /// The layer the `from_crate` belongs to.
    pub from_layer: usize,
    /// The dependency that violates the boundary.
    pub to_crate: String,
    /// The layer the `to_crate` belongs to.
    pub to_layer: usize,
}

impl std::fmt::Display for Violation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use crate::layer::{allowed_dependencies, layer_name};
        let max_allowed = *allowed_dependencies(self.from_layer).last().unwrap_or(&0);
        write!(
            f,
            "LAYER VIOLATION: {} (layer {}:{}) depends on {} (layer {}:{}) \\\n\
             — layer {} may only depend on layers 0..={}",
            self.from_crate,
            self.from_layer,
            layer_name(self.from_layer).unwrap_or("unknown"),
            self.to_crate,
            self.to_layer,
            layer_name(self.to_layer).unwrap_or("unknown"),
            layer_name(self.from_layer).unwrap_or("unknown"),
            max_allowed,
        )
    }
}
