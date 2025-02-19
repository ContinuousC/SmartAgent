/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::collections::hash_map::Entry;
use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use agent_utils::TryAppend;
use etc_base::{CheckId, FieldId, MPId, QueryId, TableId};
use query::Query;

use super::check::CheckSpec;
use super::field::FieldSpec;
use super::mp::MPSpec;
use super::table::TableSpec;
use crate::ConfigRule;

/// This structure contains the subset from the EventTypeCatalog database
/// needed to run the agent for one or more Monitoring Packs.
#[derive(Serialize, Deserialize, Clone, Default, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct Etc {
    /// Etc Objects.
    #[serde(rename = "MPs")]
    pub mps: HashMap<MPId, MPSpec>,
    pub checks: HashMap<CheckId, CheckSpec>,
    pub queries: HashMap<QueryId, Query>,
    pub tables: HashMap<TableId, TableSpec>,
    pub fields: HashMap<FieldId, FieldSpec>,
    #[serde(rename = "ConfigRules")]
    pub config_rules: HashMap<FieldId, HashMap<MPId, Vec<ConfigRule>>>,
}

impl TryAppend for Etc {
    fn try_append(&mut self, other: Self) -> agent_utils::Result<()> {
        self.mps.try_append(other.mps)?;
        self.checks.try_append(other.checks)?;
        self.queries.try_append(other.queries)?;
        self.tables.try_append(other.tables)?;
        self.fields.try_append(other.fields)?;

        // add all lookups from other
        for (fid, mps) in other.config_rules.into_iter() {
            match self.config_rules.entry(fid) {
                Entry::Vacant(ent) => {
                    ent.insert(mps);
                }
                Entry::Occupied(mut ent) => {
                    for (mpid, lookups) in mps.into_iter() {
                        match ent.get_mut().entry(mpid) {
                            Entry::Vacant(ent) => {
                                ent.insert(lookups);
                            }
                            Entry::Occupied(mut ent) => {
                                ent.get_mut().extend(lookups)
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }
}
