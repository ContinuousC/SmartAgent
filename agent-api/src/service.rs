/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::collections::{HashMap, HashSet};
use std::net::IpAddr;

use serde::{Deserialize, Serialize};

use rpc::rpc;

use agent_serde::HumanReadable;
use etc::QueryMode;
use etc_base::{
    AnnotatedResult, PackageName, PackageVersion, Protocol, TableId,
};
use nmap::nmap_xml::PortState;
use nmap::nping::{NPingMode, NPingStats};
use nmap::ping::PingStats;
use nmap::portscan::Port;
use nmap::traceroute::Traceroute;

use broker_api::AgentId;

#[rpc(service, stub(javascript, python, extra_args = "agent_id: AgentId"))]
pub trait AgentService {
    async fn ping(&self);
    async fn shutdown(&self);
    async fn config(&self, config: scheduler::Config);
    async fn install(&self, pkg: String);
    async fn uninstall(&self, pkg: String);
    //async fn get_table_from_host(&self, host: String, table: TableId) -> Result<TableData>;

    async fn hostname(&self) -> String;
    async fn host_ips(&self) -> Vec<String>;
    async fn vendor(&self) -> String;
    async fn os(&self) -> String;
    async fn ip_routes(&self) -> Vec<IpRoute>;
    async fn arp_cache(&self) -> Vec<ArpEntry>;

    async fn dns_lookup(&self, name: String) -> Vec<HumanReadable<IpAddr>>;
    async fn dns_lookups(
        &self,
        names: HashSet<String>,
    ) -> HashMap<String, Result<Vec<HumanReadable<IpAddr>>, String>>;

    async fn reverse_dns_lookup(
        &self,
        ip: HumanReadable<IpAddr>,
    ) -> Vec<String>;
    async fn reverse_dns_lookups(
        &self,
        ips: HashSet<HumanReadable<IpAddr>>,
    ) -> HashMap<HumanReadable<IpAddr>, Result<Vec<String>, String>>;

    async fn ping_hosts(
        &self,
        hosts: Vec<String>,
    ) -> HashMap<String, PingStats>;
    async fn traceroute(
        &self,
        hosts: Vec<String>,
    ) -> HashMap<String, Traceroute>;

    async fn snmp_get_table(
        &self,
        host: HumanReadable<IpAddr>,
        config: snmp_protocol::HostConfig,
        entry: netsnmp::Oid,
        cols: HashSet<netsnmp::Oid>,
    ) -> SnmpTable;
    /*async fn snmp_get_next(&self, host: IpAddr, creds: netsnmp::Auth, oid: Oid)
    -> Result<netsnmp::Value>;*/

    async fn vmware_get_managed_entities(
        &self,
        host: String,
        config: api_protocol::vmware::cc_config::EssentialConfig,
        entity_type: String,
        properties: HashSet<String>,
    ) -> HashMap<String, HashMap<String, Option<api_protocol::vmware::Value>>>;

    async fn msgraph_list_organizations(
        &self,
        client_info: api_protocol::ms_graph::Credentials,
    ) -> Vec<api_protocol::ms_graph::Organization>;
    async fn azure_list_tentants(
        &self,
        client_info: azure_protocol::ClientInfo,
    ) -> Vec<azure_protocol::Tenant>;
    async fn azure_list_subscriptions(
        &self,
        client_info: azure_protocol::ClientInfo,
    ) -> Vec<azure_protocol::Subscription>;
    async fn azure_list_resourcegroups(
        &self,
        client_info: azure_protocol::ClientInfo,
        subscriptions: Vec<azure_protocol::SubscriptionId>,
    ) -> HashMap<
        azure_protocol::SubscriptionId,
        Result<Vec<azure_protocol::ResourceGroup>, String>,
    >;
    async fn azure_list_resources(
        &self,
        client_info: azure_protocol::ClientInfo,
        subscriptions: Vec<azure_protocol::SubscriptionId>,
    ) -> HashMap<
        azure_protocol::SubscriptionId,
        std::result::Result<
            HashMap<
                azure_protocol::ResourceGroupName,
                HashSet<azure_protocol::Resource>,
            >,
            String,
        >,
    >;

    async fn nping_host(&self, host: String, mode: NPingMode) -> NPingStats;

    async fn port_scan(
        &self,
        hosts: Vec<String>,
        ports: Vec<Port>,
    ) -> HashMap<String, HashMap<Port, PortState>>;

    async fn load_pkg(
        &self,
        name: PackageName,
        version: PackageVersion,
        spec: String,
    );
    async fn unload_pkg(&self, name: PackageName);
    async fn loaded_pkgs(&self) -> HashMap<PackageName, PackageVersion>;

    async fn get_etc_tables(
        &self,
        id: HashSet<TableId>,
        config: HashMap<Protocol, serde_json::Value>,
        query_mode: QueryMode,
    ) -> HashMap<
        String,
        AnnotatedResult<
            Vec<
                HashMap<String, std::result::Result<serde_json::Value, String>>,
            >,
            String,
            String,
        >,
    >;
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case")]
pub enum AgentEvent {
    Connected,
    Disconnected,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpRoute {
    pub table: String,
    pub dev: String,
    pub proto: String,
    pub scope: String,
    pub via: Option<String>,
    pub src: Option<String>,
    pub dst: Option<String>,
    pub metric: Option<u32>,
    pub up: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArpEntry {
    pub ip: Option<String>,
    pub mac: Option<String>,
    pub vlan: Option<u16>,
    pub dev: Option<String>,
    pub state: String,
    pub ntype: String,
    pub flags: HashSet<String>,
}

pub type SnmpTable = HashMap<
    netsnmp::Oid,
    AnnotatedResult<
        HashMap<netsnmp::Oid, std::result::Result<netsnmp::Value, String>>,
        String,
        String,
    >,
>;
