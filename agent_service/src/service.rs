/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::collections::{HashMap, HashSet};
use std::iter::FromIterator;
use std::net::IpAddr;
use std::sync::Arc;

use async_trait::async_trait;
use futures::StreamExt;
use trust_dns_resolver::TokioAsyncResolver;

use agent_api::{ArpEntry, IpRoute, SnmpTable};
use agent_serde::HumanReadable;
use agent_utils::TryGetFrom;
use etc::{EtcManager, QueryMode};
use etc_base::{
    Annotated, AnnotatedResult, PackageName, PackageVersion, Protocol, TableId,
    Warning,
};
use nmap::{nmap_xml, nping, ping, portscan, traceroute};
use protocol::PluginManager;
use scheduler::Scheduler;

use super::envvar::get_envvar_from_file;
use super::error::{Error, Result};
use super::netlink;

pub struct AgentService {
    dns_resolver: TokioAsyncResolver,
    plugin_manager: Arc<PluginManager>,
    etc_manager: Arc<EtcManager>,
    pub scheduler: Scheduler,
}

impl AgentService {
    pub fn new(
        plugin_manager: Arc<PluginManager>,
        etc_manager: Arc<EtcManager>,
        scheduler: Scheduler,
    ) -> Result<Self> {
        Ok(Self {
            dns_resolver: TokioAsyncResolver::tokio_from_system_conf()?,
            plugin_manager,
            etc_manager,
            scheduler,
        })
    }
}

#[async_trait]
impl agent_api::AgentService for AgentService {
    type Error = Error;

    async fn ping(&self) -> Result<()> {
        Ok(())
    }

    async fn shutdown(&self) -> Result<()> {
        eprintln!("Received shutdown.");
        Ok(())
    }

    async fn config(&self, config: scheduler::Config) -> Result<()> {
        //eprintln!("Received new config: {:?}", config);
        Ok(self.scheduler.update_config(config).await?)
    }

    async fn install(&self, package: String) -> Result<()> {
        eprintln!("Installing package {}...", package);
        Ok(())
    }

    async fn uninstall(&self, package: String) -> Result<()> {
        eprintln!("Uninstalling package {}...", package);
        Ok(())
    }

    /* Local host information. */

    async fn hostname(&self) -> Result<String> {
        Ok(nix::sys::utsname::uname().nodename().to_string())
    }

    async fn host_ips(&self) -> Result<Vec<String>> {
        Ok(nix::ifaddrs::getifaddrs()?
            .filter_map(|interface| {
                interface.address.and_then(|addr| match addr {
                    nix::sys::socket::SockAddr::Inet(inaddr) => {
                        match inaddr.ip() {
                            nix::sys::socket::IpAddr::V4(_) => {
                                Some(format!("{}", inaddr.ip()))
                            }
                            nix::sys::socket::IpAddr::V6(_) => None,
                        }
                    }
                    _ => None,
                })
            })
            .collect())
    }

    async fn vendor(&self) -> Result<String> {
        get_envvar_from_file("/etc/os-release", "NAME").await
    }

    async fn os(&self) -> Result<String> {
        get_envvar_from_file("/etc/os-release", "PRETTY_NAME").await
    }

    async fn ip_routes(&self) -> Result<Vec<IpRoute>> {
        netlink::get_routes().await
    }

    async fn arp_cache(&self) -> Result<Vec<ArpEntry>> {
        netlink::get_neighbours().await
    }

    async fn dns_lookup(
        &self,
        host: String,
    ) -> Result<Vec<HumanReadable<IpAddr>>> {
        Ok(self
            .dns_resolver
            .ipv4_lookup(host.to_string())
            .await?
            .into_iter()
            .map(|addr| HumanReadable(IpAddr::V4(addr)))
            .collect())
    }

    async fn dns_lookups(
        &self,
        hosts: HashSet<String>,
    ) -> Result<
        HashMap<
            String,
            std::result::Result<Vec<HumanReadable<IpAddr>>, String>,
        >,
    > {
        Ok(futures::stream::iter(hosts)
            .map(|host| async move {
                (
                    host.to_string(),
                    self.dns_lookup(host).await.map_err(|e| e.to_string()),
                )
            })
            .buffer_unordered(10)
            .collect()
            .await)
    }

