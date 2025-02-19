/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::net::IpAddr;
use std::{collections::HashMap, iter::IntoIterator};

use chrono::Utc;
use dbschema::Timestamped;
use metrics_types::{
    AggregatedStatus, ByEventCategory, Data, Grouping, ItemTypeId, Metric,
    Metrics, MetricsError, MetricsInfo, MetricsResult, MetricsSuccess,
    MetricsTable,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::sync::mpsc;

use nmap::nping::{nping_host, NPingMode};

use super::super::error::Result;

#[derive(
    Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug,
)]
pub struct NPingTask {
    host_id: String,
    #[serde(deserialize_with = "agent_serde::human_readable::deserialize")]
    #[serde(serialize_with = "agent_serde::human_readable::serialize")]
    ip_addr: IpAddr,
    ping_mode: NPingMode,
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug)]
pub struct NPingKey(IpAddr);

impl NPingTask {
    pub fn key(&self) -> NPingKey {
        NPingKey(self.ip_addr)
    }

    pub async fn run(
        &self,
        data_sender: &mpsc::Sender<(
            String,
            String,
            Timestamped<MetricsTable<Data<Value>>>,
        )>,
    ) -> Result<()> {
        let result =
            match nping_host(&self.ip_addr.to_string(), self.ping_mode).await {
                Ok(s) => MetricsResult::Success(MetricsSuccess {
                    info: MetricsInfo {
                        status: AggregatedStatus::default(),
                        subtable_status: HashMap::new(),
                        warnings: vec![],
                        inventory_status: None,
                        status_by_category: ByEventCategory::default(),
                    },
                    metrics: vec![Metrics {
                        entity_id: None,
                        grouping: Grouping::Item(self.host_id.to_string()),
                        status: None,
                        status_by_category: ByEventCategory::default(),
                        metrics: IntoIterator::into_iter([
                            (
                                "max_rtt".to_string(),
                                Metric {
                                    status: None,
                                    value: s.rtt.max_rtt.map(|v| Ok(json!(v))),
                                    relative: None,
                                },
                            ),
                            (
                                "min_rtt".to_string(),
                                Metric {
                                    status: None,
                                    value: s.rtt.min_rtt.map(|v| Ok(json!(v))),
                                    relative: None,
                                },
                            ),
                            (
                                "avg_rtt".to_string(),
                                Metric {
                                    status: None,
                                    value: s.rtt.avg_rtt.map(|v| Ok(json!(v))),
                                    relative: None,
                                },
                            ),
                            (
                                "sent_pkts".to_string(),
                                Metric {
                                    status: None,
                                    value: Some(Ok(json!(&s.pkts.sent_pkts))),
                                    relative: None,
                                },
                            ),
                            (
                                "sent_bytes".to_string(),
                                Metric {
                                    status: None,
                                    value: Some(Ok(json!(&s.pkts.sent_bytes))),
                                    relative: None,
                                },
                            ),
                            (
                                "rcvd_pkts".to_string(),
                                Metric {
                                    status: None,
                                    value: Some(Ok(json!(&s.pkts.rcvd_pkts))),
                                    relative: None,
                                },
                            ),
                            (
                                "rcvd_bytes".to_string(),
                                Metric {
                                    status: None,
                                    value: Some(Ok(json!(&s.pkts.rcvd_bytes))),
                                    relative: None,
                                },
                            ),
                            (
                                "lost_pkts".to_string(),
                                Metric {
                                    status: None,
                                    value: Some(Ok(json!(&s.pkts.lost_pkts))),
                                    relative: Some(Ok(json!(
                                        &s.pkts.lost_pkts_rel
                                    ))),
                                },
                            ),
                        ])
                        .collect(),
                    }],
                }),
                Err(e) => MetricsResult::Error(MetricsError {
                    message: e.to_string(),
                }),
            };

        Ok(data_sender
            .send((
                "nping".to_string(),
                "nping".to_string(),
                Timestamped {
                    timestamp: Utc::now(),
                    value: MetricsTable {
                        queried_item_type: ItemTypeId::from(
                            "MP/builtin/host".to_string(),
                        ),
                        queried_item_id: self.host_id.to_string(),
                        item_type: ItemTypeId::from("host".to_string()),
                        result,
                    },
                },
            ))
            .await?)
    }
}
