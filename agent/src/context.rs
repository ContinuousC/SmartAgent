/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use etc_base::CheckId;
use std::collections::HashSet;

pub struct Options {
    pub host_addr: String,
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
