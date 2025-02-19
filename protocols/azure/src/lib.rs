/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

pub mod config;
pub mod definitions;
pub mod error;
pub mod input;
pub mod plugin;
pub mod requests;
pub mod schema;

pub use config::{ClientInfo, Config};
pub use definitions::{
    Resource, ResourceGroup, ResourceGroupId, ResourceGroupName, ResourceId,
    ResourceResponse, Subscription, SubscriptionId, Tenant, TenantId,
};
pub use error::{AzureError, Result};
pub use input::Input;
pub use plugin::Plugin;
pub use requests::{request_resource, request_resource_from_subscription};
