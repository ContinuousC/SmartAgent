/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::collections::HashMap;
use std::sync::Arc;

use serde_json::Value;
use tokio::sync::{mpsc, watch};
use tokio::task::JoinHandle;

use dbschema::Timestamped;
use metrics_types::{Data, MetricsTable};

use etc::Spec;
use protocol::PluginManager;

use crate::task_runner::TaskRunner;

use super::config::Config;
use super::error::Result;

#[derive(Debug)]
pub struct Scheduler {
    config_sender: watch::Sender<Arc<Config>>,
    cmd_sender: mpsc::Sender<Cmd>,
    worker: JoinHandle<Result<()>>,
}

#[derive(Debug)]
enum Cmd {
    Exit,
}

impl Scheduler {
    pub fn new(
        plugin_manager: Arc<PluginManager>,
        etc_receiver: watch::Receiver<Arc<Spec>>,
        data_sender: mpsc::Sender<(
            String,
            String,
            Timestamped<MetricsTable<Data<Value>>>,
        )>,
    ) -> Self {
        let (config_sender, config_receiver) =
            watch::channel(Arc::new(Config::default()));
        let (cmd_sender, cmd_receiver) = mpsc::channel(10);
        let worker = tokio::spawn(Self::worker(
            plugin_manager,
            config_receiver,
            etc_receiver,
            cmd_receiver,
            data_sender,
        ));
        Self {
            config_sender,
            cmd_sender,
            worker,
        }
    }

    pub async fn shutdown(self) -> Result<()> {
        self.cmd_sender.send(Cmd::Exit).await?;
        self.worker.await?
    }

    pub async fn update_config(&self, config: Config) -> Result<()> {
        Ok(self.config_sender.send(Arc::new(config))?)
    }

    async fn worker(
        plugin_manager: Arc<PluginManager>,
        mut config_receiver: watch::Receiver<Arc<Config>>,
        etc_receiver: watch::Receiver<Arc<Spec>>,
        mut cmd_receiver: mpsc::Receiver<Cmd>,
        data_sender: mpsc::Sender<(
            String,
            String,
            Timestamped<MetricsTable<Data<Value>>>,
        )>,
    ) -> Result<()> {
        let config: Arc<Config> = config_receiver.borrow().clone();
        log::debug!("Scheduling {} task(s)", config.tasks.len());

        let mut tasks = config.tasks.iter().cloned().fold(
            HashMap::new(),
            |mut map, task| {
                let key = task.key();
                map.entry(key)
                    .or_insert_with(Vec::new)
                    .push(TaskRunner::new(
                        task,
                        plugin_manager.clone(),
                        etc_receiver.clone(),
                        data_sender.clone(),
                    ));
                map
            },
        );

        loop {
            tokio::select! {
                _ = config_receiver.changed() => {
                    log::debug!("Config changed; reloading tasks...");

                    let new_tasks = config_receiver
                        .borrow()
                        .tasks
                        .iter()
                        .fold(HashMap::new(), |mut map, task| {
                            map.entry(task.key()).or_insert_with(Vec::new).push(task.clone());
                            map
                        });

                    let (updated,removed) = tasks.into_iter().partition(|(k, _)| new_tasks.contains_key(k));
                    tasks = updated; // let removed = tasks.drain_filter(|(k,_)| !new_tasks.contains_key(k))
                    let mut stopped = 0;
                    let mut started = 0;
                    let mut updated = 0;
                    let mut failed = 0;

                    /* Remove tasks whose key is no longer found. */
                    for task_runner in removed.into_values().flatten() {
                        match task_runner.stop().await {
                            Ok(()) => {
                                stopped += 1;
                            },
                            Err(e) => {
                                log::warn!("failed to stop task: {}", e);
                                failed += 1;
                            }
                        }
                    }

                    /* For tasks with the same key, we assume they
                     * correspond to the "same task", in the same
                     * order. */
                    for (key, new_tasks) in new_tasks {
                        let mut old_tasks = tasks.remove(&key).unwrap_or_default().into_iter();
                        let mut new_tasks = new_tasks.into_iter();
                        let mut updated_tasks = Vec::new();

                        /* Update existing tasks. */
                        for (mut task_runner, new_task) in (&mut old_tasks).zip(&mut new_tasks) {
                            match task_runner.update(new_task).await {
                                Ok(()) => {
                                    updated_tasks.push(task_runner);
                                    updated += 1;
                                },
                                Err(new_task) => {
                                    log::warn!(
                                        "failed to update task; replacing with new task runner"
                                    );
                                    match task_runner.stop().await {
                                        Ok(()) => {
                                            stopped += 1;
                                        },
                                        Err(e) => {
                                            log::warn!("failed to stop task: {}", e);
                                            failed += 1;
                                        }
                                    }
                                    updated_tasks.push(TaskRunner::new(new_task, plugin_manager.clone(), etc_receiver.clone(), data_sender.clone()));
                                    started += 1;
                                }
                            }
                        }

                        /* Stop remaining old tasks. */
                        for task_runner in old_tasks {
                            if let Err(e) = task_runner.stop().await {
                                log::warn!("failed to stop task: {}", e);
                            }
                        }

                        /* Start remaining new tasks. */
                        for task in new_tasks {
                            updated_tasks.push(TaskRunner::new(task, plugin_manager.clone(),
                                                           etc_receiver.clone(),
                                                               data_sender.clone()));
                            started += 1;
                        }

                        tasks.insert(key, updated_tasks);
                    }

                    log::info!("Tasks reloaded: {} task(s) updated, {} task(s) started, {} task(s) stopped, forgetting {} task(s) that failed to stop", updated, started, stopped, failed);
                },
                cmd = cmd_receiver.recv() => match cmd {
                    None => {
                        log::debug!("Scheduler command channel closed unexpectedly; \
                                     exiting...");
                        break;
                    },
                    Some(Cmd::Exit) => {
                        log::debug!("Received exit command; exiting...");
                        break;
                    }
                }
            }
        }

        log::debug!("Stopping scheduled tasks...");
        for task_runner in tasks.into_values().flatten() {
            if let Err(e) = task_runner.stop().await {
                log::warn!("task failed to stop: {}", e);
            }
        }
        log::debug!("All scheduled tasks have stopped!");

        log::debug!("Scheduler is shut down.");
        Ok(())
    }
}
