/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::collections::HashMap;

use handlebars::Handlebars;
use regex::Regex;

use crate::soap::{SoapClient, SoapError};

#[derive(Debug)]
pub struct HostsytemsRequest {
    pub systems: HashMap<String, String>,
}

impl HostsytemsRequest {
    pub async fn new(
        client: &SoapClient,
        args: &HashMap<String, String>,
    ) -> Result<HostsytemsRequest, SoapError> {
        let template = r#"<SOAP-ENV:Body xmlns:ns1="urn:vim25">
				<ns1:RetrievePropertiesEx xsi:type="ns1:RetrievePropertiesExRequestType">
					<ns1:_this type="PropertyCollector">{{property_collector}}</ns1:_this>
					<ns1:specSet>
						<ns1:propSet>
							<ns1:type>HostSystem</ns1:type>
							<ns1:pathSet>name</ns1:pathSet>
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

        let re = Regex::new(r#"<obj type="HostSystem">(.*?)</obj>.*?<val xsi:type="xsd:string">(.*?)</val>"#).unwrap();
        let mut systems: HashMap<String, String> = HashMap::new();
        for obj in re.captures_iter(response) {
            systems.insert(obj[1].to_string(), obj[2].to_string());
        }

        Ok(HostsytemsRequest { systems })
    }
}
