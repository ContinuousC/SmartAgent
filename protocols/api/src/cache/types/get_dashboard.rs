/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::{collections::HashMap, sync::Arc};

use chrono::{DateTime, Utc};
use etc_base::ProtoDataFieldId;
use protocol::CounterDb;
use serde::Deserialize;
use std::sync::Mutex;
use value::{Data, DataError, Value};

use crate::{
    cache::types::generic::{
        create_data_with_counter_db, create_enum_data, create_float_data,
        create_int_data, create_string_data,
    },
    input::FieldSpec,
};

use super::generic::{CreateTabledata, ValueSoap};

#[derive(Debug, Deserialize)]
pub struct BodyDashboard {
    #[serde(rename = "GetDashboardResponse")]
    pub response: DashboardResponse,
}

#[derive(Debug, Deserialize)]
pub struct DashboardResponse {
    #[serde(rename = "GetDashboardResult")]
    pub result: DashboardResult,
}

mod custom_date_format {

    use chrono::{DateTime, TimeZone, Utc};
    use serde::{self, Deserialize, Deserializer};

    use crate::cache::types::generic::ValueSoap;

    const FORMAT: &str = "%b %e %Y %I:%M%p";

    pub fn deserialize<'de, D>(
        deserializer: D,
    ) -> Result<Option<ValueSoap<DateTime<Utc>>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = Option::Some(String::deserialize(deserializer));

        match s {
            Some(res_) => match res_ {
                Ok(inner_str) => {
                    match Utc.datetime_from_str(&inner_str, FORMAT) {
                        Ok(value) => Ok(Some(ValueSoap { value })),
                        Err(e) => Err(serde::de::Error::custom(format!(
                            "failed to parse date from {inner_str}: {e}"
                        ))),
                    }
                }
                Err(_) => Err(serde::de::Error::custom(
                    "failed to deserialize date".to_string(),
                )),
            },
            None => Ok(None),
        }
    }
}

#[derive(Debug, Deserialize)]
enum JournalSpace {
    Normal,
    Warning,
    Troubled,
}

#[derive(Debug, Deserialize)]
pub struct DashboardResult {
    #[serde(rename = "GloRefsPerSec")]
    pub glo_refs_per_sec: Option<ValueSoap<f64>>,
    #[serde(rename = "GloRefs")]
    pub glo_refs: Option<ValueSoap<u64>>,
    #[serde(rename = "GloSets")]
    pub glo_sets: Option<ValueSoap<u64>>,
    #[serde(rename = "RouRefs")]
    pub rou_refs: Option<ValueSoap<u64>>,
    #[serde(rename = "LogicalReads")]
    pub logical_reads: Option<ValueSoap<u64>>,
    #[serde(rename = "DiskReads")]
    pub disk_reads: Option<ValueSoap<u64>>,
    #[serde(rename = "DiskWrites")]
    pub disk_writes: Option<ValueSoap<u64>>,
    #[serde(rename = "CacheEfficiency")]
    pub cache_efficiency: Option<ValueSoap<f64>>,
    #[serde(rename = "ECPAppServer")]
    pub ecpapp_server: Option<ValueSoap<String>>,
    #[serde(rename = "ECPAppSrvRate")]
    pub ecpapp_srv_rate: Option<ValueSoap<i64>>,
    #[serde(rename = "ECPDataServer")]
    pub ecpdata_server: Option<ValueSoap<String>>,
    #[serde(rename = "ECPDataSrvRate")]
    pub ecpdata_srv_rate: Option<ValueSoap<i64>>,
    #[serde(rename = "ShadowSource")]
    pub shadow_source: Option<ValueSoap<String>>,
    #[serde(rename = "ShadowServer")]
    pub shadow_server: Option<ValueSoap<String>>,
    #[serde(rename = "SystemUpTime")]
    pub system_up_time: Option<ValueSoap<String>>, // Example: 23d 23h 07m
    #[serde(default)]
    #[serde(
        rename = "LastBackup",
        with = "custom_date_format",
        skip_serializing_if = "Option::is_none"
    )]
    pub last_backup: Option<ValueSoap<DateTime<Utc>>>,
    #[serde(rename = "DatabaseSpace")]
    pub database_space: Option<ValueSoap<String>>,
    #[serde(rename = "JournalStatus")]
    pub journal_status: Option<ValueSoap<String>>,
    #[serde(rename = "JournalSpace")]
    pub journal_space: Option<ValueSoap<String>>,
    #[serde(rename = "JournalEntries")]
    pub journal_entries: Option<ValueSoap<u64>>,
    #[serde(rename = "LockTable")]
    pub lock_table: Option<ValueSoap<String>>,
    #[serde(rename = "WriteDaemon")]
    pub write_daemon: Option<ValueSoap<String>>,
    #[serde(rename = "Processes")]
    pub processes: Option<ValueSoap<i64>>,
    #[serde(rename = "CSPSessions")]
    pub cspsessions: Option<ValueSoap<i64>>,
    #[serde(rename = "SeriousAlerts")]
    pub serious_alerts: Option<ValueSoap<i64>>,
    #[serde(rename = "ApplicationErrors")]
    pub application_errors: Option<ValueSoap<i64>>,
    #[serde(rename = "LicenseLimit")]
    pub license_limit: Option<ValueSoap<i64>>,
    #[serde(rename = "LicenseType")]
    pub license_type: Option<ValueSoap<String>>,
    #[serde(rename = "LicenseCurrent")]
    pub license_current: Option<ValueSoap<i64>>,
    #[serde(rename = "LicenseHigh")]
    pub license_high: Option<ValueSoap<i64>>,
    #[serde(rename = "LicenseCurrentPct")]
    pub license_current_pct: Option<ValueSoap<i64>>,
    #[serde(rename = "LicenseHighPct")]
    pub license_high_pct: Option<ValueSoap<i64>>,
}

