/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::collections::HashMap;
use std::sync::Arc;

use handlebars::Handlebars;
use minidom::Element;
// use serde_json;

use value::{Data, EnumValue, Value};

use crate::soap::{SoapClient, SoapError};

#[derive(Debug)]
pub struct DatastoresRequest {
    pub datastores: Vec<HashMap<String, Data>>,
}

impl DatastoresRequest {
    pub async fn new(
        client: &SoapClient,
        args: &HashMap<String, String>,
    ) -> Result<DatastoresRequest, SoapError> {
        let template = r#"<SOAP-ENV:Body xmlns:ns1="urn:vim25">
				<ns1:RetrievePropertiesEx xsi:type="ns1:RetrievePropertiesExRequestType">
					<ns1:_this type="PropertyCollector">{{property_collector}}</ns1:_this>
					<ns1:specSet>
						<ns1:propSet>
							<ns1:type>Datastore</ns1:type>
							<ns1:pathSet>summary.name</ns1:pathSet>
							<ns1:pathSet>summary.freeSpace</ns1:pathSet>
							<ns1:pathSet>summary.capacity</ns1:pathSet>
							<ns1:pathSet>summary.uncommitted</ns1:pathSet>
							<ns1:pathSet>summary.url</ns1:pathSet>
							<ns1:pathSet>summary.accessible</ns1:pathSet>
							<ns1:pathSet>summary.type</ns1:pathSet>
							<ns1:pathSet>summary.maintenanceMode</ns1:pathSet>
							<ns1:pathSet>vm</ns1:pathSet>
						</ns1:propSet>
						<ns1:objectSet>
							<ns1:obj type="Folder">{{root_folder}}</ns1:obj>
							<ns1:skip>false</ns1:skip>
							<ns1:selectSet xsi:type="ns1:TraversalSpec">
								<ns1:name>visitFolders</ns1:name>
								<ns1:type>Folder</ns1:type>
								<ns1:path>childEntity</ns1:path>
								<ns1:skip>false</ns1:skip>
								<ns1:selectSet>
									<ns1:name>visitFolders</ns1:name>
								</ns1:selectSet>
								<ns1:selectSet>
									<ns1:name>dcToHf</ns1:name>
								</ns1:selectSet>
								<ns1:selectSet>
									<ns1:name>dcToVmf</ns1:name>
								</ns1:selectSet>
								<ns1:selectSet>
									<ns1:name>crToH</ns1:name>
								</ns1:selectSet>
								<ns1:selectSet>
									<ns1:name>crToRp</ns1:name>
								</ns1:selectSet>
								<ns1:selectSet>
									<ns1:name>dcToDs</ns1:name>
								</ns1:selectSet>
								<ns1:selectSet>
									<ns1:name>hToVm</ns1:name>
								</ns1:selectSet>
								<ns1:selectSet>
									<ns1:name>rpToVm</ns1:name>
								</ns1:selectSet>
							</ns1:selectSet>
							<ns1:selectSet xsi:type="ns1:TraversalSpec">
								<ns1:name>dcToVmf</ns1:name>
								<ns1:type>Datacenter</ns1:type>
								<ns1:path>vmFolder</ns1:path>
								<ns1:skip>false</ns1:skip>
								<ns1:selectSet>
									<ns1:name>visitFolders</ns1:name>
								</ns1:selectSet>
							</ns1:selectSet>
							<ns1:selectSet xsi:type="ns1:TraversalSpec">
								<ns1:name>dcToDs</ns1:name>
								<ns1:type>Datacenter</ns1:type>
								<ns1:path>datastore</ns1:path>
								<ns1:skip>false</ns1:skip>
								<ns1:selectSet>
									<ns1:name>visitFolders</ns1:name>
								</ns1:selectSet>
							</ns1:selectSet>
							<ns1:selectSet xsi:type="ns1:TraversalSpec">
								<ns1:name>dcToHf</ns1:name>
								<ns1:type>Datacenter</ns1:type>
								<ns1:path>hostFolder</ns1:path>
								<ns1:skip>false</ns1:skip>
								<ns1:selectSet>
									<ns1:name>visitFolders</ns1:name>
								</ns1:selectSet>
							</ns1:selectSet>
							<ns1:selectSet xsi:type="ns1:TraversalSpec">
								<ns1:name>crToH</ns1:name>
								<ns1:type>ComputeResource</ns1:type>
								<ns1:path>host</ns1:path>
								<ns1:skip>false</ns1:skip>
							</ns1:selectSet>
							<ns1:selectSet xsi:type="ns1:TraversalSpec">
								<ns1:name>crToRp</ns1:name>
								<ns1:type>ComputeResource</ns1:type>
								<ns1:path>resourcePool</ns1:path>
								<ns1:skip>false</ns1:skip>
								<ns1:selectSet>
									<ns1:name>rpToRp</ns1:name>
								</ns1:selectSet>
								<ns1:selectSet>
									<ns1:name>rpToVm</ns1:name>
								</ns1:selectSet>
							</ns1:selectSet>
							<ns1:selectSet xsi:type="ns1:TraversalSpec">
								<ns1:name>rpToRp</ns1:name>
								<ns1:type>ResourcePool</ns1:type>
								<ns1:path>resourcePool</ns1:path>
								<ns1:skip>false</ns1:skip>
								<ns1:selectSet>
									<ns1:name>rpToRp</ns1:name>
								</ns1:selectSet>
								<ns1:selectSet>
									<ns1:name>rpToVm</ns1:name>
								</ns1:selectSet>
							</ns1:selectSet>
							<ns1:selectSet xsi:type="ns1:TraversalSpec">
								<ns1:name>hToVm</ns1:name>
								<ns1:type>HostSystem</ns1:type>
								<ns1:path>vm</ns1:path>
								<ns1:skip>false</ns1:skip>
								<ns1:selectSet>
									<ns1:name>visitFolders</ns1:name>
								</ns1:selectSet>
							</ns1:selectSet>
							<ns1:selectSet xsi:type="ns1:TraversalSpec">
								<ns1:name>rpToVm</ns1:name>
								<ns1:type>ResourcePool</ns1:type>
								<ns1:path>vm</ns1:path>
								<ns1:skip>false</ns1:skip>
							</ns1:selectSet>
						</ns1:objectSet>
					</ns1:specSet>
					<ns1:options></ns1:options>
				</ns1:RetrievePropertiesEx>
			</SOAP-ENV:Body>"#
            .to_string();
        let body = Handlebars::new().render_template(&template, &args)?;
        let response = &client.request(body).await?;
        let root: Element = response.parse()?;
        let mut datastores: Vec<HashMap<String, Data>> = Vec::new();
        let ns = "urn:vim25";
        let objects = root
            .get_child("Body", "http://schemas.xmlsoap.org/soap/envelope/")
            .ok_or(SoapError::XMLChildNotFound("Body".to_string()))?
            .get_child("RetrievePropertiesExResponse", ns)
            .ok_or(SoapError::XMLChildNotFound(
                "RetrievePropertiesExResponse".to_string(),
            ))?
            .get_child("returnval", ns)
            .ok_or(SoapError::XMLChildNotFound("returnval".to_string()))?;

        for obj in objects.children() {
            let mut dtstore: HashMap<String, Data> = HashMap::new();
            for elem in obj.children() {
                if elem.name() == "obj" {
                    dtstore.insert(
                        "uuid".to_string(),
                        Ok(Value::UnicodeString(elem.text())),
                    );
                } else {
                    let name = elem
                        .get_child("name", ns)
                        .ok_or(SoapError::XMLChildNotFound("name".to_string()))?
                        .text();
                    let val = elem.get_child("val", ns).ok_or(
                        SoapError::XMLChildNotFound("val".to_string()),
                    )?;
                    let content = val.text();
                    dtstore.insert(
                        elem.get_child("name", ns)
                            .ok_or(SoapError::XMLChildNotFound(
                                "name".to_string(),
                            ))?
                            .text(),
                        match val.attr("xsi:type").ok_or(
                            SoapError::XMLAttrributeNotFound(
                                "xsi:type".to_string(),
                            ),
                        )? {
                            "xsd:boolean" => Ok(Value::Boolean(
                                content.parse::<bool>().map_or_else(
                                    |_e| {
                                        Err(SoapError::XMLParseValue(
                                            name.to_string(),
                                            content,
                                            String::from("bool"),
                                        ))
                                    },
                                    Ok,
                                )?,
                            )),
                            "xsd:long" => Ok(Value::Integer(
                                content.parse::<i64>().map_or_else(
                                    |_e| {
                                        Err(SoapError::XMLParseValue(
                                            name.to_string(),
                                            content,
                                            String::from("i64"),
                                        ))
                                    },
                                    Ok,
                                )?,
                            )),
                            "xsd:string" => {
                                if name == "summary.type" {
                                    EnumValue::new(
                                        Arc::new(
                                            vec![
                                                "VMFS ", "NFS ", "vSAN", "vVol",
                                            ]
                                            .into_iter()
                                            .map(|s| s.to_string())
                                            .collect(),
                                        ),
                                        content,
                                    )
                                    .map(Value::Enum)
                                } else {
                                    Ok(Value::UnicodeString(content))
                                }
                            }
                            _ => Ok(Value::UnicodeString(content)),
                        },
                    );
                }
            }
            datastores.push(dtstore);
        }

        // println!("{}", serde_json::to_string(&datastores).unwrap());
        Ok(DatastoresRequest { datastores })
    }
}
