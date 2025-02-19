/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

#[cfg(feature = "mirth-full")]
use std::collections::HashMap;
use std::fmt::Display;

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use value::{Data, DataError, EnumValue};

use crate::input::{FieldSpec, ValueTypes};

#[cfg(feature = "mirth-full")]
use super::Timestamp;
use super::{OptValue, Value};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelSpecific {
    pub id: Value<Uuid>,
    pub name: Value<String>,
    pub description: OptValue<String>,
    pub source_connector: ChannelConnector,
    pub destination_connectors: ChannelConnectors,

    #[cfg(feature = "mirth-full")]
    pub revision: Value<u64>,
    #[cfg(feature = "mirth-full")]
    pub next_meta_data_id: Value<u64>,
    #[cfg(feature = "mirth-full")]
    pub preprocessing_script: OptValue<String>,
    #[cfg(feature = "mirth-full")]
    pub postprocessing_script: OptValue<String>,
    #[cfg(feature = "mirth-full")]
    pub deploy_script: OptValue<String>,
    #[cfg(feature = "mirth-full")]
    pub undeploy_script: OptValue<String>,
    #[cfg(feature = "mirth-full")]
    pub properties: ChannelProperties,
    #[cfg(feature = "mirth-full")]
    pub export_data: ExportData,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChannelConnectors {
    #[serde(rename = "connector")]
    pub data: Vec<ChannelConnector>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelConnector {
    pub name: Value<String>,
    pub properties: ChannelConnectorProperties,
    pub transport_name: Value<String>,
    pub mode: Value<String>, // TODO: make enum
    pub enabled: Value<bool>,

    #[cfg(feature = "mirth-full")]
    pub transformer: Transformer,
    #[cfg(feature = "mirth-full")]
    pub filter: Filter,
    #[cfg(feature = "mirth-full")]
    pub meta_data_id: Value<u64>,
    #[cfg(feature = "mirth-full")]
    pub wait_for_previous: Value<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelConnectorProperties {
    #[serde(default)]
    pub scheme: OptValue<String>, // TODO: make enum
    #[serde(default)]
    pub host: OptValue<String>,

    #[cfg(feature = "mirth-full")]
    class: String,
    #[cfg(feature = "mirth-full")]
    pub source_connector_properties: Option<SourceConnectorProperties>,
    #[cfg(feature = "mirth-full")]
    pub destination_connector_properties:
        Option<DestinationConnectorProperties>,
    #[cfg(feature = "mirth-full")]
    pub plugin_properties: PluginProperties,
    #[cfg(feature = "mirth-full")]
    #[serde(default)]
    pub poll_connector_properties: Option<PollConnectorProperties>,
    #[cfg(feature = "mirth-full")]
    #[serde(default)]
    pub file_filter: OptValue<String>,
    #[cfg(feature = "mirth-full")]
    #[serde(default)]
    pub regex: OptValue<bool>,
    #[cfg(feature = "mirth-full")]
    #[serde(default)]
    pub directory_recursion: OptValue<bool>,
    #[cfg(feature = "mirth-full")]
    #[serde(default)]
    pub ignore_dot: OptValue<bool>,
    #[cfg(feature = "mirth-full")]
    #[serde(default)]
    pub anonymous: OptValue<bool>,
    #[cfg(feature = "mirth-full")]
    #[serde(default)]
    pub username: OptValue<String>,
    #[cfg(feature = "mirth-full")]
    #[serde(default)]
    pub password: OptValue<String>,
    #[cfg(feature = "mirth-full")]
    #[serde(default)]
    pub timeout: OptValue<u64>,
    #[cfg(feature = "mirth-full")]
    #[serde(default)]
    pub secure: OptValue<bool>,
    #[cfg(feature = "mirth-full")]
    #[serde(default)]
    pub passive: OptValue<bool>,
    #[cfg(feature = "mirth-full")]
    #[serde(default)]
    pub validate_connection: OptValue<bool>,
    #[cfg(feature = "mirth-full")]
    #[serde(default)]
    pub after_processing_action: OptValue<String>, // TODO: make enum
    #[cfg(feature = "mirth-full")]
    #[serde(default)]
    pub move_to_directory: OptValue<String>,
    #[cfg(feature = "mirth-full")]
    #[serde(default)]
    pub move_to_file_name: OptValue<String>,
    #[cfg(feature = "mirth-full")]
    #[serde(default)]
    pub error_reading_action: OptValue<String>, // TODO: make enum
    #[cfg(feature = "mirth-full")]
    #[serde(default)]
    pub error_response_action: OptValue<String>, // TODO: make enum
    #[cfg(feature = "mirth-full")]
    #[serde(default)]
    pub error_move_to_directory: OptValue<String>,
    #[cfg(feature = "mirth-full")]
    #[serde(default)]
    pub error_move_to_file_name: OptValue<String>,
    #[cfg(feature = "mirth-full")]
    #[serde(default)]
    pub check_file_age: OptValue<bool>,
    #[cfg(feature = "mirth-full")]
    #[serde(default)]
    pub file_age: OptValue<u64>,
    #[cfg(feature = "mirth-full")]
    #[serde(default)]
    pub file_size_minimum: OptValue<u64>,
    #[cfg(feature = "mirth-full")]
    #[serde(default)]
    pub file_size_maximum: OptValue<u64>,
    #[cfg(feature = "mirth-full")]
    #[serde(default)]
    pub ignore_file_size_maximum: OptValue<bool>,
    #[cfg(feature = "mirth-full")]
    #[serde(default)]
    pub sort_by: OptValue<String>, // TODO: make enum,
    #[cfg(feature = "mirth-full")]
    #[serde(default)]
    pub binary: OptValue<bool>,
    #[cfg(feature = "mirth-full")]
    #[serde(default)]
    pub charset_encoding: OptValue<String>,
}

#[cfg(feature = "mirth-full")]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginProperties {}

#[cfg(feature = "mirth-full")]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PollConnectorProperties {
    pub polling_type: Value<String>, // TODO: make enum
    pub poll_on_start: Value<bool>,
    pub polling_frequency: Value<u64>,
    pub polling_hour: Value<u8>,
    pub polling_minute: Value<u8>,
    pub cron_jobs: CronJobs,
    pub poll_connector_properties_advanced: PollConnectorPropertiesAdvanced,
}

#[cfg(feature = "mirth-full")]
#[derive(Debug, Serialize, Deserialize)]
pub struct CronJobs {
    #[serde(rename = "cronJob", default)] // TODO: verify
    pub data: Vec<CronJob>,
}

#[cfg(feature = "mirth-full")]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CronJob {
    // TODO: fill in
}

#[cfg(feature = "mirth-full")]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PollConnectorPropertiesAdvanced {
    pub weekly: Value<bool>,
    pub inactive_days: InactiveDays,
    pub day_of_month: Value<u8>,
    pub all_day: Value<bool>,
    pub starting_hour: Value<u8>,
    pub starting_minute: Value<u8>,
    pub ending_hour: Value<u8>,
    pub ending_minute: Value<u8>,
}

