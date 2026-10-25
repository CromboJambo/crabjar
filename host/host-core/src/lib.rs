pub mod adapter;
pub mod config;
pub mod email;
pub mod event_bus;
pub mod plugin;
pub mod work_item;

pub use adapter::{
    AdapterError, AdapterRegistry, IncomingMessage, OutgoingMessage, ProductAdapter,
};
pub use config::HostConfig;
pub use email::{EmailAddress, EmailError, EmailListFilter, EmailMessage, EmailProvider};
pub use event_bus::{Event, EventBus, EventType};
pub use plugin::{Plugin, PluginContext, PluginError, PluginRegistry};
pub use work_item::{Status, TaskStatus, WorkItem};
