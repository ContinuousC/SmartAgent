/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::collections::HashMap;
use std::sync::Arc;

use handlebars::Handlebars;
use minidom::Element;

use value::{Data, EnumValue, Value};

use crate::soap::{self, SoapClient, SoapError};

#[derive(Debug)]
pub struct VmDetailsRequest {
    pub vms: Vec<HashMap<String, Data>>,
    pub datastores: HashMap<String, Vec<VmDataStore>>,
}

#[derive(Debug)]
pub struct VmDataStore {
    pub datastore: Data,
    pub committed: Data,
    pub uncommitted: Data,
    pub unshared: Data,
}

impl VmDetailsRequest {
    pub async fn new(
        client: &SoapClient,
        args: &HashMap<String, String>,
    ) -> Result<VmDetailsRequest, SoapError> {
        let template = r#"<SOAP-ENV:Body xmlns:ns1="urn:vim25">
							<ns1:RetrievePropertiesEx xsi:type="ns1:RetrievePropertiesExRequestType">
								<ns1:_this type="PropertyCollector">{{property_collector}}</ns1:_this>
								<ns1:specSet>
									<ns1:propSet>
										<ns1:type>VirtualMachine</ns1:type>
										<ns1:pathSet>summary.config.ftInfo.role</ns1:pathSet>
										<ns1:pathSet>summary.quickStats.consumedOverheadMemory</ns1:pathSet>
										<ns1:pathSet>config.hardware.numCPU</ns1:pathSet>
										<ns1:pathSet>summary.quickStats.overallCpuDemand</ns1:pathSet>
										<ns1:pathSet>summary.quickStats.distributedCpuEntitlement</ns1:pathSet>
										<ns1:pathSet>runtime.host</ns1:pathSet>
										<ns1:pathSet>summary.quickStats.distributedMemoryEntitlement</ns1:pathSet>
										<ns1:pathSet>summary.quickStats.uptimeSeconds</ns1:pathSet>
										<ns1:pathSet>summary.quickStats.sharedMemory</ns1:pathSet>
										<ns1:pathSet>config.hardware.memoryMB</ns1:pathSet>
										<ns1:pathSet>summary.quickStats.privateMemory</ns1:pathSet>
										<ns1:pathSet>summary.quickStats.balloonedMemory</ns1:pathSet>
										<ns1:pathSet>summary.quickStats.staticMemoryEntitlement</ns1:pathSet>
										<ns1:pathSet>runtime.powerState</ns1:pathSet>
										<ns1:pathSet>summary.quickStats.overallCpuUsage</ns1:pathSet>
										<ns1:pathSet>config.hardware.numCoresPerSocket</ns1:pathSet>
										<ns1:pathSet>guest.toolsVersion</ns1:pathSet>
										<ns1:pathSet>guest.disk</ns1:pathSet>
										<ns1:pathSet>guestHeartbeatStatus</ns1:pathSet>
										<ns1:pathSet>name</ns1:pathSet>
										<ns1:pathSet>summary.guest.hostName</ns1:pathSet>
										<ns1:pathSet>config.guestFullName</ns1:pathSet>
										# Guest OS
										<ns1:pathSet>config.version</ns1:pathSet>
										# Compatibility
										<ns1:pathSet>config.uuid</ns1:pathSet>
										<ns1:pathSet>summary.quickStats.compressedMemory</ns1:pathSet>
										<ns1:pathSet>summary.quickStats.swappedMemory</ns1:pathSet>
										<ns1:pathSet>summary.quickStats.guestMemoryUsage</ns1:pathSet>
										<ns1:pathSet>summary.quickStats.staticCpuEntitlement</ns1:pathSet>
										<ns1:pathSet>summary.quickStats.hostMemoryUsage</ns1:pathSet>
										<ns1:pathSet>snapshot.rootSnapshotList</ns1:pathSet>
										<ns1:pathSet>config.datastoreUrl</ns1:pathSet>
										<ns1:pathSet>guest.toolsVersionStatus2</ns1:pathSet>
										# storage
										<ns1:pathSet>summary.storage.committed</ns1:pathSet>
										<ns1:pathSet>summary.storage.timestamp</ns1:pathSet>
										<ns1:pathSet>summary.storage.uncommitted</ns1:pathSet>
										<ns1:pathSet>summary.storage.unshared</ns1:pathSet>
										<ns1:pathSet>storage.perDatastoreUsage</ns1:pathSet>
										<ns1:pathSet>layoutEx.file</ns1:pathSet>
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

        let properties = vec![
            "name",
            "guestHeartbeatStatus",
            "summary.guest.hostName",
            "summary.quickStats.consumedOverheadMemory",
            "config.hardware.memoryMB",
            "config.hardware.numCPU",
            "guest.toolsVersion",
            "guest.toolsVersionStatus2",
            "guest.disk",
            "runtime.powerState",
            "summary.storage.committed",
            "summary.storage.timestamp",
            "summary.storage.uncommitted",
            "summary.storage.unshared",
            "storage.perDatastoreUsage",
        ];
        let integers = [
            "summary.quickStats.consumedOverheadMemory",
            "config.hardware.memoryMB",
            "config.hardware.numCPU",
            "summary.storage.committed",
            "summary.storage.uncommitted",
            "summary.storage.unshared",
        ];
        let mut vms: Vec<HashMap<String, Data>> = Vec::new();
        let mut datastores: HashMap<String, Vec<VmDataStore>> = HashMap::new();
        for obj in objects.children() {
            let mut vm: HashMap<String, Data> = HashMap::new();
            let mut id: String = String::new();
            for elem in obj.children() {
                if elem.name() == "obj" {
                    id = elem.text();
                    vm.insert(
                        "id".to_string(),
                        Ok(Value::UnicodeString(id.to_string())),
                    );
                } else {
                    let name = elem
                        .get_child("name", ns)
                        .ok_or(SoapError::XMLChildNotFound(
                            "returnval".to_string(),
                        ))?
                        .text();
                    if properties.contains(&name.as_str()) {
                        let val = elem.get_child("val", ns).ok_or(
                            SoapError::XMLChildNotFound("val".to_string()),
                        )?;
                        if &name == "guestHeartbeatStatus" {
                            vm.insert(
                                name.clone(),
                                EnumValue::new(
                                    Arc::new(
                                        vec!["green", "yellow", "red", "gray"]
                                            .into_iter()
                                            .map(|s| s.to_string())
                                            .collect(),
                                    ),
                                    val.text(),
                                )
                                .map(Value::Enum),
                            );
                        } else if &name == "storage.perDatastoreUsage" {
                            let mut vm_datastores: Vec<VmDataStore> =
                                Vec::new();
                            for datastore in val.children() {
                                vm_datastores.push(VmDataStore {
                                    datastore: Ok(soap::get_child_as_string(
                                        datastore,
                                        ns.to_string(),
                                        String::from("datastore"),
                                    )?),
                                    committed: Ok(soap::get_child_as_int(
                                        datastore,
                                        ns.to_string(),
                                        String::from("committed"),
                                    )?),
                                    uncommitted: Ok(soap::get_child_as_int(
                                        datastore,
                                        ns.to_string(),
                                        String::from("uncommitted"),
                                    )?),
                                    unshared: Ok(soap::get_child_as_int(
                                        datastore,
                                        ns.to_string(),
                                        String::from("unshared"),
                                    )?),
                                })
                            }
                            datastores.insert(id.clone(), vm_datastores);
                        } else if &name == "guest.disk" {
                            let mut capacity: i64 = 0;
                            let mut free: i64 = 0;
                            for disk in val.children() {
                                if let Value::Integer(i) =
                                    soap::get_child_as_int(
                                        disk,
                                        ns.to_string(),
                                        String::from("capacity"),
                                    )?
                                {
                                    capacity += i;
                                }
                                if let Value::Integer(i) =
                                    soap::get_child_as_int(
                                        disk,
                                        ns.to_string(),
                                        String::from("freeSpace"),
                                    )?
                                {
                                    free += i;
                                }
                            }
                            vm.insert(
                                String::from("guest.disk.capacity"),
                                Ok(Value::Integer(capacity)),
                            );
                            vm.insert(
                                String::from("guest.disk.free"),
                                Ok(Value::Integer(free)),
                            );
                        } else if integers.contains(&name.as_str()) {
                            vm.insert(
                                name.clone(),
                                Ok(soap::get_child_as_int(
                                    elem,
                                    ns.to_string(),
                                    String::from("val"),
                                )?),
                            );
                        } else {
                            vm.insert(
                                name,
                                Ok(soap::get_child_as_string(
                                    elem,
                                    ns.to_string(),
                                    String::from("val"),
                                )?),
                            );
                        }
                    }
                }
            }
            vms.push(vm);
        }

        Ok(VmDetailsRequest { vms, datastores })
    }
}
