/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::collections::HashMap;

use handlebars::Handlebars;
use minidom::Element;

use crate::soap::{SoapClient, SoapError};

#[derive(Debug)]
pub struct PNicRequest {
    pub pnics: Vec<Pnic>,
}

#[derive(Debug)]
pub struct Pnic {
    pub key: String,
    pub device: String,
    pub mac: String,
    pub bandwidth: i64,
    pub state: bool,
}

impl PNicRequest {
    pub async fn new(
        client: &SoapClient,
        args: &HashMap<String, String>,
    ) -> Result<PNicRequest, SoapError> {
        let template = r#"<SOAP-ENV:Body xmlns:ns1="urn:vim25">
							<ns1:RetrievePropertiesEx xsi:type="ns1:RetrievePropertiesExRequestType">
								<ns1:_this type="PropertyCollector">{{property_collector}}</ns1:_this>
									<ns1:specSet>
										<ns1:propSet>
											<ns1:type>HostNetworkSystem</ns1:type>
											<all>0</all>
											<ns1:pathSet>networkInfo</ns1:pathSet>
										</ns1:propSet>
										<ns1:objectSet>
											<ns1:obj type="HostNetworkSystem">networkSystem</ns1:obj>
										</ns1:objectSet>
									</ns1:specSet>
								<ns1:options></ns1:options>
							</ns1:RetrievePropertiesEx>
						</SOAP-ENV:Body>"#
            .to_string();
        let body = Handlebars::new().render_template(&template, &args)?;
        let response = &client.request(body).await?;
        let root: Element = response.parse()?;

        let ns = "urn:vim25";
        let objects = root
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

        let mut nics: Vec<Pnic> = Vec::new();
        for obj in objects.children() {
            if obj.name() == "pnic" {
                let key = obj
                    .get_child("key", ns)
                    .ok_or(SoapError::XMLChildNotFound(
                        "returnval".to_string(),
                    ))?
                    .text();
                let device = obj
                    .get_child("device", ns)
                    .ok_or(SoapError::XMLChildNotFound("device".to_string()))?
                    .text();
                let mac = obj
                    .get_child("mac", ns)
                    .ok_or(SoapError::XMLChildNotFound("mac".to_string()))?
                    .text();

                if let Some(speed) = obj.get_child("linkSpeed", ns) {
                    nics.push(Pnic {
                        key,
                        device,
                        mac,
                        state: true,
                        bandwidth: speed
                            .get_child("speedMb", ns)
                            .map(|node| {
                                node.text().parse::<i64>().map_or_else(
                                    |_e| {
                                        Err(SoapError::XMLParseValue(
                                            "speedMb".to_string(),
                                            node.text(),
                                            String::from("i64"),
                                        ))
                                    },
                                    Ok,
                                )
                            })
                            .ok_or(SoapError::XMLChildNotFound(
                                "speedMb".to_string(),
                            ))??,
                    })
                } else {
                    nics.push(Pnic {
                        key,
                        device,
                        mac,
                        state: false,
                        bandwidth: 0,
                    })
                }
            }
        }

        Ok(PNicRequest { pnics: nics })
    }
}
