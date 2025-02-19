/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::soap::{SoapClient, SoapError, Value};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SysteminfoRequest {
    #[serde(rename = "Body")]
    pub(crate) body: Body,
}

impl SysteminfoRequest {
    pub async fn new(
        client: &SoapClient,
        _args: &HashMap<String, String>,
    ) -> Result<SysteminfoRequest, SoapError> {
        let body = r#"<SOAP-ENV:Body xmlns:ns1="urn:vim25">
							<ns1:RetrieveServiceContent xsi:type="ns1:RetrieveServiceContentRequestType">
								<ns1:_this type="ServiceInstance">ServiceInstance</ns1:_this>
							</ns1:RetrieveServiceContent>
						</SOAP-ENV:Body>"#
            .to_string();
        let request: SysteminfoRequest =
            serde_xml_rs::from_str(&client.request(body).await?)?;
        Ok(request)
    }
}

impl SysteminfoRequest {
    pub fn to_hashmap(&self) -> HashMap<String, String> {
        let return_val = &self.body.response.returnval;
        let mut attrs: HashMap<String, String> = HashMap::new();
        attrs.insert(
            "root_folder".to_string(),
            return_val.root_folder.data.clone(),
        );
        attrs.insert(
            "property_collector".to_string(),
            return_val.property_collector.data.clone(),
        );
        attrs.insert(
            "view_manager".to_string(),
            return_val.view_manager.data.clone(),
        );
        attrs.insert("setting".to_string(), return_val.setting.data.clone());
        attrs.insert(
            "user_directory".to_string(),
            return_val.root_folder.data.clone(),
        );
        attrs.insert(
            "session_manager".to_string(),
            return_val.session_manager.data.clone(),
        );
        attrs.insert(
            "authorization_manager".to_string(),
            return_val.authorization_manager.data.clone(),
        );
        attrs.insert(
            "perf_manager".to_string(),
            return_val.perf_manager.data.clone(),
        );
        attrs.insert(
            "event_manager".to_string(),
            return_val.event_manager.data.clone(),
        );
        attrs.insert(
            "task_manager".to_string(),
            return_val.task_manager.data.clone(),
        );
        attrs.insert(
            "diagnostic_manager".to_string(),
            return_val.diagnostic_manager.data.clone(),
        );
        attrs.insert(
            "license_manager".to_string(),
            return_val.license_manager.data.clone(),
        );
        attrs.insert(
            "search_index".to_string(),
            return_val.search_index.data.clone(),
        );
        attrs.insert(
            "file_manager".to_string(),
            return_val.file_manager.data.clone(),
        );
        attrs.insert(
            "virtual_disk_manager".to_string(),
            return_val.virtual_disk_manager.data.clone(),
        );
        attrs.insert(
            "ovf_manager".to_string(),
            return_val.ovf_manager.data.clone(),
        );
        attrs.insert(
            "dv_switch_manager".to_string(),
            return_val.dv_switch_manager.data.clone(),
        );
        attrs.insert(
            "localization_manager".to_string(),
            return_val.localization_manager.data.clone(),
        );
        attrs.insert(
            "storage_resource_manager".to_string(),
            return_val.storage_resource_manager.data.clone(),
        );
        attrs.insert(
            "guest_operations_manager".to_string(),
            return_val.guest_operations_manager.data.clone(),
        );
        for (k, v) in return_val.about.to_hashmap() {
            attrs.insert(k, v);
        }
        attrs
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub(crate) struct Body {
    #[serde(rename = "RetrieveServiceContentResponse")]
    pub(crate) response: Response,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub(crate) struct Response {
    pub(crate) returnval: ReturnValue,
}

// TODO: Use Value<String>
#[derive(Serialize, Deserialize, Debug, Clone)]
pub(crate) struct ReturnValue {
    #[serde(rename = "rootFolder")]
    pub(crate) root_folder: Value<String>,
    #[serde(rename = "propertyCollector")]
    pub(crate) property_collector: Value<String>,
    #[serde(rename = "viewManager")]
    pub(crate) view_manager: Value<String>,
    pub(crate) about: About,
    pub(crate) setting: Value<String>,
    #[serde(rename = "userDirectory")]
    pub(crate) user_directory: Value<String>,
    #[serde(rename = "sessionManager")]
    pub(crate) session_manager: Value<String>,
    #[serde(rename = "authorizationManager")]
    pub(crate) authorization_manager: Value<String>,
    #[serde(rename = "perfManager")]
    pub(crate) perf_manager: Value<String>,
    #[serde(rename = "eventManager")]
    pub(crate) event_manager: Value<String>,
    #[serde(rename = "taskManager")]
    pub(crate) task_manager: Value<String>,
    #[serde(rename = "accountManager")]
    pub(crate) account_manager: Option<Value<String>>,
    #[serde(rename = "diagnosticManager")]
    pub(crate) diagnostic_manager: Value<String>,
    #[serde(rename = "licenseManager")]
    pub(crate) license_manager: Value<String>,
    #[serde(rename = "searchIndex")]
    pub(crate) search_index: Value<String>,
    #[serde(rename = "fileManager")]
    pub(crate) file_manager: Value<String>,
    #[serde(rename = "virtualDiskManager")]
    pub(crate) virtual_disk_manager: Value<String>,
    #[serde(rename = "ovfManager")]
    pub(crate) ovf_manager: Value<String>,
    #[serde(rename = "dvSwitchManager")]
    pub(crate) dv_switch_manager: Value<String>,
    #[serde(rename = "localizationManager")]
    pub(crate) localization_manager: Value<String>,
    #[serde(rename = "storageResourceManager")]
    pub(crate) storage_resource_manager: Value<String>,
    #[serde(rename = "guestOperationsManager")]
    pub(crate) guest_operations_manager: Value<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub(crate) struct About {
    pub(crate) name: Value<String>,
    #[serde(rename = "fullName")]
    pub(crate) fullname: Value<String>,
    pub(crate) vendor: Value<String>,
    pub(crate) version: Value<String>,
    pub(crate) build: Value<i64>,
    #[serde(rename = "localeVersion")]
    pub(crate) locale_version: Value<String>,
    #[serde(rename = "localeBuild")]
    pub(crate) locale_build: Value<String>,
    #[serde(rename = "osType")]
    pub(crate) os_type: Value<String>,
    #[serde(rename = "productLineId")]
    pub(crate) product_line_id: Value<String>,
    #[serde(rename = "apiType")]
    pub(crate) api_type: Value<String>,
    #[serde(rename = "apiVersion")]
    pub(crate) api_version: Value<String>,
    #[serde(rename = "licenseProductName")]
    pub(crate) license_product_name: Value<String>,
    #[serde(rename = "licenseProductVersion")]
    pub(crate) license_product_version: Value<String>,
}

impl About {
    fn to_hashmap(&self) -> HashMap<String, String> {
        let mut attrs: HashMap<String, String> = HashMap::new();
        attrs.insert("name".to_string(), self.name.data.clone());
        attrs.insert("fullname".to_string(), self.fullname.data.clone());
        attrs.insert("vendor".to_string(), self.vendor.data.clone());
        attrs.insert("version".to_string(), self.version.data.clone());
        attrs.insert("build".to_string(), self.build.data.clone().to_string());
        attrs.insert(
            "locale_version".to_string(),
            self.locale_version.data.clone(),
        );
        attrs
            .insert("locale_build".to_string(), self.locale_build.data.clone());
        attrs.insert("os_type".to_string(), self.os_type.data.clone());
        attrs.insert(
            "product_line_id".to_string(),
            self.product_line_id.data.clone(),
        );
        attrs.insert("api_type".to_string(), self.api_type.data.clone());
        attrs.insert("api_version".to_string(), self.api_version.data.clone());
        attrs.insert(
            "license_product_name".to_string(),
            self.license_product_name.data.clone(),
        );
        attrs.insert(
            "license_product_version".to_string(),
            self.license_product_version.data.clone(),
        );
        attrs
    }
}
