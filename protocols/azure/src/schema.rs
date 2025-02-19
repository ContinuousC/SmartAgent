/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use chrono::{DateTime, Utc};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashSet;

use crate::error::AzureError;
/* Metrics */
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Metrics {
    pub cost: Option<u32>,
    pub timespan: String,
    pub interval: Option<Interval>,
    pub namespace: Option<String>,
    pub resourceregion: Option<String>,
    pub value: Vec<Metric>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Metric {
    pub id: String,
    #[serde(rename = "type")]
    pub typ: String,
    pub name: LocalizableString,
    #[serde(rename = "displayDescription")]
    pub display_description: Option<String>,
    pub unit: Unit,
    pub timeseries: Vec<TimeSeriesElement>,
    #[serde(rename = "errorMessage")]
    pub error_message: Option<String>,
    #[serde(rename = "errorCode")]
    pub error_code: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TimeSeriesElement {
    pub metadatavalues: Option<Vec<MetaDataValue>>,
    pub data: Vec<MetricValue>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MetaDataValue {
    pub name: LocalizableString,
    pub value: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MetricValue {
    #[serde(rename = "timeStamp")]
    pub timestamp: DateTime<Utc>,
    pub average: Option<f64>,
    pub minimum: Option<f64>,
    pub maximum: Option<f64>,
    pub total: Option<f64>,
    pub count: Option<f64>,
}

impl MetricValue {
    pub fn has_data(&self) -> bool {
        // self.count.is_some() ||
        self.average.is_some()
            || self.minimum.is_some()
            || self.maximum.is_some()
            || self.total.is_some()
        // self.count.map_or(true, |v| v > 0.0) && (self.average.is_some() || self.minimum.is_some() || self.maximum.is_some() || self.total.is_some())
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub enum Interval {
    PT1M,
    PT5M,
    PT15M,
    PT30M,
    PT1H,
    PT6,
    PT12H,
    PT1D,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub enum Unit {
    Count,
    Bytes,
    Seconds,
    CountPerSecond,
    BytesPerSecond,
    Percent,
    MilliSeconds,
    ByteSeconds,
    Unspecified,
    Cores,
    MilliCores,
    NanoCores,
    BitsPerSecond,
}

/* Common */
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LocalizableString {
    pub value: String,
    #[serde(rename = "localizedValue")]
    pub localized_value: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum Response<T> {
    Ok(T),
    Err(Error),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Error {
    pub code: String,
    pub message: String,
}

impl Error {
    pub fn get_error_metrics(
        &self,
        old_metrics: &HashSet<String>,
    ) -> Result<HashSet<String>, AzureError> {
        let mut result: HashSet<String> = HashSet::new();
        if self.message.contains("Valid metrics: ") {
            let new_metrics: Vec<String> = self
                .message
                .split("Valid metrics: ")
                .map(String::from)
                .collect::<Vec<String>>()
                .pop()
                .ok_or(AzureError::ResponseError(String::from(
                    "No valid metrics",
                )))?
                .split(',')
                .map(String::from)
                .collect();
            for metric in new_metrics {
                if old_metrics.contains(&metric) {
                    result.insert(metric);
                }
            }
            Ok(result)
        } else {
            Err(AzureError::ResponseError(String::from("No valid metrics")))
        }
    }
}

lazy_static! {
    pub static ref METRICS: Value = json!(
    {
        "title": "Metrics - List",
          "description": "Lists the metric values for a resource",
          "type": "object",
        "properties": {
            "cost": { "type": "number", "minimum": 0 },
            "interval": {
                "type": "string",
                "enum": [ "PT1M", "PT5M", "PT15M", "PT30M", "PT1H", "PT6H", "PT12H", "PT1D" ]
            },
            "namespace": { "type": "string" },
            "resourceregion": { "type": "string" },
            "timespan": { "type": "string" },
            "value": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "string" },
                        "name": {
                            "type": "object",
                            "properties": {
                                "localizedValue": { "type": "string" },
                                "value": { "type": "string" }
                            },
                            "required": [ "localizedValue", "value" ]
                        },
                        "timeseries": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "data": {
                                        "type": "array",
                                        "items": {
                                            "type": "object",
                                            "properties": {
                                                "average": { "type": "number" },
                                                "count": { "type": "number", "minimum": 0 },
                                                "maximum": { "type": "number" },
                                                "minimum": { "type": "number" },
                                                "timeStamp": { "type": "string", "format": "date-time" },
                                                "total": { "type": "number" }
                                            },
                                            "required": [ "timeStamp" ]
                                        },
                                        "uniqueItems": true,
                                    },
                                    "metadatavalues": {
                                        "type": "array",
                                        "items": {
                                            "type": "object",
                                            "properties": {
                                                "name": {
                                                    "type": "object",
                                                    "properties": {
                                                        "localizedValue": { "type": "string" },
                                                        "value": { "type": "string" }
                                                    },
                                                    "required": [ "localizedValue", "value" ]
                                                },
                                                "value": { "type": "string" }
                                            }
                                        },
                                        "uniqueItems": true
                                    }
                                },
                                "required": [ "data", "metadatavalues" ]
                            },
                            "uniqueItems": true,
                        },
                        "type": { "type": "string" },
                        "unit": {
                            "type": "string",
                            "enum": [ "BitsPerSecond", "ByteSeconds", "Bytes", "BytesPerSecond", "Cores", "Count", "CountPerSecond", "MilliCores", "MilliSeconds", "NanoCores", "Percent", "Seconds", "Unspecified" ]
                        }
                    },
                    "required": [ "id", "name", "timeseries", "type", "unit" ]
                },
                "uniqueItems": true
            }
        },
        "required": [ "cost", "interval", "namespace", "resourceregion", "timespan", "value" ]
    });
    pub static ref RESOURCES: Value = json!(
    {
        "title": "Resources - List",
          "description": "Lists the resources for a subscription",
          "type": "object",
        "properties": {
            "nextLink": { "type": "string", "format": "uri"},
            "value": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "changedTime": { "type": "string", "format": "date-time" },
                        "createdTime": { "type": "string", "format": "date-time" },
                        "id": { "type": "string" },
                        "identity": {
                            "type": "object",
                            "properties": {
                                "principalId": { "type": "string" },
                                "tenantId": { "type": "string" },
                                "type": {
                                    "type": "string",
                                    "enum": [ "None", "SystemAssigned", "SystemAssigned", "UserAssigned" ]
                                },
                                "userAssignedIdentities": {
                                    "type": "object",
                                    "propertyNames": {
                                        "^\\/subscriptions\\/[a-z0-9\\-]*\\/resourcegroups\\/[a-z0-9\\-\\.]*\\/providers\\/[a-z\\.].*\\/[a-z].*\\/[a-z0-9\\-\\.]*": {
                                            "type": "object",
                                            "properties": {
                                                "principalId": { "type": "string" },
                                                "clientId": { "type": "string" }
                                            }
                                        }
                                    },
                                }
                            },
                            "required": [ "type" ]
                        },
                        "kind": { "type": "string" },
                        "location": { "type": "string" },
                        "managedBy": { "type": "string" },
                        "name": { "type": "string" },
                        "plan": {
                            "type": "object",
                            "properties": {
                                "name": { "type": "string" },
                                "product": { "type": "string" },
                                "promotionCode": { "type": "string" },
                                "publisher": { "type": "string" },
                                "version": { "type": "string" }
                            },
                            "required": [ "name", "product", "publisher" ]
                        },
                        "properties": { "type": "object" },
                        "provisioningState": { "type": "string" },
                        "sku": {
                            "type": "object",
                            "properties": {
                                "capacity": { "type": "number", "minimum": 0 },
                                "family": { "type": "string" },
                                "model": { "type": "string" },
                                "name": { "type": "string" },
                                "size": { "type": "string" },
                                "tier": { "type": "string" }
                            },
                            "required": [ "name" ]
                        },
                        "tags": { "type": "object" },
                        "type": { "type": "string" }
                    },
                    "required": [ "id", "location", "name", "type"]
                },
                "additionalItems": false,
                "uniqueItems": true
            }
        },
        "required": [ "value" ]
    });
}
