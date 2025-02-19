/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

#[derive(
    Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug,
)]
#[serde(rename_all = "lowercase")]
pub enum Schedule {
    Period(#[serde(with = "agent_serde::duration")] Duration),
}

impl Schedule {
    /// Provide the ideal next scheduler target.
    pub(crate) fn next_target(&self, last: DateTime<Utc>) -> DateTime<Utc> {
        match self {
            Self::Period(p) => last + *p,
        }
    }

    /// Verify checking is allowed at the actual scheduled time.
    pub(crate) fn is_allowed(&self, _target: DateTime<Utc>) -> bool {
        true
    }
}
