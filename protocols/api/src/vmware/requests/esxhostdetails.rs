/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::collections::HashMap;

use handlebars::Handlebars;
use minidom::Element;

use serde::{Deserialize, Serialize};
// use serde_json;

use crate::soap::{SoapClient, SoapError};

#[derive(Serialize, Deserialize, Debug)]
pub struct ESXHostDetailsRequest {
    pub hosts: Vec<Host>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Host {
    pub name: String,
    pub luns: Vec<Lun>,
    pub sensors: Vec<Sensor>,
    pub cpu: Vec<Cpu>,
    pub overall_status: String,
    pub total_memory: i64,
    pub memory_usage: i64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Lun {
    pub key: String,
    pub id: String,
    pub lun: String,
    pub paths: Vec<Path>,
}

impl Lun {
    fn from(propset: &Element) -> Result<Vec<Lun>, SoapError> {
        let ns = "urn:vim25";
        let mut luns: Vec<Lun> = Vec::new();

        for val in propset
            .get_child("val", ns)
            .ok_or(SoapError::XMLChildNotFound("val".to_string()))?
            .children()
        {
            let key = val
                .get_child("key", ns)
                .ok_or(SoapError::XMLChildNotFound("key".to_string()))?
                .text();
            let id = val
                .get_child("id", ns)
                .ok_or(SoapError::XMLChildNotFound("id".to_string()))?
                .text();
            let lun = val
                .get_child("lun", ns)
                .ok_or(SoapError::XMLChildNotFound("id".to_string()))?
                .text();
            let mut paths: Vec<Path> = Vec::new();
            for path in val.children() {
                if path.name() == "path" {
                    paths.push(Path::from(path)?);
                }
            }
            luns.push(Lun {
                key,
                id,
                lun,
                paths,
            });
        }
        Ok(luns)
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Path {
    pub key: String,
    pub name: String,
    pub path_state: String,
    pub state: String,
    pub is_working_path: bool,
}

impl Path {
    fn from(path: &Element) -> Result<Path, SoapError> {
        let ns = "urn:vim25";
        let is_working = path
            .get_child("isWorkingPath", ns)
            .ok_or(SoapError::XMLChildNotFound("isWorkingPath".to_string()))?
            .text();
        Ok(Path {
            key: path
                .get_child("key", ns)
                .ok_or(SoapError::XMLChildNotFound("key".to_string()))?
                .text(),
            name: path
                .get_child("name", ns)
                .ok_or(SoapError::XMLChildNotFound("name".to_string()))?
                .text(),
            path_state: path
                .get_child("pathState", ns)
                .ok_or(SoapError::XMLChildNotFound("pathState".to_string()))?
                .text(),
            state: path
                .get_child("state", ns)
                .ok_or(SoapError::XMLChildNotFound("state".to_string()))?
                .text(),
            is_working_path: is_working.parse::<bool>().map_or_else(
                |_e| {
                    Err(SoapError::XMLParseValue(
                        String::from("isWorkingPath"),
                        is_working,
                        String::from("bool"),
                    ))
                },
                Ok,
            )?,
        })
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Sensor {
    pub name: String,
    pub label: String,
    pub summary: String,
    pub key: String,
    pub sensor_type: String,
}

impl Sensor {
    fn from(propset: &Element) -> Result<Sensor, SoapError> {
        let ns = "urn:vim25";
        let health_state = propset
            .get_child("healthState", ns)
            .or(propset.get_child("status", ns))
            .ok_or(SoapError::XMLChildNotFound("status".to_string()))?;
        let name = propset
            .get_child("name", ns)
            .ok_or(SoapError::XMLChildNotFound("name".to_string()))?
            .text();
        Ok(Sensor {
            name: name.split(" --- ").next().unwrap_or(&name).to_string(),
            label: health_state
                .get_child("label", ns)
                .ok_or(SoapError::XMLChildNotFound("label".to_string()))?
                .text(),
            summary: health_state
                .get_child("summary", ns)
                .ok_or(SoapError::XMLChildNotFound("summary".to_string()))?
                .text(),
            key: health_state
                .get_child("key", ns)
                .ok_or(SoapError::XMLChildNotFound("key".to_string()))?
                .text(),
            sensor_type: match propset.get_child("sensorType", ns) {
                Some(e) => e.text(),
                None => String::new(),
            },
        })
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Cpu {
    pub index: i64,
    pub hz: i64,
    pub bus_hz: i64,
    pub description: String,
}

impl Cpu {
    fn from(propset: &Element) -> Result<Vec<Cpu>, SoapError> {
        let ns = "urn:vim25";
        let mut cpus: Vec<Cpu> = Vec::new();
        for cpupkg in propset
            .get_child("val", ns)
            .ok_or(SoapError::XMLChildNotFound("val".to_string()))?
            .children()
        {
            cpus.push(Cpu {
                index: cpupkg
                    .get_child("index", ns)
                    .map(|cpupkg| {
                        cpupkg.text().parse::<i64>().map_or_else(
                            |_e| {
                                Err(SoapError::XMLParseValue(
                                    "index".to_string(),
                                    cpupkg.text(),
                                    String::from("i64"),
                                ))
                            },
                            Ok,
                        )
                    })
                    .ok_or(SoapError::XMLChildNotFound(
                        "index".to_string(),
                    ))??,
                hz: cpupkg
                    .get_child("hz", ns)
                    .map(|cpupkg| {
                        cpupkg.text().parse::<i64>().map_or_else(
                            |_e| {
                                Err(SoapError::XMLParseValue(
                                    "hz".to_string(),
                                    cpupkg.text(),
                                    String::from("i64"),
                                ))
                            },
                            Ok,
                        )
                    })
                    .ok_or(SoapError::XMLChildNotFound("hz".to_string()))??,
                bus_hz: cpupkg
                    .get_child("busHz", ns)
                    .map(|cpupkg| {
                        cpupkg.text().parse::<i64>().map_or_else(
                            |_e| {
                                Err(SoapError::XMLParseValue(
                                    "busHz".to_string(),
                                    cpupkg.text(),
                                    String::from("i64"),
                                ))
                            },
                            Ok,
                        )
                    })
                    .ok_or(SoapError::XMLChildNotFound(
                        "busHz".to_string(),
                    ))??,
                description: cpupkg
                    .get_child("description", ns)
                    .ok_or(SoapError::XMLChildNotFound(
                        "description".to_string(),
                    ))?
                    .text(),
            })
        }
        Ok(cpus)
    }
}

impl ESXHostDetailsRequest {
    pub async fn new(
        client: &SoapClient,
        args: &HashMap<String, String>,
    ) -> Result<ESXHostDetailsRequest, SoapError> {
        let template = r#"<SOAP-ENV:Body xmlns:ns1="urn:vim25">
				<ns1:RetrievePropertiesEx xsi:type="ns1:RetrievePropertiesExRequestType">
					<ns1:_this type="PropertyCollector">{{property_collector}}</ns1:_this>
					<ns1:specSet>
						<ns1:propSet>
							<ns1:type>HostSystem</ns1:type>
							<ns1:pathSet>summary.quickStats.overallMemoryUsage</ns1:pathSet>
							<ns1:pathSet>hardware.cpuPkg</ns1:pathSet>
							<ns1:pathSet>hardware.pciDevice</ns1:pathSet>
							<ns1:pathSet>runtime.powerState</ns1:pathSet>
							<ns1:pathSet>summary.quickStats.overallCpuUsage</ns1:pathSet>
							<ns1:pathSet>hardware.biosInfo.biosVersion</ns1:pathSet>
							<ns1:pathSet>hardware.biosInfo.releaseDate</ns1:pathSet>
							<ns1:pathSet>hardware.cpuInfo.hz</ns1:pathSet>
							<ns1:pathSet>hardware.cpuInfo.numCpuThreads</ns1:pathSet>
							<ns1:pathSet>hardware.cpuInfo.numCpuPackages</ns1:pathSet>
							<ns1:pathSet>hardware.cpuInfo.numCpuCores</ns1:pathSet>
							<ns1:pathSet>config.storageDevice.multipathInfo</ns1:pathSet>
							<ns1:pathSet>hardware.systemInfo.model</ns1:pathSet>
							<ns1:pathSet>hardware.systemInfo.uuid</ns1:pathSet>
							<ns1:pathSet>hardware.systemInfo.otherIdentifyingInfo</ns1:pathSet>
							<ns1:pathSet>hardware.systemInfo.vendor</ns1:pathSet>
							<ns1:pathSet>name</ns1:pathSet>
							<ns1:pathSet>overallStatus</ns1:pathSet>
							<ns1:pathSet>runtime.healthSystemRuntime.systemHealthInfo.numericSensorInfo</ns1:pathSet>
							<ns1:pathSet>runtime.healthSystemRuntime.hardwareStatusInfo.storageStatusInfo</ns1:pathSet>
							<ns1:pathSet>runtime.healthSystemRuntime.hardwareStatusInfo.cpuStatusInfo</ns1:pathSet>
							<ns1:pathSet>runtime.healthSystemRuntime.hardwareStatusInfo.memoryStatusInfo</ns1:pathSet>
							<ns1:pathSet>runtime.inMaintenanceMode</ns1:pathSet>
							<ns1:pathSet>hardware.memorySize</ns1:pathSet>
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
			</SOAP-ENV:Body>"#.to_string();
        let body = Handlebars::new().render_template(&template, &args)?;
        let response = &client.request(body).await?;
        // println!("hostdetails: {}", &response);
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

        let mut hosts: Vec<Host> = Vec::new();
        let sensor_names = [
            "runtime.healthSystemRuntime.hardwareStatusInfo.storageStatusInfo",
            "runtime.healthSystemRuntime.hardwareStatusInfo.cpuStatusInfo",
            "runtime.healthSystemRuntime.hardwareStatusInfo.memoryStatusInfo",
            "runtime.healthSystemRuntime.systemHealthInfo.numericSensorInfo",
        ];
        for obj in objects.children() {
            let mut name = String::new();
            let mut luns: Vec<Lun> = Vec::new();
            let mut sensors: Vec<Sensor> = Vec::new();
            let mut cpu: Vec<Cpu> = Vec::new();
            let mut overall_status = String::new();
            let mut total_memory: i64 = 0;
            let mut memory_usage: i64 = 0;

            for propset in obj.children() {
                if propset.name() == "obj" {
                    name = propset.text();
                } else {
                    let name = propset
                        .get_child("name", ns)
                        .ok_or(SoapError::XMLChildNotFound("name".to_string()))?
                        .text();
                    let val = propset.get_child("val", ns).ok_or(
                        SoapError::XMLChildNotFound("val".to_string()),
                    )?;
                    if name == "summary.quickStats.overallMemoryUsage" {
                        // println!("summary.quickStats.overallMemoryUsage found: '{}'", val.text());
                        memory_usage = val.text().parse::<i64>().map_or_else(
                            |_e| {
                                Err(SoapError::XMLParseValue(
                                    format!("{}: {}", name, val.text()),
                                    val.text(),
                                    String::from("i64"),
                                ))
                            },
                            Ok,
                        )?;
                    } else if name == "hardware.memorySize" {
                        // println!("hardware.memorySize found: '{}'", val.text());
                        total_memory = val.text().parse::<i64>().map_or_else(
                            |_e| {
                                Err(SoapError::XMLParseValue(
                                    format!("{}: {}", name, val.text()),
                                    val.text(),
                                    String::from("i64"),
                                ))
                            },
                            Ok,
                        )?;
                    } else if name == "config.storageDevice.multipathInfo" {
                        luns = Lun::from(propset)?;
                    } else if sensor_names.contains(&name.as_str()) {
                        for sensor in val.children() {
                            sensors.push(Sensor::from(sensor)?);
                        }
                    } else if name == "hardware.cpuPkg" {
                        cpu = Cpu::from(propset)?;
                    } else if name == "overallStatus" {
                        overall_status = val.text();
                    }
                }
            }
            hosts.push(Host {
                name,
                luns,
                sensors,
                cpu,
                overall_status,
                total_memory,
                memory_usage,
            });
        }

        Ok(ESXHostDetailsRequest { hosts })
    }
}
