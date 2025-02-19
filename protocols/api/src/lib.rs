/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

mod config;
pub mod error;
mod input;
pub mod livestatus;
pub mod plugin;
pub mod soap;

pub mod azure;
pub mod cache;
pub mod elastic;
pub mod ldap;
pub mod mirth;
pub mod ms_graph;
pub mod proxmox;
pub mod unity;
pub mod vmware;
pub mod xenapp_director;

pub use config::Config;
pub use error::{DTWarning, Error, Result};
pub use input::Input;
pub use plugin::{APIPlugin, Plugin};

use std::{
    convert::TryInto,
    time::{SystemTime, UNIX_EPOCH},
};

pub fn get_current_unix_timestamp() -> usize {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_micros()
        .try_into()
        .unwrap()
}
