/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::cmp;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use futures::{stream, StreamExt};
use reqwest::header::{HeaderMap, HeaderValue};

use agent_utils::{KeyVault, TryGetFrom};
use etc_base::{Annotated, ProtoDataFieldId, ProtoDataTableId, ProtoQueryMap};
use log::info;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use value::{DataError, EnumValue, Value};

use super::error::{DTError, Error, Result};
use super::requests::{
    AvailableCountersRequest, DatastoresRequest, ESXHostDetailsRequest,
    HostsytemsRequest, LicensesRequest, LoginRequest, PNicRequest,
    PerfCounterDataRequest, PerfCounterSyntaxRequest, StatsType,
    SysteminfoRequest, VmDetailsRequest,
};
use super::Config;
use crate::error::Result as APIResult;
use crate::input::PluginId;
use crate::livestatus::LivestatusSocket;
use crate::plugin::{DataMap, TableData};
use crate::soap::SoapClient;
use crate::vmware::command::Command;
use crate::vmware::config::Credentials;
use crate::Input;
use crate::{APIPlugin, Plugin as ProtPlugin};

pub struct Plugin {
    key_vault: KeyVault,
    cache_dir: PathBuf,
    config: Config,
}

impl Plugin {
    pub fn new(
        cache_dir: PathBuf,
        key_vault: KeyVault,
        config: Config,
    ) -> Result<Self> {
        Ok(Self {
            key_vault,
            cache_dir,
            config,
        })
    }
}

#[async_trait]
impl APIPlugin for Plugin {
    async fn run_queries(
        &self,
        input: &Input,
        query: &ProtoQueryMap,
    ) -> APIResult<DataMap> {
        info!("Using vmware plugin");

        let endpoint = format!(
            "https://{}:{}/sdk",
            self.config.get_hostname().await?,
            self.config.port.unwrap_or(443)
        );
        let mut headers: HeaderMap = HeaderMap::new();
        headers.insert("SOAPAction", HeaderValue::from_static("urn:vim25/5.0"));
        let soapclient = Arc::new(
            SoapClient::create(
                endpoint,
                headers,
                self.config.certificate.as_ref(),
                self.config
                    .disable_certificate_verification
                    .unwrap_or(false),
                self.config.disable_hostname_verification.unwrap_or(false),
            )
            .await
            .map_err(Error::SoapError)?,
        );

        let sysinfo = SysteminfoRequest::new(&soapclient, &HashMap::new())
            .await
            .map_err(Error::SoapError)?;
        info!("sysinfo: {:?}", &sysinfo);
        let mut args = sysinfo.to_hashmap();

        if let Some(creds) = &self.config.credentials {
            let creds = match self.key_vault {
                KeyVault::Identity => creds.clone(),
                KeyVault::KeyReader(_) => {
                    let kr_entry = self
                        .key_vault
                        .retrieve_creds(creds.username.clone())
                        .await?
                        .ok_or(Error::MissingKREntry)?;

                    Credentials {
                        username: kr_entry.username.ok_or(
                            Error::MissingKRObject(String::from("Username")),
                        )?,
                        password: Some(kr_entry.password.ok_or(
                            Error::MissingKRObject(String::from("Password")),
                        )?),
                    }
                }
            };
            args.insert(String::from("username"), creds.username);
            args.insert(
                String::from("password"),
                creds
                    .password
                    .ok_or(Error::MissingKRObject(String::from("Password")))?,
            );

            LoginRequest::new(&soapclient, &args)
                .await
                .map_err(Error::Login)?;
        }

        let hostsystems = HostsytemsRequest::new(&soapclient, &args)
            .await
            .map_err(Error::SoapError)?
            .systems;
        info!("hostsytems: {:?}", &hostsystems);

        let ts_file = self.cache_dir.join("counters.timestamp");

        let mut requests = Vec::new();
        for (table_id, field_ids) in query {
            let command = ProtPlugin::get_datatable_id(table_id)
                .try_get_from(&input.data_tables)?;
            if command.plugin != PluginId(String::from("vmware")) {
                continue; // shouldn't happend, but better safe than sorry
            }
            let fields = field_ids
                .iter()
                .map(|df_id| {
                    Ok((
                        df_id.clone(),
                        ProtPlugin::get_datafield_id(df_id)
                            .try_get_from(&input.data_fields)?
                            .parameter_name
                            .clone(),
                    ))
                })
                .collect::<Result<HashMap<ProtoDataFieldId, String>>>()?;

            let cmd = Command {
                fields,
                soapclient: soapclient.clone(),
                args: args.clone(),
                ts_file: ts_file.clone(),
                hostsystems: hostsystems.clone(),
                table: table_id.clone(),
                command_name: command.command_name.clone(),
                command_line: command.command_line.clone(),
            };
            requests
                .push(exec_query(cmd, self.config.is_cluster.unwrap_or(false)))
        }

        let data: DataMap =
            stream::iter(requests).buffer_unordered(8).collect().await;

        if let Some(dir) = ts_file.parent() {
            fs::create_dir_all(dir).await?;
        }
        let mut f = fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(ts_file)
            .await?;
        f.write_all(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or_else(|_e| Err(Error::SysTime), Ok)?
                .as_secs()
                .to_string()
                .as_bytes(),
        )
        .await?;

        Ok(data)
    }
}