    async fn reverse_dns_lookup(
        &self,
        addr: HumanReadable<IpAddr>,
    ) -> Result<Vec<String>> {
        Ok(self
            .dns_resolver
            .reverse_lookup(addr.0)
            .await?
            .into_iter()
            .map(|name| name.to_utf8())
            .collect())
    }

    async fn reverse_dns_lookups(
        &self,
        addrs: HashSet<HumanReadable<IpAddr>>,
    ) -> Result<
        HashMap<
            HumanReadable<IpAddr>,
            std::result::Result<Vec<String>, String>,
        >,
    > {
        Ok(futures::stream::iter(addrs)
            .map(|addr| async move {
                (
                    addr.clone(),
                    self.reverse_dns_lookup(addr)
                        .await
                        .map_err(|e| e.to_string()),
                )
            })
            .buffer_unordered(10)
            .collect()
            .await)
    }

    async fn ping_hosts(
        &self,
        hosts: Vec<String>,
    ) -> Result<HashMap<String, ping::PingStats>> {
        Ok(ping::ping(hosts).await?)
    }

    async fn traceroute(
        &self,
        hosts: Vec<String>,
    ) -> Result<HashMap<String, traceroute::Traceroute>> {
        Ok(traceroute::traceroute(hosts).await?)
    }

    async fn snmp_get_table(
        &self,
        host: HumanReadable<IpAddr>,
        config: snmp_protocol::HostConfig,
        entry: netsnmp::Oid,
        cols: HashSet<netsnmp::Oid>,
    ) -> Result<SnmpTable> {
        let context = match &config.auth {
            Some(netsnmp::Auth::V3(auth)) => {
                auth.context.as_deref().unwrap_or("")
            }
            _ => "",
        }
        .to_string();
        let config = snmp_protocol::Config {
            host_name: host.0.to_string(),
            ip_addr: Some(host.0),
            host_config: config,
        };

        let stats = parking_lot::Mutex::new(snmp_protocol::Stats::new());
        let mut walks = snmp_protocol::Walks::new();
        let mut table = snmp_protocol::WalkTable::new();
        let gets = snmp_protocol::Gets::new();

        for col in &cols {
            table.push(
                snmp_protocol::WalkVar::new(
                    entry.join(col.as_slice().to_vec()),
                ),
                None,
            );
        }

        walks.push(table);

        let queries = HashMap::from_iter([(context, (walks, gets))]);
        let r = self
            .plugin_manager
            .get_local_plugin::<snmp_protocol::Plugin>()?
            .get_raw_table(&config, queries, &stats)
            .await?;

        Ok(r.into_iter()
            .map(|(oid, walk)| {
                (
                    oid,
                    match walk {
                        Ok(Annotated {
                            value: vals,
                            warnings,
                        }) => Ok(Annotated {
                            value: vals
                                .into_iter()
                                .map(|(idx, val)| {
                                    (idx, val.map_err(|e| format!("{:?}", e)))
                                })
                                .collect(),
                            warnings: warnings
                                .into_iter()
                                .map(|Warning { verbosity, message }| Warning {
                                    verbosity,
                                    message: message.to_string(),
                                })
                                .collect(),
                        }),
                        Err(err) => Err(err.to_string()),
                    },
                )
            })
            .collect())
    }

    async fn vmware_get_managed_entities(
        &self,
        host: String,
        config: api_protocol::vmware::cc_config::EssentialConfig,
        entity_type: String,
        properties: HashSet<String>,
    ) -> Result<
        HashMap<String, HashMap<String, Option<api_protocol::vmware::Value>>>,
    > {
        api_protocol::vmware::get_managed_entities(
            host,
            config,
            entity_type,
            properties,
        )
        .await
        .map_err(Error::VmWareProto)
    }