impl CreateTabledata for BodyDashboard {
    fn create_tabledata(
        self,
        fields: HashMap<ProtoDataFieldId, &FieldSpec>,
        counterdb: Arc<Mutex<CounterDb>>,
    ) -> Vec<HashMap<ProtoDataFieldId, Data>> {
        let item = &self.response.result;
        let row: HashMap<ProtoDataFieldId, Data> = fields
            .into_iter()
            .map(|(id, field)| match field.parameter_name.as_str() {
                "GloRefsPerSec" => {
                    create_float_data(&id, &item.glo_refs_per_sec)
                }
                "GloRefs" => create_data_with_counter_db(
                    &id,
                    field.parameter_name.clone(),
                    &item.glo_refs,
                    &counterdb,
                    &field,
                ),
                "GloSets" => create_data_with_counter_db(
                    &id,
                    field.parameter_name.clone(),
                    &item.glo_sets,
                    &counterdb,
                    &field,
                ),
                "RouRefs" => create_data_with_counter_db(
                    &id,
                    field.parameter_name.clone(),
                    &item.rou_refs,
                    &counterdb,
                    &field,
                ),
                "LogicalReads" => create_data_with_counter_db(
                    &id,
                    field.parameter_name.clone(),
                    &item.logical_reads,
                    &counterdb,
                    &field,
                ),
                "DiskReads" => create_data_with_counter_db(
                    &id,
                    field.parameter_name.clone(),
                    &item.disk_reads,
                    &counterdb,
                    &field,
                ),
                "DiskWrites" => create_data_with_counter_db(
                    &id,
                    field.parameter_name.clone(),
                    &item.disk_writes,
                    &counterdb,
                    &field,
                ),
                "CacheEfficiency" => {
                    create_float_data(&id, &item.cache_efficiency)
                }
                "ECPAppServer" => create_string_data(&id, &item.ecpapp_server),
                "ECPAppSrvRate" => create_int_data(&id, &item.ecpapp_srv_rate),
                "ECPDataServer" => {
                    create_string_data(&id, &item.ecpdata_server)
                }
                "ECPDataSrvRate" => {
                    create_int_data(&id, &item.ecpdata_srv_rate)
                }
                "ShadowSource" => create_string_data(&id, &item.shadow_source),
                "ShadowServer" => create_string_data(&id, &item.shadow_server),
                "SystemUpTime" => create_string_data(&id, &item.system_up_time),
                "LastBackup" => (
                    id.clone(),
                    match item.last_backup.clone() {
                        Some(value) => Ok(Value::Time(value.value)),
                        None => Err(DataError::Missing),
                    },
                ),
                "DatabaseSpace" => {
                    create_enum_data(&id, &item.database_space, &field)
                }
                "JournalStatus" => {
                    create_enum_data(&id, &item.journal_status, &field)
                }
                "JournalSpace" => {
                    create_enum_data(&id, &item.journal_space, &field)
                }
                "JournalEntries" => create_data_with_counter_db(
                    &id,
                    field.parameter_name.clone(),
                    &item.journal_entries,
                    &counterdb,
                    &field,
                ),
                "LockTable" => create_enum_data(&id, &item.lock_table, &field),
                "WriteDaemon" => {
                    create_enum_data(&id, &item.write_daemon, &field)
                }
                "Processes" => create_int_data(&id, &item.processes),
                "CSPSessions" => create_int_data(&id, &item.cspsessions),
                "SeriousAlerts" => create_int_data(&id, &item.serious_alerts),
                "ApplicationErrors" => {
                    create_int_data(&id, &item.application_errors)
                }
                "LicenseLimit" => create_int_data(&id, &item.license_limit),
                "LicenseType" => create_string_data(&id, &item.license_type),
                "LicenseCurrent" => create_int_data(&id, &item.license_current),
                "LicenseHigh" => create_int_data(&id, &item.license_high),
                "LicenseCurrentPct" => {
                    create_int_data(&id, &item.license_current_pct)
                }
                "LicenseHighPct" => {
                    create_int_data(&id, &item.license_high_pct)
                }
                _ => (id.clone(), Err(value::DataError::Missing)),
            })
            .collect();
        vec![row]
    }
}
