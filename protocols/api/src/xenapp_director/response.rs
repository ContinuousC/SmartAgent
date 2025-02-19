/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::{collections::BTreeMap, net::IpAddr, sync::Arc};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use value::{Data, DataError};

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Value<T> {
    #[serde(rename = "$value")]
    pub inner: T,
}

impl<T> AsRef<T> for Value<T> {
    fn as_ref(&self) -> &T {
        &self.inner
    }
}

impl Value<String> {
    pub fn to_value(&self) -> Data {
        Ok(value::Value::UnicodeString(self.inner.clone()))
    }
}

impl Value<i64> {
    pub fn to_value(&self) -> Data {
        Ok(value::Value::Integer(self.inner))
    }
    pub fn to_intenum(&self, choices: Arc<BTreeMap<i64, String>>) -> Data {
        value::IntEnumValue::new(choices, self.inner).map(value::Value::IntEnum)
    }
}

impl Value<bool> {
    pub fn to_value(&self) -> Data {
        Ok(value::Value::Boolean(self.inner))
    }
}

impl Value<Uuid> {
    pub fn to_value(&self) -> Data {
        Ok(value::Value::UnicodeString(self.inner.to_string()))
    }
}

impl Value<IpAddr> {
    pub fn to_value(&self) -> Data {
        Ok(match self.inner {
            IpAddr::V4(ip) => value::Value::Ipv4Address(ip.octets()),
            IpAddr::V6(ip) => value::Value::Ipv6Address(ip.segments()),
        })
    }
}

impl Value<DateTime> {
    pub fn to_value(&self) -> Data {
        Ok(value::Value::Time(self.inner.0))
    }
}

#[derive(Debug, Default, Clone, Copy, Eq, PartialEq, Serialize)]
pub struct DateTime(chrono::DateTime<Utc>);