    async fn msgraph_list_organizations(
        &self,
        client_info: api_protocol::ms_graph::Credentials,
    ) -> Result<Vec<api_protocol::ms_graph::Organization>> {
        let client = client_info.login(None).await.map_err(|e| {
            protocol::Error::Plugin(
                Protocol(String::from("Azure")),
                Box::new(e),
            )
        })?;
        Ok(api_protocol::ms_graph::requests::get_object(
            &client,
            "organization",
        )
        .await
        .map_err(|e| {
            protocol::Error::Plugin(
                Protocol(String::from("Azure")),
                Box::new(e),
            )
        })?)
    }

    async fn azure_list_tentants(
        &self,
        client_info: azure_protocol::ClientInfo,
    ) -> Result<Vec<azure_protocol::Tenant>> {
        let client = client_info.login(None).await.map_err(|e| {
            protocol::Error::Plugin(
                Protocol(String::from("Azure")),
                Box::new(e),
            )
        })?;
        Ok(
            azure_protocol::request_resource(&client, "tenants", "2020-01-01")
                .await
                .map_err(|e| {
                    protocol::Error::Plugin(
                        Protocol(String::from("Azure")),
                        Box::new(e),
                    )
                })?,
        )
    }
    async fn azure_list_subscriptions(
        &self,
        client_info: azure_protocol::ClientInfo,
    ) -> Result<Vec<azure_protocol::Subscription>> {
        let client = client_info.login(None).await.map_err(|e| {
            protocol::Error::Plugin(
                Protocol(String::from("Azure")),
                Box::new(e),
            )
        })?;
        Ok(azure_protocol::request_resource(
            &client,
            "subscriptions",
            "2020-01-01",
        )
        .await
        .map_err(|e| {
            protocol::Error::Plugin(
                Protocol(String::from("Azure")),
                Box::new(e),
            )
        })?)
    }
    async fn azure_list_resourcegroups(
        &self,
        client_info: azure_protocol::ClientInfo,
        subscriptions: Vec<azure_protocol::SubscriptionId>,
    ) -> Result<
        HashMap<
            azure_protocol::SubscriptionId,
            std::result::Result<Vec<azure_protocol::ResourceGroup>, String>,
        >,
    > {
        let client = client_info.login(None).await.map_err(|e| {
            protocol::Error::Plugin(
                Protocol(String::from("Azure")),
                Box::new(e),
            )
        })?;
        let requests = subscriptions
            .iter()
            .map(|s| {
                azure_protocol::request_resource_from_subscription(
                    &client,
                    s,
                    "resourcegroups",
                    "2021-04-01",
                )
            })
            .collect::<Vec<_>>();
        let futures = futures::stream::iter(requests)
            .buffered(10)
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .map(|res| res.map_err(|e| e.to_string()))
            .collect::<Vec<_>>();
        Ok(subscriptions.into_iter().zip(futures).collect())
    }
    async fn azure_list_resources(
        &self,
        client_info: azure_protocol::ClientInfo,
        subscriptions: Vec<azure_protocol::SubscriptionId>,
    ) -> Result<
        HashMap<
            azure_protocol::SubscriptionId,
            std::result::Result<
                HashMap<
                    azure_protocol::ResourceGroupName,
                    HashSet<azure_protocol::Resource>,
                >,
                String,
            >,
        >,
    > {
        let client = client_info.login(None).await.map_err(|e| {
            protocol::Error::Plugin(
                Protocol(String::from("Azure")),
                Box::new(e),
            )
        })?;

        let requests = subscriptions
            .iter()
            .map(|s| {
                azure_protocol::request_resource_from_subscription(
                    &client,
                    s,
                    "resources",
                    "2021-04-01",
                )
            })
            .collect::<Vec<_>>();

        let futures = futures::stream::iter(requests)
            .buffered(10)
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .map(|res| res.map_err(|e| e.to_string())
                .map(|resources| resources.into_iter()
                    .fold(HashMap::new(), |
                            mut accum: HashMap<String, HashSet<azure_protocol::Resource>>,
                            elem: azure_protocol::Resource
                        | {
                        accum.entry(elem.get_resource_group().to_lowercase())
                            .or_default()
                            .insert(elem);
                        accum
                    })
                )
            )
            .collect::<Vec<_>>();

        Ok(subscriptions.into_iter().zip(futures).collect())
    }

