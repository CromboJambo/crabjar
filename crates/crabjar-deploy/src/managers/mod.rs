mod dinit;
mod runit;
mod systemd;
mod null;

pub use dinit::DinitManager;
pub use runit::RunitManager;
pub use systemd::SystemdManager;
pub use null::NullManager;