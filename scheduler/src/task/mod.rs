/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

mod check_task;
mod nping_task;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::mpsc;

use dbschema::Timestamped;
use metrics_types::{Data, MetricsTable};

use etc::Spec;
use protocol::PluginManager;

use super::error::Result;

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Task {
    NPing(nping_task::NPingTask),
    Checks(check_task::CheckTask),
}

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub enum TaskKey {
    NPing(nping_task::NPingKey),
    Checks(check_task::CheckKey),
}

impl Task {
    pub fn key(&self) -> TaskKey {
        match self {
            Task::NPing(task) => TaskKey::NPing(task.key()),
            Task::Checks(task) => TaskKey::Checks(task.key()),
        }
    }

    pub async fn run(
        &self,
        plugin_manager: &PluginManager,
        spec: &Spec,
        data_sender: &mpsc::Sender<(
            String,
            String,
            Timestamped<MetricsTable<Data<Value>>>,
        )>,
    ) -> Result<()> {
        match self {
            Self::NPing(task) => task.run(data_sender).await,
            Self::Checks(task) => {
                task.run(plugin_manager, spec, data_sender).await
            }
        }
    }
}