#[cfg(feature = "mirth-full")]
#[derive(Debug, Serialize, Deserialize)]
pub struct InactiveDays {
    #[serde(rename = "boolean")]
    pub data: Vec<Value<bool>>,
}

#[cfg(feature = "mirth-full")]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceConnectorProperties {
    pub response_variable: Value<String>,
    pub respond_after_processing: Value<bool>,
    pub process_batch: Value<bool>,
    pub first_response: Value<bool>,
    pub processing_threads: Value<u32>,
    pub resource_ids: ResourceIds,
    pub queue_buffer_size: Value<u64>,
}

#[cfg(feature = "mirth-full")]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DestinationConnectorProperties {
    pub queue_enabled: Value<bool>,
    pub send_first: Value<bool>,
    pub retry_interval_millis: Value<u64>,
    pub regenerate_template: Value<bool>,
    pub retry_count: Value<bool>,
    pub rotate: Value<bool>,
    pub include_filter_transformer: Value<bool>,
    pub thread_count: Value<u64>,
    pub thread_assignment_variable: Value<Option<()>>, // TODO: fill in
    pub vailidate_response: Value<bool>,
    pub resource_ids: ResourceIds,
    pub queue_buffer_size: Value<u64>,
    pub reattach_attachments: Value<bool>,
}

