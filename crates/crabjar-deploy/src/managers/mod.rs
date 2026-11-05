mod dinit;
mod null;
mod runit;
mod systemd;

pub use dinit::DinitManager;
pub use null::NullManager;
pub use runit::RunitManager;
pub use systemd::SystemdManager;
