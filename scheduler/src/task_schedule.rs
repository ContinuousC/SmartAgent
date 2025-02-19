/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use serde::{Deserialize, Serialize};

use crate::{task::TaskKey, Schedule, Task};

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug)]
pub struct TaskSchedule {
    pub task: Task,
    pub schedule: Schedule,
}

impl TaskSchedule {
    pub fn key(&self) -> TaskKey {
        self.task.key()
    }
}