async fn exec_query(
    cmd: Command,
    is_cluster: bool,
) -> (ProtoDataTableId, TableData) {
    let cmd_line = &cmd.command_name.clone();
    info!("executing cmd: {}", &cmd_line);
    let res = match cmd_line.as_str() {
        "get_nics" => get_nics(cmd).await,
        "get_counter" => get_counter(cmd).await,
        "get_hostdetails" => get_hostdetails(cmd).await,
        "get_vms" => get_vms(cmd).await,
        "get_licenses" => get_licenses(cmd).await,
        _ => {
            if is_cluster {
                match cmd.command_name.as_str() {
                    "get_datastores" => get_datastores(cmd).await,
                    _ => (
                        cmd.table,
                        Err(DTError::CommandNotFound(cmd.command_name).to_api()),
                    ),
                }
            } else {
                (
                    cmd.table,
                    Err(DTError::CommandNotFound(cmd.command_name).to_api()),
                )
            }
        }
    };
    info!(
        "cmd {} executed {}",
        cmd_line,
        if res.1.is_ok() {
            "succesfully"
        } else {
            "unsuccesfully"
        }
    );
    res
}

async fn get_nics(cmd: Command) -> (ProtoDataTableId, TableData) {
    match PNicRequest::new(&cmd.soapclient, &cmd.args).await {
        Err(e) => (cmd.table, Err(DTError::SoapError(e).to_api())),
        Ok(pnic_request) => {
            info!("Found {} physical nics", pnic_request.pnics.len());
            let mut rows = Vec::with_capacity(pnic_request.pnics.len());
            for nic in pnic_request.pnics {
                rows.push(
                    cmd.fields
                        .iter()
                        .map(|(fieldid, param)| {
                            (
                                fieldid.clone(),
                                match param.as_str() {
                                    "key" => Ok(Value::UnicodeString(
                                        nic.key.to_string(),
                                    )),
                                    "device" => Ok(Value::UnicodeString(
                                        nic.device.to_string(),
                                    )),
                                    "mac" => Ok(Value::UnicodeString(
                                        nic.mac.to_string(),
                                    )),
                                    "bandwidth" => {
                                        Ok(Value::Integer(nic.bandwidth))
                                    }
                                    "state" => Ok(Value::Boolean(nic.state)),
                                    _ => Err(DataError::InvalidChoice(
                                        fieldid.to_string(),
                                    )),
                                },
                            )
                        })
                        .collect(),
                );
            }
            (
                cmd.table,
                Ok(Annotated {
                    value: rows,
                    warnings: Vec::new(),
                }),
            )
        }
    }
}

