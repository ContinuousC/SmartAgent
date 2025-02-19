/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

mod generic_plugin;
mod local_plugin;
mod plugin_manager;
// mod plugin_service;
// mod remote_plugin;
#[cfg(feature = "tokio")]
pub mod counters;
mod data_field;
mod data_table;
mod error;

#[cfg(feature = "tokio")]
pub mod auth;
#[cfg(feature = "reqwest")]
pub mod http;

mod input;
#[cfg(feature = "rpc")]
mod remote_plugin;
#[cfg(feature = "rpc")]
pub mod service;

#[cfg(feature = "tokio")]
pub use counters::CounterDb;

pub use data_field::DataFieldSpec;
pub use data_table::DataTableSpec;
pub use error::{DataTableError, Error, ErrorCategory, ErrorOrigin, Result};
pub use generic_plugin::{DataMap, GenericPlugin, ProtoDataMap};
pub use input::Input;
pub use local_plugin::LocalPlugin;
pub use plugin_manager::PluginManager;
#[cfg(feature = "rpc")]
pub use remote_plugin::RemotePlugin;
#[cfg(feature = "rpc")]
pub use service::{
    ConfigRef, InputRef, ProtoJsonDataMap, ProtocolHandler, ProtocolProto,
    ProtocolRequest, ProtocolService, ProtocolServiceStub,
};
// mod config
//pub use config::HostConfig;
