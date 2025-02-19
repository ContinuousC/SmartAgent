/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::collections::HashMap;
use std::convert::TryFrom;
use std::io::BufReader;
use std::path::PathBuf;
use std::str::FromStr;
use std::fs::File;
use std::{fmt,process};
use std::ffi::c_void;

//use clap::{Arg, App, ArgMatches, AppSettings,SubCommand};
use serde::{Serialize,Deserialize};
use chrono::{DateTime,Utc};
use netsnmp::{TransportPtr,Usm,CallbackOp,Version,
	      MultiSessionPtr,PduPtr,
	      Msg,Oid,SyncQuery,SessionInfo};
use agent::elastic::write_events;
//use agent::agent_utils::quote_filename;


#[derive(Serialize,Deserialize,Clone,Debug)]
struct Config {
    data_dir: PathBuf,
    user: Option<String>,
    group: Option<String>,
    instances: Vec<String>,
    snmp: Option<SNMPConfig>,
}


#[derive(Serialize,Deserialize,Clone,Debug)]
struct SNMPConfig {
    listen: Vec<String>,
    communities: Option<Vec<String>>,
    users: Option<HashMap<String, SNMPUser>>,
}

#[derive(Serialize,Deserialize,Clone,Debug)]
struct SNMPUser {
    engine_id: Vec<u8>,
    auth: netsnmp::V3Level
}

#[derive(Debug)]
enum SNMPError {
    Transport(String, netsnmp::Error),
    Session(netsnmp::Error),
    User(String, netsnmp::Error)
}

#[derive(Serialize,Deserialize,Clone,Debug)]
struct SNMPEvent {
    #[serde(rename = "@timestamp")]
    timestamp: DateTime<Utc>,
    hostname: String,
    transport: String,
    oid: Oid,
    variables: HashMap<Oid, Result<netsnmp::Value,netsnmp::ErrType>>
}

struct State<'a> {
    config: &'a Config,
    snmp_config: &'a SNMPConfig,
    transport: *mut netsnmp::api::netsnmp_transport
}


fn main() {

    let config : Config = serde_json::from_reader(BufReader::new(
	File::open("event_receiver.json").expect("Failed to open config"))
    ).expect("Failed to decode config!");


    if let Some(snmp_config) = &config.snmp {
	if let Err(err) = snmp_event_receiver(&config, snmp_config) {
	    eprintln!("Error: {}", err);
	    process::exit(1);
	}
    }

}


fn snmp_event_receiver(config: &Config, snmp_config: &SNMPConfig) -> Result<(),SNMPError> {

    let snmp = netsnmp::init("SmartM Event Receiver");
    //snmp.set_debug(true);

    let _usm = match &snmp_config.users {
	Some(users) => {
	    let mut usm = Usm::init();
	    for (name,user) in users {
		usm.add_user(
		    usm.create_user()
			.set_name(name).map_err(
			    |e| SNMPError::User(name.to_string(), e))?
			.set_engine_id(&user.engine_id)
			.set_auth(&user.auth).map_err(
			    |e| SNMPError::User(name.to_string(), e))?
		).map_err(|e| SNMPError::User(name.to_string(), e))?;
	    }
	    Some(usm)
	},
	None => None
    };


    let mut sessions = Vec::new();
    let mut states = Vec::new();

    for ep in &snmp_config.listen {

	let mut transport = snmp.server_transport("SmartM Event Receiver", ep)
	    .map_err(|e| SNMPError::Transport(ep.to_string(), e))?;

	let state = Box::into_raw(Box::new(State {
	    config: &config,
	    snmp_config: &snmp_config,
	    transport: transport.as_mut_ptr()
	}));

	unsafe {
	    states.push(Box::from_raw(state));
	}
	    
	let (session,_) = snmp.session()
	    .set_callback_static(event_callback, state as *mut c_void)
	    .open_with_transport(transport)
	    .map_err(|e| SNMPError::Session(e))?;

	sessions.push(session);

    }

    /* Main loop. This will need to be changed when other event protocols are
     * added, eg. by putting this in a separate thread, by adding other fds (but
     * netsnmp is not kind enough to let us know its fds) or by using a non-blocking
     * function. */
    
    loop {
	snmp.read();
    }

}

