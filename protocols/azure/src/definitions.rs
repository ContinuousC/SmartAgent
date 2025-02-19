/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub type SubscriptionId = String;
pub type TenantId = String;
pub type ResourceGroupId = String;
pub type ResourceGroupName = String;
pub type ResourceId = String;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(
    untagged,
    rename_all(deserialize = "camelCase", serialize = "snake_case")
)]
pub enum ResourceResponse<T> {
    Error(ErrorResponse),
    Success(SuccessResponse<T>),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ErrorResponse {
    pub error: AzureError,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AzureError {
    code: String,
    pub message: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all(deserialize = "camelCase", serialize = "snake_case"))]
pub struct SuccessResponse<T> {
    pub next_link: Option<String>,
    count: Option<ResponseAggregation>,
    pub value: Vec<T>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all(deserialize = "camelCase", serialize = "snake_case"))]
pub struct ResponseAggregation {
    #[serde(rename = "type")]
    typ: String,
    value: i64,
}

// TENANT
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all(deserialize = "camelCase", serialize = "snake_case"))]
pub struct Tenant {
    country: Option<String>,
    country_code: Option<String>,
    default_domain: Option<String>,
    display_name: Option<String>,
    domains: Option<Vec<String>>,
    id: String,
    tenant_branding_logo_url: Option<String>,
    tenant_category: TenantCategory,
    tenant_id: TenantId,
    tenant_type: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all(deserialize = "PascalCase", serialize = "snake_case"))]
pub enum TenantCategory {
    Home,
    ManagedBy,
    ProjectedBy,
}

// SUBSCRIPTION
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all(deserialize = "camelCase", serialize = "snake_case"))]
pub struct Subscription {
    id: String,
    subscription_id: SubscriptionId,
    tenant_id: TenantId,
    display_name: String,
    state: String,
    subscription_policies: SubscriptionPolicies,
    autorization_source: Option<String>,
    managed_by_tenants: Vec<ManagedTenant>,
    tags: Option<HashMap<String, String>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all(deserialize = "camelCase", serialize = "snake_case"))]
pub struct SubscriptionPolicies {
    location_placement_id: String,
    quota_id: String,
    spending_limit: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all(deserialize = "camelCase", serialize = "snake_case"))]
pub struct ManagedTenant {
    tenant_id: TenantId,
}

// RESOURCEGROUP
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all(deserialize = "camelCase", serialize = "snake_case"))]
pub struct ResourceGroup {
    id: ResourceGroupId,
    location: String,
    managed_by: Option<String>,
    name: ResourceGroupName,
    properties: ResourceGroupProperty,
    #[serde(default)]
    tags: HashMap<String, String>,
    #[serde(rename = "type")]
    typ: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all(deserialize = "camelCase", serialize = "snake_case"))]
pub struct ResourceGroupProperty {
    provisioning_state: String,
}

// RESOURCE
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all(deserialize = "camelCase", serialize = "snake_case"))]
pub struct Resource {
    changed_time: Option<DateTime<Utc>>,
    created_time: Option<DateTime<Utc>>,
    extended_location: Option<ExtendedLocation>,
    id: ResourceId,
    identity: Option<Identity>,
    kind: Option<String>,
    location: String,
    managed_by: Option<String>,
    name: Option<String>,
    plan: Option<Plan>,
    properties: Option<HashMap<String, String>>,
    provisioning_state: Option<String>,
    sku: Option<Sku>,
    #[serde(default)]
    tags: HashMap<String, String>,
    r#type: String,
    system_data: Option<SystemData>,
}

impl Resource {
    pub fn get_resource_group(&self) -> String {
        let needle = "/resourceGroups/";
        match self.id.find(needle) {
            Some(i) => self.id[i + needle.len()..].find('/').map(|j| {
                String::from(&self.id[i + needle.len()..i + needle.len() + j])
            }),
            None => None,
        }
        .unwrap_or_default()
    }
}

impl PartialEq for Resource {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl Eq for Resource {}

impl Hash for Resource {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all(deserialize = "camelCase", serialize = "snake_case"))]
pub struct ExtendedLocation {
    name: Option<String>,
    r#type: Option<ExtendedLocationType>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all(deserialize = "camelCase", serialize = "snake_case"))]
pub enum ExtendedLocationType {
    EdgeZone,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all(deserialize = "camelCase", serialize = "snake_case"))]
pub struct Identity {
    principal_id: Option<String>,
    tenant_id: Option<TenantId>,
    r#type: Option<ResourceIdentityType>,
    user_assigned_identities: Option<HashMap<String, String>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all(deserialize = "camelCase", serialize = "snake_case"))]
pub enum ResourceIdentityType {
    None,
    SystemAssigned,
    UserAssigned,
    SystemAssignedUserAssigned,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all(deserialize = "camelCase", serialize = "snake_case"))]
pub struct Plan {
    name: Option<String>,
    product: Option<String>,
    promotion_code: Option<String>,
    publisher: Option<String>,
    version: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all(deserialize = "camelCase", serialize = "snake_case"))]
pub struct Sku {
    capacity: Option<i32>,
    family: Option<String>,
    model: Option<String>,
    name: String,
    size: Option<String>,
    tier: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all(deserialize = "camelCase", serialize = "snake_case"))]
pub struct SystemData {
    created_by: String,
    created_by_type: String,
    created_at: DateTime<Utc>,
    last_modified_by: String,
    last_modified_by_type: String,
    last_modified_at: DateTime<Utc>,
}
