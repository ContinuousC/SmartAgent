/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use serde::{Deserialize, Serialize};

use crate::task_schedule::TaskSchedule;

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Default)]
pub struct Config {
    pub(crate) tasks: Vec<TaskSchedule>,
}