extern "C" fn event_callback(op: i32, session: *mut netsnmp::api::snmp_session, _x: i32,
			     pdu: *mut netsnmp::api::snmp_pdu,
			     magic: *mut std::ffi::c_void) -> i32 {

    let state: &mut State = unsafe {
	&mut *(magic as *mut State)
    };

    let pdu = unsafe { netsnmp::PduPtr::from_ptr(pdu) };
    let session = unsafe { netsnmp::MultiSessionPtr::from_mut(session) };
    let transport = unsafe { netsnmp::TransportPtr::from_ptr(state.transport) };
		    
    if session.has_error() {
	eprintln!("Received packet with error; discarding!");
	return 1;
    }

    match CallbackOp::try_from(op) {
	Ok(CallbackOp::ReceivedMessage) => {
	    if let Err(e) = handle_snmp_notification(state.config, state.snmp_config,
						     transport, session, pdu) {
		eprintln!("Error while handling trap: {}", e);
	    }
	},
	_ => {
	    eprintln!("Unrecognised callback operation: {:?}", op);
	}
    }
    1

}


fn handle_snmp_notification(config: &Config, snmp_config: &SNMPConfig,
			    transport: &mut TransportPtr,
			    session: &mut MultiSessionPtr,
			    pdu: &PduPtr) -> netsnmp::Result<()> {

    let v1_generic_trap_type = Oid::from_str("1.3.6.1.6.3.1.1.5")?;
    let trap_type_oid = Oid::from_str("1.3.6.1.6.3.1.1.4.1.0")?;
    
    match pdu.version()? {
	Version::V1 | Version::V2c => {
	    let authenticated = match &snmp_config.communities {
		Some(communities) => communities.contains(&pdu.community()?),
		None => false
	    };
	    match authenticated {
		true => Ok(()),
		false => Err(netsnmp::Error::General(
		    format!("V1/2c authentication failed!")))
	    }
	},
	Version::V3 => {
	    /* Authentication and privacy have already been handled via the USM subsystem. */
	    Ok(())
	}
    }?;

    let (oid,variables) = match pdu.command()? {
	Msg::Trap => Ok((
	    match pdu.trap_type() {
		6 => pdu.enterprise().join(vec![0, pdu.specific_type()]),
		t => v1_generic_trap_type.join(vec![t+1])
	    },
	    pdu.variables().into_iter().map(
		|var| (var.get_name(), var.get_value())
	    ).collect()
	)),
	Msg::Trap2 | Msg::Inform => Ok((
	    match pdu.variables().into_iter().filter(|var| var.get_name() == trap_type_oid)
		.next().map(|var| var.get_value()) {
		    Some(Ok(netsnmp::Value::Oid(oid))) => Ok(oid),
		    _ => Err(netsnmp::Error::General(String::from("Missing trap type!")))
		}?,
	    pdu.variables().into_iter().filter(|var| var.get_name() != trap_type_oid).map(
		|var| (var.get_name(), var.get_value())
	    ).collect()
	)),
	cmd => Err(netsnmp::Error::General(format!("Unsupported command: {:?}", cmd))),
    }?;

    if let Msg::Inform = pdu.command()? {
	let mut response = pdu.to_owned();
	response.set_command(Msg::Response);
	response.clear_error();
	session.send(response)?;
    }
    
    let event = SNMPEvent {
	timestamp: Utc::now(), /* Pdu 'time' seems to be uptime. */
	hostname: transport.format_lookup(pdu.transport_data())
	    .unwrap_or_else(|| String::from("unknown")),
	transport: transport.format_nolookup(pdu.transport_data())
	    .unwrap_or_else(|| String::from("unknown")),
	oid, variables
    };
    

    /* Write to stdout (debug). */
    println!("{}", serde_json::to_string(&event).map_err(|e| netsnmp::Error::General(
	format!("Event serialization failed: {}", e)))?);

    
    /* Write event to file. */
    
    for instance in &config.instances {
	let data_dir = config.data_dir.join(instance);
	write_events(&data_dir, String::from("snmp_events"), vec![&event])
	    .map_err(|e| netsnmp::Error::General(format!("Data writing error: {}", e)))?;
    }

    Ok(())

}


impl fmt::Display for SNMPError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
	match self {
	    Self::Transport(ep, err) => write!(f, "failed to open transport on {}: {}", ep, err),
	    Self::Session(err) => write!(f, "failed to open session: {}", err),
	    Self::User(name, err) => write!(f, "failed to add user {}: {}", name, err),
        }
    }
}
