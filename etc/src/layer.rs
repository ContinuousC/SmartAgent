/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use serde::{Deserialize, Serialize};

use dbschema::HasSchema;

#[derive(
    HasSchema,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Clone,
    Copy,
    Debug,
)]
#[serde(rename_all = "snake_case")]
pub enum Layer {
    Network,
    Infrastructure,
    Cloud,
    Application,
    EndToEnd,
}

impl Layer {
    pub fn name(&self) -> &'static str {
        match self {
            Layer::Network => "network",
            Layer::Infrastructure => "infrastructure",
            Layer::Cloud => "cloud",
            Layer::Application => "application",
            Layer::EndToEnd => "end-to-end",
        }
    }

    pub fn iter_all() -> impl Iterator<Item = Layer> {
        [
            Layer::Network,
            Layer::Infrastructure,
            Layer::Cloud,
            Layer::Application,
            Layer::EndToEnd,
        ]
        .iter()
        .copied()
    }
}
