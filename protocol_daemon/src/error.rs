/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::fmt::Debug;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error<D, E> {
    #[error("missing input")]
    MissingInput,
    #[error("missing config")]
    MissingConfig,
    #[error("failed to deserialize input: {0}")]
    DecodeInput(serde_json::Error),
    #[error("failed to deserialize config: {0}")]
    DecodeConfig(serde_json::Error),
    #[error("failed to join inputs: {0}")]
    AppendInput(agent_utils::Error),
    #[error(transparent)]
    Plugin(D),
    #[error(transparent)]
    PluginType(E),
}
