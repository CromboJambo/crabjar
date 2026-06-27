pub mod adapter;
pub mod config;
pub mod event_bus;
pub mod plugin;
pub mod work_item;

pub use adapter::{AdapterRegistry, IncomingMessage, OutgoingMessage, ProductAdapter};
pub use config::HostConfig;
pub use event_bus::{Event, EventBus, EventType};
pub use plugin::{Plugin, PluginContext, PluginRegistry};
pub use work_item::{Status, WorkItem};
