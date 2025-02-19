/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

pub mod database;
mod error;
pub mod pyrepr;
mod template;
mod utils;
#[cfg(feature = "key-reader")]
pub mod vault;

pub use database::{DBId, DBObj};
pub use error::{Error, Result};
pub use template::Template;
#[cfg(feature = "trust-dns-resolver")]
pub use utils::{ip_lookup, ip_lookup_one, ip_lookup_one_sync, ip_lookup_sync};
pub use utils::{quote_filename, unquote_filename};
pub use utils::{Key, KeyFor, NamedObj};
pub use utils::{TryAppend, TryAppendState};
pub use utils::{TryGet, TryGetFrom};
#[cfg(feature = "key-reader")]
pub use vault::KeyVault;