#[cfg(feature = "mirth-full")]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Transformer {
    #[serde(default)]
    pub elements: Option<HashMap<String, Vec<TransformerElement>>>,
    #[serde(default)]
    pub inbound_template: Option<TransformerTemplate>,
    #[serde(default)]
    pub outbound_template: Option<TransformerTemplate>,
    pub inbound_data_type: Value<String>, // TODO: make enum
    pub outbound_data_type: Value<String>,
    pub inbound_properties: TransformerXboundProperties,
    pub outbound_properties: TransformerXboundProperties,
}

#[cfg(feature = "mirth-full")]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransformerElement {
    pub name: Value<String>,
    pub sequence_number: Value<u32>,
    pub enabled: Value<bool>,
    #[serde(default)]
    pub variable: OptValue<String>,
    #[serde(default)]
    pub mapping: OptValue<String>,
    #[serde(default)]
    pub default_value: OptValue<String>,
    // replacements: Replacements // TODO: implement,
    #[serde(default)]
    pub scope: OptValue<String>, // TODO: make enum
}

#[cfg(feature = "mirth-full")]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransformerTemplate {
    pub encoding: String, // TODO: make enum
}

#[cfg(feature = "mirth-full")]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransformerXboundProperties {
    pub class: String,
    #[serde(default)]
    pub serialization_properties: Option<TransformerSerializationProperties>,
    #[serde(default)]
    pub deserialization_properties:
        Option<TransformerDeserializationProperties>,
    #[serde(default)]
    pub batch_properties: Option<TransformerBatchProperties>,
    #[serde(default)]
    pub response_generation_properties:
        Option<TransformerResponseGenerationProperties>,
    #[serde(default)]
    pub response_validation_properties:
        Option<TransformerResponseValidationProperties>,
}

#[cfg(feature = "mirth-full")]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransformerSerializationProperties {
    pub class: String,
    #[serde(default)]
    pub handle_repetitions: OptValue<bool>,
    #[serde(default)]
    pub handle_subcomponents: OptValue<bool>,
    #[serde(default)]
    pub use_strict_parser: OptValue<bool>,
    #[serde(default)]
    pub use_strict_validation: OptValue<bool>,
    #[serde(default)]
    pub strip_namespaces: OptValue<bool>,
    #[serde(default)]
    pub segment_delimiter: OptValue<String>,
    #[serde(default)]
    pub convert_line_breaks: OptValue<bool>,
}

#[cfg(feature = "mirth-full")]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransformerDeserializationProperties {
    pub class: String,
    pub use_strict_parser: Value<bool>,
    pub use_strict_validation: Value<bool>,
    pub segment_delimiter: Value<String>,
}

#[cfg(feature = "mirth-full")]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransformerBatchProperties {
    pub class: String,
    pub split_type: Value<String>,
    pub batch_script: OptValue<String>,
}