async fn get_licenses(cmd: Command) -> (ProtoDataTableId, TableData) {
    match LicensesRequest::new(&cmd.soapclient, &cmd.args).await {
        Err(e) => (cmd.table, Err(DTError::SoapError(e).to_api())),
        Ok(lic_req) => {
            info!("found {} licences", lic_req.licenses.len());
            let mut rows = Vec::with_capacity(lic_req.licenses.len());
            for lic in lic_req.licenses {
                rows.push(
                    cmd.fields
                        .iter()
                        .map(|(fieldid, param)| {
                            (
                                fieldid.clone(),
                                match param.as_str() {
                                    "key" => Ok(Value::UnicodeString(
                                        lic.key.to_string(),
                                    )),
                                    "name" => Ok(Value::UnicodeString(
                                        lic.name.to_string(),
                                    )),
                                    "total" => Ok(Value::Integer(lic.total)),
                                    "used" => Ok(Value::Integer(lic.used)),
                                    _ => Err(DataError::InvalidChoice(
                                        fieldid.to_string(),
                                    )),
                                },
                            )
                        })
                        .collect(),
                );
            }
            (
                cmd.table,
                Ok(Annotated {
                    value: rows,
                    warnings: Vec::new(),
                }),
            )
        }
    }
}

async fn get_datastores(cmd: Command) -> (ProtoDataTableId, TableData) {
    match DatastoresRequest::new(&cmd.soapclient, &cmd.args).await {
        Err(e) => (cmd.table, Err(DTError::SoapError(e).to_api())),
        Ok(ds_req) => {
            info!("found {} datastores", ds_req.datastores.len());
            let mut rows = Vec::with_capacity(ds_req.datastores.len());
            for mut ds in ds_req.datastores {
                rows.push(
                    cmd.fields
                        .iter()
                        .map(|(fieldid, param)| {
                            (
                                fieldid.clone(),
                                ds.remove(param)
                                    .unwrap_or(Err(DataError::Missing)),
                            )
                        })
                        .collect(),
                );
            }
            (
                cmd.table,
                Ok(Annotated {
                    value: rows,
                    warnings: Vec::new(),
                }),
            )
        }
    }
}

