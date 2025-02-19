/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use uuid::Uuid;

#[derive(Deserialize)]
struct ServiceResponse {
    pub value: Vec<JsonValue>,
}

#[derive(Deserialize)]
struct MappedServiceResponse {
    pub value: Vec<HashMap<String, JsonValue>>,
}

#[derive(Deserialize)]
struct GroupsResponse {
    pub value: Vec<Group>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Group {
    pub id: String,
    pub resource_provisioning_options: Vec<String>,
    pub display_name: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all(deserialize = "camelCase", serialize = "snake_case"))]
pub struct ResourceResponse<T> {
    #[serde(rename = "@odata.context", default)]
    context: Option<String>,
    pub value: Vec<T>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all(deserialize = "camelCase", serialize = "snake_case"))]
pub struct LicenseSku {
    pub account_id: Uuid,
    pub account_name: String,
    pub applies_to: SkuTarget,
    pub capability_status: SkuCompatibility,
    pub consumed_units: i32,
    #[serde(with = "serde_skuid")]
    pub id: (Uuid, Uuid),
    pub sku_id: Uuid,
    pub sku_part_number: String,
    pub subscription_ids: Vec<String>,
    pub prepaid_units: SkuPrepaidUnits,
    // pub service_plans: Vec<ServicePlan>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all(deserialize = "camelCase", serialize = "snake_case"))]
pub struct SkuPrepaidUnits {
    enabled: u32,
    suspended: u32,
    warning: u32,
    locked_out: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServicePlan {
    service_plan_id: Uuid,
    service_plan_name: String,
    provisioning_status: ServicePlanProvisioningStatus,
    applies_to: SkuTarget,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServicePlanProvisioningStatus {
    Success,
    Disabled,
    Error,
    PendingInput,
    PendingActivation,
    PendingProvisioning,
}

mod serde_skuid {
    use serde::{de::Visitor, Deserializer, Serializer};
    use uuid::Uuid;

    struct SkuIdVisitor;

    impl<'de> Visitor<'de> for SkuIdVisitor {
        type Value = (Uuid, Uuid);

        fn expecting(
            &self,
            formatter: &mut std::fmt::Formatter,
        ) -> std::fmt::Result {
            formatter.write_str("2 uuids seperated with an underscore")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            let (left, right) = v
                .split_once('_')
                .ok_or(E::custom(format!("Missing an underscore in {v}")))?;

            let left = Uuid::parse_str(left).map_err(|e| {
                E::custom(format!("left value is not an uuid: {e}"))
            })?;
            let right = Uuid::parse_str(right).map_err(|e| {
                E::custom(format!("left value is not an uuid: {e}"))
            })?;

            Ok((left, right))
        }
    }

    pub fn serialize<S>(
        value: &(Uuid, Uuid),
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let id = format!("{}_{}", value.0, value.1);
        serializer.serialize_str(&id)
    }

    pub fn deserialize<'de, D>(
        deserializer: D,
    ) -> Result<(Uuid, Uuid), D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(SkuIdVisitor)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SkuTarget {
    User,
    Company,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SkuCompatibility {
    Enabled,
    Warning,
    Suspended,
    Deleted,
    LockedOut,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all(deserialize = "camelCase", serialize = "snake_case"))]
pub struct Organization {
    id: String,
    deleted_date_time: Option<DateTime<Utc>>,
    business_phones: Vec<String>,
    city: Option<String>,
    country: Option<String>,
    country_letter_code: Option<String>,
    created_date_time: Option<DateTime<Utc>>,
    display_name: String,
    is_multiple_data_locations_for_services_enabled: Option<bool>,
    marketing_notification_emails: Vec<String>,
    on_premise_sync_enabled: Option<bool>,
    postal_code: Option<String>,
    preferred_language: Option<String>,
    security_compliance_notification_mails: Vec<String>,
    security_compliance_notification_phones: Vec<String>,
    state: Option<String>,
    street: Option<String>,
    technical_notification_phones: Option<Vec<String>>,
    tenant_type: String,
    directory_size_quota: DirectoryQuota,
    assigned_plans: Vec<AssignedPlan>,
    privacy_profile: Option<PrivacyProfile>,
    provisioned_plans: Vec<ProvisionedPlan>,
    verified_domains: Vec<VerifiedDomain>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DirectoryQuota {
    used: u64,
    total: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all(deserialize = "camelCase", serialize = "snake_case"))]
pub struct AssignedPlan {
    assinged_date_time: Option<DateTime<Utc>>,
    capability_status: CapabilityStatus,
    service: String,
    service_plan_id: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum CapabilityStatus {
    Enabled,
    Warning,
    Suspended,
    Deleted,
    LockedOut,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all(deserialize = "camelCase", serialize = "snake_case"))]
pub struct PrivacyProfile {
    contact_email: String,
    statement_url: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all(deserialize = "camelCase", serialize = "snake_case"))]
pub struct ProvisionedPlan {
    capability_status: String,
    provisioning_status: String,
    service: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all(deserialize = "camelCase", serialize = "snake_case"))]
pub struct VerifiedDomain {
    capabilities: String,
    is_default: bool,
    is_initial: bool,
    name: String,
    #[serde(rename = "type")]
    typ: String,
}
