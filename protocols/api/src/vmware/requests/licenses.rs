/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::collections::HashMap;

use handlebars::Handlebars;
use minidom::Element;

use crate::soap::{SoapClient, SoapError};

#[derive(Debug)]
pub struct LicensesRequest {
    pub licenses: Vec<License>,
}

#[derive(Debug)]
pub struct License {
    pub key: String,
    pub name: String,
    pub total: i64,
    pub used: i64,
}

impl LicensesRequest {
    pub async fn new(
        client: &SoapClient,
        args: &HashMap<String, String>,
    ) -> Result<LicensesRequest, SoapError> {
        let template = r#"<SOAP-ENV:Body xmlns:ns1="urn:vim25">
							<ns1:RetrievePropertiesEx xsi:type="ns1:RetrievePropertiesExRequestType">
								<ns1:_this type="PropertyCollector">{{property_collector}}</ns1:_this>
								<ns1:specSet>
									<ns1:propSet>
										<ns1:type>LicenseManager</ns1:type>
										<all>0</all>
										<ns1:pathSet>licenses</ns1:pathSet>
									</ns1:propSet>
									<ns1:objectSet>
										<ns1:obj type="LicenseManager">{{license_manager}}</ns1:obj>
									</ns1:objectSet>
								</ns1:specSet>
								<ns1:options/>
							</ns1:RetrievePropertiesEx>
						</SOAP-ENV:Body>"#
            .to_string();
        let body = Handlebars::new().render_template(&template, &args)?;
        let response = &client.request(body).await?;
        let root: Element = response.parse()?;
        let ns = "urn:vim25";
        let values = root
            .get_child("Body", "http://schemas.xmlsoap.org/soap/envelope/")
            .ok_or(SoapError::XMLChildNotFound("Body".to_string()))?
            .get_child("RetrievePropertiesExResponse", ns)
            .ok_or(SoapError::XMLChildNotFound(
                "RetrievePropertiesExResponse".to_string(),
            ))?
            .get_child("returnval", ns)
            .ok_or(SoapError::XMLChildNotFound("returnval".to_string()))?
            .get_child("objects", ns)
            .ok_or(SoapError::XMLChildNotFound("objects".to_string()))?
            .get_child("propSet", ns)
            .ok_or(SoapError::XMLChildNotFound("propSet".to_string()))?
            .get_child("val", ns)
            .ok_or(SoapError::XMLChildNotFound("val".to_string()))?;
        let mut licenses: Vec<License> = Vec::new();
        for license_node in values.children() {
            licenses.push(License {
                key: license_node
                    .get_child("licenseKey", ns)
                    .map(|node| node.text())
                    .ok_or(SoapError::XMLChildNotFound(
                        "licenseKey".to_string(),
                    ))?,
                name: license_node
                    .get_child("name", ns)
                    .map(|node| node.text())
                    .ok_or(SoapError::XMLChildNotFound("name".to_string()))?,
                total: license_node
                    .get_child("total", ns)
                    .map(|node| {
                        node.text().parse::<i64>().map_or_else(
                            |_e| {
                                Err(SoapError::XMLParseValue(
                                    "total".to_string(),
                                    node.text(),
                                    String::from("i64"),
                                ))
                            },
                            Ok,
                        )
                    })
                    .ok_or(SoapError::XMLChildNotFound(
                        "total".to_string(),
                    ))??,
                used: license_node
                    .get_child("used", ns)
                    .map(|node| {
                        node.text().parse::<i64>().map_or_else(
                            |_e| {
                                Err(SoapError::XMLParseValue(
                                    "used".to_string(),
                                    node.text(),
                                    String::from("i64"),
                                ))
                            },
                            Ok,
                        )
                    })
                    .ok_or(SoapError::XMLChildNotFound("used".to_string()))??,
            })
        }

        Ok(LicensesRequest { licenses })
    }
}