async fn get_counter(mut cmd: Command) -> (ProtoDataTableId, TableData) {
    // TODO: improvement: request only the needed counters instead of all of them
    // And the obvious refactor of course
    let mut data = Vec::new();

    info!("retrieve samples from: {:?}", &cmd.ts_file);
    let last_run = fs::read_to_string(cmd.ts_file.clone()).await;
    info!("time of last run: {:?}", last_run);
    let samples: u64 = cmp::max(
        match last_run {
            Ok(s) => cmp::min(
                (match SystemTime::now().duration_since(UNIX_EPOCH) {
                    Err(_) => {
                        return (cmd.table, Err(DTError::SysTime.to_api()))
                    }
                    Ok(v) => v.as_secs(),
                } - match s.parse::<u64>() {
                    Err(_) => {
                        return (
                            cmd.table,
                            Err(DTError::ParseIntError(s).to_api()),
                        )
                    }
                    Ok(i) => i,
                }) / 20,
                60 * 60 / 3,
            ), // 20 seconds per sample, can only go back 1 hour ago, with a minumum of 1
            Err(_e) => Duration::new(3 * 15, 0).as_secs(), // 20 seconds per sample * 3 for 1 minutte * 15 minuts
        },
        1,
    );
    cmd.args.insert("samples".to_string(), samples.to_string());
    info!("need to retrieve {} counter samples", samples);

    let mut set_instance = None;
    let mut set_host = None;
    let mut set_hostname = None;

    for (system, name) in cmd.hostsystems {
        cmd.args.insert("esxhost".to_string(), system.to_string());
        // {instance: Row}
        let mut per_instance = HashMap::new();
        match AvailableCountersRequest::new(&cmd.soapclient, &cmd.args).await {
            Ok(counters) => {
                info!("got available counters");
                let mut counter_input = Vec::new();
                let mut counter_ids = Vec::new();
                for counter in
                    counters.body.query_available_perf_metric_response.returnval
                {
                    if let Some(instance) = counter.instance.data {
                        let id = &counter.counter_id.data;
                        counter_ids.push(format!(
                            "<ns1:counterId>{}</ns1:counterId>",
                            &id
                        ));
                        counter_input.push(format!("<ns1:metricId><ns1:counterId>{}</ns1:counterId><ns1:instance>{}</ns1:instance></ns1:metricId>", &id, &instance));
                    }
                }
                cmd.args
                    .insert("counterids".to_string(), counter_ids.join("\n"));
                cmd.args
                    .insert("counters".to_string(), counter_input.join("\n"));

                match PerfCounterSyntaxRequest::new(&cmd.soapclient, &cmd.args)
                    .await
                {
                    Ok(syntax) => {
                        info!("got counter syntax");
                        match PerfCounterDataRequest::new(
                            &cmd.soapclient,
                            &cmd.args,
                        )
                        .await
                        {
                            Ok(counters) => {
                                info!("got counter data");
                                let syntax = syntax
                                    .body
                                    .query_perf_counter_response
                                    .returnval;
                                for (fieldid, param) in &cmd.fields {
                                    match param.as_str() {
                                        "instance" => {
                                            set_instance = Some(fieldid)
                                        }
                                        "hostname" => set_host = Some(fieldid),
                                        "name" => set_hostname = Some(fieldid),
                                        _ => {
                                            if let Some(s) =
                                                param.split('.').next()
                                            {
                                                if s != cmd.command_line {
                                                    continue;
                                                }
                                            }
                                            for syn in &syntax {
                                                let id = format!(
                                                    "{}.{}",
                                                    syn.group_info.key.data,
                                                    syn.name_info.key.data
                                                );

                                                if id == param.clone() {
                                                    if let Some(perfdata) =
                                                        counters
                                                            .perfcounters
                                                            .get(
                                                                syn.key
                                                                    .data
                                                                    .to_string()
                                                                    .as_str(),
                                                            )
                                                    {
                                                        for perf in perfdata {
                                                            if !per_instance
                                                                .contains_key(
                                                                &perf.instance,
                                                            ) {
                                                                per_instance.insert(
                                                                    perf.instance.clone(),
                                                                    HashMap::new(),
                                                                );
                                                            }
                                                            per_instance
                                                                .get_mut(&perf.instance)
                                                                .unwrap()
                                                                .insert(fieldid.clone(), {
                                                                    let unit_label = syn.unit_info
                                                                        .label
                                                                        .data.as_deref()
                                                                        .ok_or("")
                                                                        .unwrap();

                                                                    if unit_label == "%" || unit_label.to_lowercase().contains("percent") {
                                                                        // 10000 == 100%
                                                                        Ok(Value::Float(
                                                                            perf.value
                                                                                / 100.0,
                                                                        ))
                                                                    } else if syn.stats_type
                                                                        == StatsType::Delta
                                                                    {
                                                                        // needs to be devided by sample interval
                                                                        Ok(Value::Float(
                                                                            perf.value
                                                                                / 20.0,
                                                                        ))
                                                                    } else {
                                                                        Ok(Value::Float(
                                                                            perf.value,
                                                                        ))
                                                                    }
                                                                });
                                                        }
                                                    }
                                                    break;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                return (
                                    cmd.table,
                                    Err(DTError::SoapError(e).to_api()),
                                )
                            }
                        }
                    }
                    Err(e) => {
                        return (cmd.table, Err(DTError::SoapError(e).to_api()))
                    }
                }
            }
            Err(e) => return (cmd.table, Err(DTError::SoapError(e).to_api())),
        };

        if set_instance.is_some()
            || set_host.is_some()
            || set_hostname.is_some()
        {
            for (instance, row) in per_instance.iter_mut() {
                if let Some(id) = set_instance {
                    row.insert(
                        id.clone(),
                        Ok(Value::UnicodeString(instance.to_string())),
                    );
                }
                if let Some(id) = set_host {
                    row.insert(
                        id.clone(),
                        Ok(Value::UnicodeString(system.to_string())),
                    );
                }
                if let Some(id) = set_hostname {
                    row.insert(
                        id.clone(),
                        Ok(Value::UnicodeString(name.to_string())),
                    );
                }
            }
        }
        data.extend(per_instance.into_values());
    }

    (
        cmd.table,
        Ok(Annotated {
            value: data,
            warnings: Vec::new(),
        }),
    )
}

async fn get_hostdetails(cmd: Command) -> (ProtoDataTableId, TableData) {
    match ESXHostDetailsRequest::new(&cmd.soapclient, &cmd.args).await {
        Err(e) => (cmd.table, Err(DTError::SoapError(e).to_api())),
        Ok(response) => {
            // TODO: REFACTOR, for obvious reasons.....
            // create an dictobj trait or work with hashmaps?
            // cache the response from the api
            let mut rows = Vec::with_capacity(response.hosts.len());
            info!("found {} host details", response.hosts.len());
            for host in response.hosts {
                if cmd.command_line == "get_hostdetails" {
                    let mut row = HashMap::new();
                    for (fieldid, param) in &cmd.fields {
                        row.insert(
                            fieldid.clone(),
                            match param.as_str() {
                                "name" => Ok(Value::UnicodeString(
                                    host.name.to_string(),
                                )),
                                "overall_status" => EnumValue::new(
                                    Arc::new(
                                        vec!["green", "yellow", "red", "gray"]
                                            .into_iter()
                                            .map(|s| s.to_string())
                                            .collect(),
                                    ),
                                    host.overall_status.clone(),
                                )
                                .map(Value::Enum),
                                "total_memory" => {
                                    Ok(Value::Integer(host.total_memory))
                                }
                                "memory_usage" => {
                                    Ok(Value::Integer(host.memory_usage))
                                }
                                _ => Err(DataError::InvalidChoice(
                                    fieldid.to_string(),
                                )),
                            },
                        );
                    }
                    rows.push(row);
                } else if cmd.command_line == "cpu" {
                    for cpu in &host.cpu {
                        let mut row = HashMap::new();
                        for (fieldid, param) in &cmd.fields {
                            row.insert(
                                fieldid.clone(),
                                match param.as_str() {
                                    "index" => Ok(Value::Integer(cpu.index)),
                                    "hz" => Ok(Value::Integer(cpu.hz)),
                                    "bus_hz" => Ok(Value::Integer(cpu.bus_hz)),
                                    "description" => Ok(Value::UnicodeString(
                                        cpu.description.to_string(),
                                    )),
                                    "hostname" => Ok(Value::UnicodeString(
                                        host.name.to_string(),
                                    )),
                                    _ => Err(DataError::InvalidChoice(
                                        fieldid.to_string(),
                                    )),
                                },
                            );
                        }
                        rows.push(row);
                    }
                } else if cmd.command_line == "sensor" {
                    for sensor in &host.sensors {
                        let mut row = HashMap::new();
                        for (fieldid, param) in &cmd.fields {
                            row.insert(
                                fieldid.clone(),
                                match param.as_str() {
                                    "name" => Ok(Value::UnicodeString(
                                        sensor.name.to_string(),
                                    )),
                                    "label" => Ok(Value::UnicodeString(
                                        sensor.label.to_string(),
                                    )),
                                    "summary" => Ok(Value::UnicodeString(
                                        sensor.summary.to_string(),
                                    )),
                                    "key" => EnumValue::new(
                                        Arc::new(
                                            vec![
                                                "green", "yellow", "red",
                                                "gray",
                                            ]
                                            .into_iter()
                                            .map(|s| s.to_string())
                                            .collect(),
                                        ),
                                        sensor.key.clone(),
                                    )
                                    .map(Value::Enum),
                                    "sensor_type" => Ok(Value::UnicodeString(
                                        sensor.sensor_type.to_string(),
                                    )),
                                    "hostname" => Ok(Value::UnicodeString(
                                        host.name.to_string(),
                                    )),
                                    _ => Err(DataError::InvalidChoice(
                                        fieldid.to_string(),
                                    )),
                                },
                            );
                        }
                        rows.push(row);
                    }
                } else if cmd.command_line == "luns" {
                    for lun in &host.luns {
                        let mut row = HashMap::new();
                        for (fieldid, param) in &cmd.fields {
                            row.insert(
                                fieldid.clone(),
                                match param.as_str() {
                                    "key" => Ok(Value::UnicodeString(
                                        lun.key.to_string(),
                                    )),
                                    "id" => Ok(Value::UnicodeString(
                                        lun.id.to_string(),
                                    )),
                                    "lun" => Ok(Value::UnicodeString(
                                        lun.lun.to_string(),
                                    )),
                                    "hostname" => Ok(Value::UnicodeString(
                                        host.name.to_string(),
                                    )),
                                    _ => Err(DataError::InvalidChoice(
                                        fieldid.to_string(),
                                    )),
                                },
                            );
                        }
                        rows.push(row);
                    }
                } else if cmd.command_line == "paths" {
                    for lun in &host.luns {
                        for path in &lun.paths {
                            let mut row = HashMap::new();
                            for (fieldid, param) in &cmd.fields {
                                row.insert(
                                    fieldid.clone(),
                                    match param.as_str() {
                                        "key" => Ok(Value::UnicodeString(
                                            path.key.to_string(),
                                        )),
                                        "name" => Ok(Value::UnicodeString(
                                            path.name.to_string(),
                                        )),
                                        "path_state" => {
                                            Ok(Value::UnicodeString(
                                                path.path_state.to_string(),
                                            ))
                                        }
                                        "state" => Ok(Value::UnicodeString(
                                            path.state.to_string(),
                                        )),
                                        "is_working_path" => {
                                            Ok(Value::Boolean(
                                                path.is_working_path,
                                            ))
                                        }
                                        "lun" => Ok(Value::UnicodeString(
                                            lun.id.to_string(),
                                        )),
                                        "hostname" => Ok(Value::UnicodeString(
                                            host.name.to_string(),
                                        )),
                                        _ => Err(DataError::InvalidChoice(
                                            fieldid.to_string(),
                                        )),
                                    },
                                );
                            }
                            rows.push(row);
                        }
                    }
                }
            }
            (
                cmd.table,
                Ok(Annotated {
                    value: rows,
                    warnings: Vec::new(),
                }),
            )
        }
    }
}

async fn get_vms(cmd: Command) -> (ProtoDataTableId, TableData) {
    match VmDetailsRequest::new(&cmd.soapclient, &cmd.args).await {
        Err(e) => (cmd.table, Err(DTError::SoapError(e).to_api())),
        Ok(response) => {
            let mut rows = Vec::with_capacity(response.vms.len());
            info!(
                "found {} vms and {} datastores",
                response.vms.len(),
                response.datastores.len()
            );

            if cmd.command_line == "vms" || cmd.command_line == "get_vms" {
                let hosts = match LivestatusSocket::new().await {
                    Err(_) => {
                        Err(DataError::TypeError("Not an OMD site".to_string()))
                    }
                    Ok(mut ls) => match ls.get_hosts().await {
                        Ok(hosts) => Ok(hosts),
                        Err(e) => Err(DataError::TypeError(format!(
                            "Unable to retrieve hosts from livestatus: {:?}",
                            e
                        ))),
                    },
                };

                for vm in response.vms {
                    rows.push(cmd.fields.iter().map(|(fieldid, param)| (
                        fieldid.clone(),
                        match param.as_str() {
                            "monitored" => match hosts {
                                Err(ref e) => Err(e.clone()),
                                Ok(ref set) => {
                                    match vm.get("summary.guest.hostName") {
										Some(Ok(Value::UnicodeString(fqdn))) => {
											Ok(Value::Boolean(set.contains(fqdn)))
										},
										Some(Ok(_)) => Err(DataError::TypeError("wrong type".to_string())),
										Some(Err(e)) => Err(e.clone()),
										None => {
											Err(DataError::Missing)
										}
									}
                                }
                            },
                            _ => vm.get(param.as_str()).cloned().ok_or(
                                DataError::Missing).and_then(|v| v)
                        }
                    )).collect());
                }
            } else if cmd.command_line == "datastores" {
                for (vm, datastores) in response.datastores {
                    for datastore in datastores {
                        rows.push(
                            cmd.fields
                                .iter()
                                .map(|(fieldid, param)| {
                                    (
                                        fieldid.clone(),
                                        match param.as_str() {
                                            "vm" => Ok(Value::UnicodeString(
                                                vm.to_string(),
                                            )),
                                            "datastore" => {
                                                datastore.datastore.clone()
                                            }
                                            "committed" => {
                                                datastore.committed.clone()
                                            }
                                            "uncommitted" => {
                                                datastore.uncommitted.clone()
                                            }
                                            "unshared" => {
                                                datastore.unshared.clone()
                                            }
                                            _ => Err(DataError::Missing),
                                        },
                                    )
                                })
                                .collect(),
                        );
                    }
                }
            }
            (
                cmd.table,
                Ok(Annotated {
                    value: rows,
                    warnings: Vec::new(),
                }),
            )
        }
    }
}
