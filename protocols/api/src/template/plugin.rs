/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::path::PathBuf;

use agent_utils::KeyVault;
use etc_base::ProtoQueryMap;

use crate::{
    plugin::DataMap, 
    APIPlugin, 
    Input, 
    Plugin as ProtPlugin
};
use crate::error::{DTError as APIDTError, Result as APIResult};
use crate::input::{FieldSpec, ParameterType, TableSpec, ValueTypes};

use super::{Config, Error, Result};


pub struct Plugin {
    key_vault: KeyVault,
    cache_dir: PathBuf,
    config: Config,
}

impl Plugin {
    pub fn new(
        cache_dir: PathBuf,
        key_vault: KeyVault,
        config: Config,
    ) -> Self {
        Self {
            key_vault,
            cache_dir,
            config,
        }
    }
}

#[async_trait::async_trait]
impl APIPlugin for Plugin {
    async fn run_queries(
        &self,
        input: &Input,
        query: &ProtoQueryMap,
    ) -> APIResult<DataMap> {
        todo!()
    }
}
