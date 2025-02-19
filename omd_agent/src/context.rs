/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use etc_base::{CheckId, MPId};
use std::collections::HashSet;

use crate::config::HostConfig;
use etc::Spec;
use std::{path::PathBuf, sync::Arc};

#[derive(Debug)]
pub struct Options {
    pub host_addr: Option<String>,
    pub host_name: String,
    pub omd_compat: bool,
    pub mode: Mode,
    pub checks: Option<HashSet<CheckId>>,
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Debug)]
pub enum Mode {
    Inventory,
    Active,
    Current,
}

pub struct Context {
    pub options: Options,
    pub spec: Arc<Spec>,
    pub config: HostConfig,
    pub site_name: String,
    pub cache_dir: PathBuf,
    pub parsers_dir: PathBuf,
}

impl Context {
    pub fn get_mps(&self) -> HashSet<&MPId> {
        self.spec
            .etc
            .mps
            .iter()
            .filter(|(_mp_id, mp)| self.config.tags.contains(&mp.tag))
            .map(|(mp_id, _mp)| mp_id)
            .collect()
    }
}
