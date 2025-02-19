/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::collections::HashMap;
use std::error::Error;
use std::fmt::Debug;

use async_trait::async_trait;
use serde::{de::DeserializeOwned, Serialize};

use agent_utils::TryAppend;
use etc_base::{AnnotatedResult, ProtoDataFieldId, ProtoDataTableId};
use etc_base::{ProtoQueryMap, ProtoRow};

use super::data_field::DataFieldSpec;
use super::data_table::DataTableSpec;

/* Plugin interface */

#[async_trait]
pub trait LocalPlugin: Send + Sync {
    /// General protocol-specific error type.
    type Error: Error + Debug + Send + Sync + 'static;
    /// General protocol-specific type error type.
    type TypeError: Error + Debug + Send + Sync + 'static;
    /// Protocol-specific data table error type.
    type DTError: Error + Debug + Send + Sync + 'static;
    /// Protocol-specific data table warning type.
    type DTWarning: Error + Debug + Send + Sync + 'static;

    /// Type defining the structure of the parameter definitions.
    type Input: DeserializeOwned
        + TryAppend
        + Default
        + Clone
        + Send
        + Sync
        + 'static;
    /// The configuration needed to log in to a host.
    type Config: Serialize + DeserializeOwned + Send + Sync + 'static;

    /// The unique name of the protocol.
    const PROTOCOL: &'static str;
    /// The version of the protocol plugin.
    const VERSION: &'static str;

    /* Query API. */

    fn show_queries(
        &self,
        input: &Self::Input,
        query: &ProtoQueryMap,
    ) -> Result<String, Self::Error>;

    async fn run_queries(
        &self,
        input: &Self::Input,
        config: &Self::Config,
        query: &ProtoQueryMap,
    ) -> Result<
        HashMap<
            ProtoDataTableId,
            AnnotatedResult<Vec<ProtoRow>, Self::DTWarning, Self::DTError>,
        >,
        Self::Error,
    >;

    /* Self-description API. */

    fn get_tables(
        &self,
        input: &Self::Input,
    ) -> Result<HashMap<ProtoDataTableId, DataTableSpec>, Self::TypeError>;
    fn get_fields(
        &self,
        input: &Self::Input,
    ) -> Result<HashMap<ProtoDataFieldId, DataFieldSpec>, Self::TypeError>;
}
