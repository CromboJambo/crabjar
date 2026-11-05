// Terrarium simulation module — maps run contracts to crab entities.
//
// The "simulation" is a synchronous mapping from task states to crab positions,
// behaviors, and visual attributes. There is no tick loop; the habitat is
// recomputed from scratch whenever the contract changes.

pub use crate::crabs::Crab;
pub use crate::habitat::{Habitat, simulate};
