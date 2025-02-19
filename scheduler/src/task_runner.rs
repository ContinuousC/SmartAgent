/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::sync::Arc;

use chrono::{Duration, Utc};
use dbschema::Timestamped;
use etc::Spec;
use metrics_types::{Data, MetricsTable};
use protocol::PluginManager;
use serde_json::Value;
use tokio::{
    sync::{
        mpsc,
        watch::{self, error::SendError},
    },
    task::JoinHandle,
};

use crate::{Error, Result, TaskSchedule};

pub struct TaskRunner {
    task_sender: watch::Sender<Option<TaskSchedule>>,
    task_runner: JoinHandle<Result<()>>,
}

impl TaskRunner {
    pub fn new(
        task: TaskSchedule,
        plugin_manager: Arc<PluginManager>,
        etc_receiver: watch::Receiver<Arc<Spec>>,
        data_sender: mpsc::Sender<(
            String,
            String,
            Timestamped<MetricsTable<Data<Value>>>,
        )>,
    ) -> Self {
        let (task_sender, task_receiver) = watch::channel(Some(task));
        Self {
            task_sender,
            task_runner: tokio::spawn(run_task(
                task_receiver,
                plugin_manager,
                etc_receiver,
                data_sender,
            )),
        }
    }

    pub async fn update(
        &mut self,
        task: TaskSchedule,
    ) -> std::result::Result<(), TaskSchedule> {
        self.task_sender
            .send(Some(task))
            .map_err(|SendError(task)| task.unwrap())
    }

    pub async fn stop(mut self) -> Result<()> {
        if let Err(e) = self.task_sender.send(None) {
            log::warn!("failed to send close signal to task: {}", e);
        }
        tokio::select! {
            r = &mut self.task_runner => r?,
            _ = tokio::time::sleep(std::time::Duration::from_secs(60)) => {
                self.task_runner.abort();
                Err(Error::Timeout)
            }
        }
    }
}

async fn run_task(
    mut task_receiver: watch::Receiver<Option<TaskSchedule>>,
    plugin_manager: Arc<PluginManager>,
    etc_receiver: watch::Receiver<Arc<Spec>>,
    data_sender: mpsc::Sender<(
        String,
        String,
        Timestamped<MetricsTable<Data<Value>>>,
    )>,
) -> Result<()> {
    let mut last = Utc::now();

    loop {
        let task = match task_receiver.borrow().as_ref() {
            Some(task) => task.clone(),
            None => break,
        };
        let spec = etc_receiver.borrow().clone();

        let next = task.schedule.next_target(last);
        let delay = next - Utc::now();

        if let Ok(delay) = delay.to_std()
        /* > 0 */
        {
            tokio::select! {
                _ = task_receiver.changed() => continue,
                _ = tokio::time::sleep(delay) => {}
            };
        }

        let now = Utc::now();

        last = match now - next < Duration::milliseconds(500) {
            true => next,
            false => now,
        };

        if !task.schedule.is_allowed(now) {
            continue;
        }

        if let Err(e) = task
            .task
            .run(plugin_manager.as_ref(), spec.as_ref(), &data_sender)
            .await
        {
            log::warn!("task failed: {}", e);
        }
    }

    Ok(())
}
