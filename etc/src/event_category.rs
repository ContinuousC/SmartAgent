/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::fmt::Display;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "snake_case")]
pub enum EventCategory {
    Availability,
    Performance,
    Security,
    Health,
    Config,
}

impl EventCategory {
    pub const OPTIONS: &'static [Self] = &[
        Self::Availability,
        Self::Performance,
        Self::Security,
        Self::Health,
        Self::Config,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            EventCategory::Availability => "availability",
            EventCategory::Performance => "performance",
            EventCategory::Security => "security",
            EventCategory::Health => "health",
            EventCategory::Config => "config",
        }
    }
}

impl Display for EventCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl EventCategory {
    pub fn iter_all() -> impl Iterator<Item = EventCategory> {
        [
            EventCategory::Availability,
            EventCategory::Performance,
            EventCategory::Health,
            EventCategory::Config,
            EventCategory::Security,
        ]
        .iter()
        .copied()
    }
}
