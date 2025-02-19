/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

mod datastores;
mod hostsystems;
mod licenses;
mod login;
mod pnics;
mod systeminfo;

pub use datastores::DatastoresRequest;
pub use hostsystems::HostsytemsRequest;
pub use licenses::LicensesRequest;
pub use login::LoginRequest;
pub use pnics::PNicRequest;
pub use systeminfo::SysteminfoRequest;

// per host system
mod available_counters;
mod counter_request;
mod counter_syntax;
mod esxhostdetails;
mod vmdetails;

pub use available_counters::AvailableCountersRequest;
pub use counter_request::PerfCounterDataRequest;
pub use counter_syntax::PerfCounterSyntaxRequest;
pub use counter_syntax::StatsType;
pub use esxhostdetails::ESXHostDetailsRequest;
pub use vmdetails::VmDetailsRequest;
