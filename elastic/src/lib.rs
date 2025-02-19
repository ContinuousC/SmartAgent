/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

mod error;
mod output;
mod state;

pub use error::{Error, Result};
pub use output::{write_events, write_output};
pub use output::{ElasticFieldName, ElasticTableName};
