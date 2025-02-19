/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::cmp::Ordering;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use chrono::{DateTime, Utc};
use log::warn;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::error::{DTEResult, DTError};
use crate::ms_graph::Plugin;

// filter rapports
impl Plugin {
    pub fn filter_message(&self, value: &Value) -> DTEResult<bool> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| DTError::SystemTimeError)?
            .as_millis() as i64;

        if let Some(dt) = value.get("endDateTime") {
            if dt.is_string() {
                match serde_json::from_value::<DateTime<Utc>>(dt.clone()) {
                    Ok(ts) => {
                        if ts.timestamp_millis() < now {
                            return Ok(true);
                        }
                    }
                    Err(e) => {
                        warn!("cannot convert {} to a datetime: {}", dt, e)
                    }
                }
            }
        }
        if let Some(dt) = value.get("actionRequiredByDateTime") {
            if dt.is_string() {
                match serde_json::from_value::<DateTime<Utc>>(dt.clone()) {
                    Ok(ts) => {
                        if ts.timestamp_millis() < now {
                            return Ok(true);
                        }
                    }
                    Err(e) => {
                        warn!("cannot convert {} to a datetime: {}", dt, e)
                    }
                }
            }
        }
        Ok(false)
    }

    pub fn filter_rapport(
        &self,
        cmd_line: &str,
        mut data: Vec<HashMap<String, String>>,
    ) -> Vec<HashMap<String, String>> {
        let rapport_config = &self.config.rapports.clone().unwrap_or_default();
        let mut to_take = data.len();
        match cmd_line {
            "reports/getOneDriveUsageAccountDetail(period='D7')" => {
                let (to_take_conf, usage_filter) =
                    &rapport_config.onedrive_usage;
                to_take = *to_take_conf;
                data.sort_by_cached_key(|row| usage_filter.get_sortkey(row));
                data
            }
            "reports/getSharePointSiteUsageDetail(period='D7')" => {
                let (to_take_conf, usage_filter) =
                    &rapport_config.sharepoint_usage;
                to_take = *to_take_conf;
                data.sort_by_cached_key(|row| usage_filter.get_sortkey(row));
                data
            }
            "reports/getMailboxUsageDetail(period='D7')" => {
                let (to_take_conf, usage_filter) =
                    &rapport_config.outlook_usage;
                to_take = *to_take_conf;
                data.sort_by_cached_key(|row| usage_filter.get_sortkey(row));
                data
            }
            _ => data,
        }
        .into_iter()
        .rev()
        .take(to_take)
        .collect()
    }
}

fn get_int(row: &HashMap<String, String>, key: &str, default: u64) -> u64 {
    let s_default = default.to_string();
    row.get(key)
        .unwrap_or(&s_default)
        .parse::<u64>()
        .map(|v| if v == 0 { default } else { v })
        .unwrap_or(default)
}