type OptValue<T> = Value<Option<T>>;
impl OptValue<DateTime> {
    pub fn to_value(&self) -> Data {
        self.inner
            .map(|dt| value::Value::Time(dt.0))
            .ok_or(DataError::Missing)
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Property<T> {
    #[serde(rename = "@null", default)]
    null: bool,
    #[serde(rename = "@type")]
    r#type: String,
    #[serde(rename = "$value", default)]
    value: Option<T>,
}

impl<T> AsRef<Option<T>> for Property<T> {
    fn as_ref(&self) -> &Option<T> {
        &self.value
    }
}

impl Property<i64> {
    pub fn to_value(&self) -> Data {
        self.value
            .ok_or(DataError::Missing)
            .map(value::Value::Integer)
    }
}

impl Property<Uuid> {
    pub fn to_value(&self) -> Data {
        self.value
            .ok_or(DataError::Missing)
            .map(String::from)
            .map(value::Value::UnicodeString)
    }
}

impl Property<DateTime> {
    pub fn to_value(&self) -> Data {
        self.value
            .ok_or(DataError::Missing)
            .map(|dt| value::Value::Time(dt.0))
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct CitrixMonitorData {
    pub id: Value<String>,
    pub title: Value<String>,
    pub updated: Value<DateTime>,
    pub entry: CitrixMonitorMachine,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct CitrixMonitorMachine {
    pub id: Value<String>,
    pub link: Vec<Link>,
    pub updated: Value<DateTime>,
    pub content: CitrixMonitorMachineContent,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Link {
    #[serde(rename = "@title")]
    title: String,
    pub inline: Option<ExpandedCurrentLoadIndex>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct ExpandedCurrentLoadIndex {
    pub entry: CurrentLoadIndex,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct CurrentLoadIndex {
    pub content: CurrentLoadIndexContent,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct CurrentLoadIndexContent {
    pub properties: CurrentLoadIndexProperties,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct CurrentLoadIndexProperties {
    pub id: Property<i64>,
    pub effective_load_index: Property<i64>,
    pub cpu: Property<i64>,
    pub memory: Property<i64>,
    pub disk: Property<i64>,
    pub network: Property<i64>,
    pub session_count: Property<i64>,
    pub machine_id: Property<Uuid>,
    // failed to deserialize: Custom("premature end of input")
    pub created_date: Property<DateTime>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct CitrixMonitorMachineContent {
    pub properties: CitrixMonitorMachineProperties,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct CitrixMonitorMachineProperties {
    pub id: Value<Uuid>,
    pub sid: Value<String>,
    pub name: Value<String>,
    pub dns_name: Value<String>,
    pub lifecycle_state: Value<i64>,

    #[serde(rename = "IPAddress")]
    pub ip_address: Value<IpAddr>,
    pub hosted_machine_id: Value<Uuid>,
    pub hosting_server_name: Value<String>,
    pub hosted_machine_name: Value<String>,
    pub is_assigned: Value<bool>,
    pub is_in_maintenance_mode: Value<bool>,
    pub is_pending_update: Value<bool>,
    pub agent_version: Value<String>,
    pub associated_user_full_names: Value<String>,
    pub associated_user_names: Value<String>,
    #[serde(rename = "AssociatedUserUPNs")]
    pub associated_user_upns: Value<String>,
    pub current_registration_state: Value<i64>,

    pub last_deregistered_code: Value<i64>,
    pub current_power_state: Value<i64>,
    pub current_session_count: Value<i64>,
    pub controller_dns_name: Value<String>,
    pub functional_level: Value<i64>,
    pub windows_connection_setting: Value<i64>,
    pub is_preparing: Value<bool>,
    pub fault_state: Value<i64>,
    #[serde(rename = "OSType")]
    pub os_type: Value<String>,
    pub current_load_index_id: Value<i64>,
    pub catalog_id: Value<Uuid>,
    pub desktop_group_id: Value<Uuid>,
    pub hypervisor_id: Value<Uuid>,
    pub hash: Value<String>,
    pub machine_role: Value<i64>,

    // failed to deserialize: Custom("premature end of input")
    pub registration_state_change_date: Value<DateTime>,
    pub last_deregistered_date: Value<DateTime>,
    pub powered_on_date: Value<DateTime>,
    pub power_state_change_date: Value<DateTime>,
    pub failure_date: OptValue<DateTime>,
    pub created_date: OptValue<DateTime>,
    pub modified_date: OptValue<DateTime>,
}

/// Citrix returns an iso8601 formatted datetime without the timezone if it is UTC, making it invalid rf3339
/// so we add the Z manually if it is not present
mod datetime_serde {
    use std::fmt;

    use chrono::Utc;
    use serde::{
        de::{self, Visitor},
        Deserialize, Deserializer,
    };

    use super::DateTime;

    impl<'de> Deserialize<'de> for DateTime {
        fn deserialize<D>(deserializer: D) -> Result<DateTime, D::Error>
        where
            D: Deserializer<'de>,
        {
            struct DateTimeVisitor;

            impl<'de> Visitor<'de> for DateTimeVisitor {
                type Value = DateTime;

                fn expecting(
                    &self,
                    formatter: &mut fmt::Formatter,
                ) -> fmt::Result {
                    formatter.write_str("an iso8601 formatted datetime")
                }

                fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
                where
                    E: de::Error,
                {
                    v.chars()
                        .skip(v.find('T').unwrap_or_default())
                        .any(|c| ['Z', '+', '-'].contains(&c))
                        .then(|| chrono::DateTime::parse_from_rfc3339(v))
                        .unwrap_or_else(|| {
                            chrono::DateTime::parse_from_rfc3339(&format!(
                                "{v}Z"
                            ))
                        })
                        .map_err(|e| {
                            E::custom(format!("parsing datetime failed: {e}"))
                        })
                        .map(|dt| dt.with_timezone(&Utc))
                        .map(DateTime)
                }
            }

            deserializer.deserialize_str(DateTimeVisitor)
        }
    }
}