    async fn load_pkg(
        &self,
        name: PackageName,
        version: PackageVersion,
        spec: String,
    ) -> Result<()> {
        Ok(self
            .etc_manager
            .load_pkg(name, version, spec, &self.plugin_manager)
            .await?)
    }

    async fn unload_pkg(&self, name: PackageName) -> Result<()> {
        Ok(self
            .etc_manager
            .unload_pkg(name, &self.plugin_manager)
            .await?)
    }

    async fn loaded_pkgs(
        &self,
    ) -> Result<HashMap<PackageName, PackageVersion>> {
        Ok(self.etc_manager.loaded_pkgs().await?)
    }

    async fn get_etc_tables(
        &self,
        table_ids: HashSet<TableId>,
        config: HashMap<Protocol, serde_json::Value>,
        query_mode: QueryMode,
    ) -> Result<
        HashMap<
            String,
            AnnotatedResult<
                Vec<
                    HashMap<
                        String,
                        std::result::Result<serde_json::Value, String>,
                    >,
                >,
                String,
                String,
            >,
        >,
    > {
        let config = config
            .into_iter()
            .map(|(k, v)| Ok((k, serde_json::value::to_raw_value(&v)?)))
            .collect::<Result<_>>()?;
        let spec = self.etc_manager.spec().await;
        let prot_queries = spec.queries_for(&table_ids, query_mode)?;
        let data = self
            .plugin_manager
            .run_queries(&spec.input, config, &prot_queries)
            .await?;
        let tables: HashMap<_, _> = table_ids
            .iter()
            .map(|table_id| {
                Ok((
                    table_id.clone(),
                    table_id
                        .try_get_from(&spec.etc.tables)?
                        .calculate(query_mode, &spec.etc, &data)?,
                ))
            })
            .collect::<Result<_>>()?;

        tables
            .into_iter()
            .map(|(table_id, res)| {
                let elastic_index: String = table_id
                    .try_get_from(&spec.etc.tables)?
                    .elastic_index
                    .as_deref()
                    .ok_or_else(|| {
                        Error::MissingElasticIndex(table_id.0.to_string())
                    })?
                    .to_string();

                Ok((
                    elastic_index,
                    match res {
                        Ok(Annotated {
                            value: rows,
                            warnings,
                        }) => Ok(Annotated {
                            value: rows
                                .into_iter()
                                .map(|row| {
                                    row.into_iter()
                                        .filter_map(|(k, v)| {
                                            let field = match k
                                                .try_get_from(&spec.etc.fields)
                                            {
                                                Ok(v) => v,
                                                Err(e) => return Some(Err(e.into())),
                                            };
                                            if !field.elastic_data {
                                                return None;
                                            }
                                            let elastic_field: String = match &field
                                                .elastic_field {
													Some(v) => v.to_string(),
													None => return Some(Err(Error::MissingElasticField(
                                                        k.to_string(),
                                                    )))
												};
                                            Some(Ok((
                                                elastic_field,
                                                v.map_err(|e| e.to_string())
                                                    .and_then(|v| {
                                                        v.to_json_value()
                                                            .ok_or_else(|| {
                                                                "unserializable"
                                                                    .to_string()
                                                            })
                                                    }),
                                            )))
                                        })
                                        .collect()
                                })
                                .collect::<Result<_>>()?,
                            warnings: warnings
                                .into_iter()
                                .map(|Warning { verbosity, message }| Warning {
                                    verbosity,
                                    message: message.to_string(),
                                })
                                .collect(),
                        }),
                        Err(e) => Err(e.to_string()),
                    },
                ))
            })
            .collect()
    }

    async fn nping_host(
        &self,
        host: String,
        mode: nping::NPingMode,
    ) -> Result<nping::NPingStats> {
        Ok(nping::nping_host(host.as_str(), mode).await?)
    }

    async fn port_scan(
        &self,
        hosts: Vec<String>,
        ports: Vec<portscan::Port>,
    ) -> Result<HashMap<String, HashMap<portscan::Port, nmap_xml::PortState>>>
    {
        Ok(portscan::portscan(hosts, ports).await?)
    }
}