fn divide(row: &HashMap<String, String>, num: &str, denum: &str) -> f64 {
    get_int(row, num, 0) as f64 / get_int(row, denum, u64::MAX) as f64
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub enum OnedriveUsage {
    Owner,
    SiteURL,
    FileCount,
    ActiveFileCount,
    StorageUsed,
    #[default]
    StorageUsedRel,
    LastActivity,
}

impl OnedriveUsage {
    pub fn get_sortkey(&self, row: &HashMap<String, String>) -> Sortable {
        match self {
            OnedriveUsage::Owner => Sortable::String(
                row.get("Owner Principal Name")
                    .cloned()
                    .unwrap_or(String::new()),
            ),
            OnedriveUsage::SiteURL => Sortable::String(
                row.get("Site URL").cloned().unwrap_or(String::new()),
            ),
            OnedriveUsage::FileCount => {
                Sortable::Integer(get_int(row, "File Count", 0))
            }
            OnedriveUsage::ActiveFileCount => {
                Sortable::Integer(get_int(row, "Active File Count", 0))
            }
            OnedriveUsage::StorageUsed => {
                Sortable::Integer(get_int(row, "Storage Used (Byte)", 0))
            }
            OnedriveUsage::StorageUsedRel => Sortable::Float(divide(
                row,
                "Storage Used (Byte)",
                "Storage Allocated (Byte)",
            )),
            OnedriveUsage::LastActivity => Sortable::String(
                row.get("Last Activity Date")
                    .cloned()
                    .unwrap_or(String::new()),
            ),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub enum OutlookUsage {
    UserPrincipalName,
    ItemCount,
    StorageUsed,
    #[default]
    StorageUsedRel,
    DeletedItemSize,
    DeletedItemSizeRel,
    LastActivity,
}

impl OutlookUsage {
    pub fn get_sortkey(&self, row: &HashMap<String, String>) -> Sortable {
        match self {
            OutlookUsage::UserPrincipalName => Sortable::String(
                row.get("User Principal Name")
                    .cloned()
                    .unwrap_or(String::new()),
            ),
            OutlookUsage::ItemCount => {
                Sortable::Integer(get_int(row, "Item Count", 0))
            }
            OutlookUsage::StorageUsed => {
                Sortable::Integer(get_int(row, "Storage Used (Byte)", 0))
            }
            OutlookUsage::StorageUsedRel => Sortable::Float(divide(
                row,
                "Storage Used (Byte)",
                "Prohibit Send/Receive Quota (Byte)",
            )),
            OutlookUsage::DeletedItemSize => {
                Sortable::Integer(get_int(row, "Deleted Item Size (Byte)", 0))
            }
            OutlookUsage::DeletedItemSizeRel => Sortable::Float(divide(
                row,
                "Deleted Item Size (Byte)",
                "Deleted Item Quota (Byte)",
            )),
            OutlookUsage::LastActivity => Sortable::String(
                row.get("Last Activity Date")
                    .cloned()
                    .unwrap_or(String::new()),
            ),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub enum SharepointUsage {
    Owner,
    SiteURL,
    FileCount,
    ActiveFileCount,
    PageViews,
    VisitedPages,
    StorageUsed,
    #[default]
    StorageUsedRel,
    LastActivity,
}

impl SharepointUsage {
    pub fn get_sortkey(&self, row: &HashMap<String, String>) -> Sortable {
        match self {
            SharepointUsage::Owner => Sortable::String(
                row.get("Owner Display Name")
                    .cloned()
                    .unwrap_or(String::new()),
            ),
            SharepointUsage::SiteURL => Sortable::String(
                row.get("Site URL").cloned().unwrap_or(String::new()),
            ),
            SharepointUsage::FileCount => {
                Sortable::Integer(get_int(row, "File Count", 0))
            }
            SharepointUsage::ActiveFileCount => {
                Sortable::Integer(get_int(row, "Active File Count", 0))
            }
            SharepointUsage::PageViews => {
                Sortable::Integer(get_int(row, "Page View Count", 0))
            }
            SharepointUsage::VisitedPages => {
                Sortable::Integer(get_int(row, "Visited Page Count", 0))
            }
            SharepointUsage::StorageUsed => {
                Sortable::Integer(get_int(row, "Storage Used (Byte)", 0))
            }
            SharepointUsage::StorageUsedRel => Sortable::Float(divide(
                row,
                "Storage Used (Byte)",
                "Storage Allocated (Byte)",
            )),
            SharepointUsage::LastActivity => Sortable::String(
                row.get("Last Activity Date")
                    .cloned()
                    .unwrap_or(String::new()),
            ),
        }
    }
}

pub enum Sortable {
    String(String),
    Float(f64),
    Integer(u64),
}

impl Ord for Sortable {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Self::String(a), Self::String(b)) => a.cmp(b),
            (Self::Float(a), Self::Float(b)) => {
                a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)
            }
            (Self::Integer(a), Self::Integer(b)) => a.cmp(b),
            _ => unreachable!(), // _ => self.to_string().cmp(other.to_string())
        }
    }
}

impl PartialOrd for Sortable {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for Sortable {}

impl PartialEq for Sortable {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}