#[cfg(feature = "mirth-full")]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransformerResponseGenerationProperties {
    pub class: String,
    pub segment_delimiter: Value<String>,
    #[serde(rename = "successfulACKCode")]
    pub successful_ack_code: Value<String>, // TODO: make enum
    #[serde(rename = "successfulACKMessage")]
    pub successful_ack_message: OptValue<String>,
    #[serde(rename = "errorACKCode")]
    pub error_ack_code: Value<String>, // TODO: make enum
    #[serde(rename = "errorACKMessage")]
    pub error_ack_message: OptValue<String>,
    #[serde(rename = "rejectedACKCode")]
    pub rejected_ack_code: Value<String>, // TODO: make enum
    #[serde(rename = "rejectedACKMessage")]
    pub rejected_ack_message: OptValue<String>,
    #[serde(rename = "msh15ACKAccept")]
    pub msh15_ack_accept: Value<bool>,
    pub date_format: Value<String>,
}

#[cfg(feature = "mirth-full")]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransformerResponseValidationProperties {
    pub class: String,
    #[serde(rename = "successfulACKCode")]
    pub successful_ack_code: Value<String>, // TODO: make enum
    #[serde(rename = "errorACKCode")]
    pub error_ack_code: Value<String>, // TODO: make enum
    #[serde(rename = "rejectedACKCode")]
    pub rejected_ack_code: Value<String>, // TODO: make enum
    pub validate_message_control_id: Value<bool>,
    pub original_message_control_id: Value<String>, // TODO: make enum
                                                    // pub original_id_map_variable: HashMap<String, Value<String>> // TODO: verify
}

#[cfg(feature = "mirth-full")]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Filter {
    // TODO: implement
}

#[cfg(feature = "mirth-full")]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelProperties {
    pub clear_global_channel_map: Value<bool>,
    pub message_storage_mode: Value<String>, // TODO: make enum
    pub encrypt_data: Value<bool>,
    pub remove_content_on_completion: Value<bool>,
    pub remove_only_filtered_on_completion: Value<bool>,
    pub remove_attachments_on_completion: Value<bool>,
    pub initial_state: Value<String>, // TODO: make enum
    pub store_attachments: Value<bool>,
    pub meta_data_columns: MetaDataColumns,
    pub attachment_properties: AttachtmentProperties,
    pub resource_ids: ResourceIds,
}

#[cfg(feature = "mirth-full")]
#[derive(Debug, Serialize, Deserialize)]
pub struct MetaDataColumns {
    #[serde(default, rename = "metaDataColumn")]
    pub data: Vec<MetaDataColumn>,
}

#[cfg(feature = "mirth-full")]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetaDataColumn {
    pub name: Value<String>,
    pub r#type: Value<String>, // TODO: make enum
    pub mapping_name: Value<String>,
}

#[cfg(feature = "mirth-full")]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachtmentProperties {
    pub r#type: Value<String>,
    pub properties: Vec<AttachmentProperty>,
}

#[cfg(feature = "mirth-full")]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
// TODO: fill in
pub struct AttachmentProperty {}

#[cfg(feature = "mirth-full")]
#[derive(Debug, Serialize, Deserialize)]
pub struct ResourceIds {
    #[serde(rename = "entry")]
    pub entries: Vec<HashMap<String, Vec<Value<String>>>>,
}

#[cfg(feature = "mirth-full")]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportData {
    pub metadata: ExportMetaData,
}

#[cfg(feature = "mirth-full")]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportMetaData {
    pub enabled: Value<bool>,
    pub last_modified: Timestamp,
    pub pruning_settings: PruningSettings,
}

#[cfg(feature = "mirth-full")]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PruningSettings {
    pub prune_meta_data_days: Value<u64>,
    pub archive_enabled: Value<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ConnectorType {
    Source,
    Destination,
}

impl ConnectorType {
    pub fn to_smartm_value(&self, field: &FieldSpec) -> Data {
        match &field.values {
            Some(ValueTypes::String(smap)) => {
                EnumValue::new(smap.clone(), self.to_string())
                    .map(value::Value::Enum)
            }
            _ => Err(DataError::External(
                "Expected an string enum field for this parameter".to_string(),
            )),
        }
    }
}

impl Display for ConnectorType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Source => "source",
                Self::Destination => "destination",
            }
        )
    }
}
