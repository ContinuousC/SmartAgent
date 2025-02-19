/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::{fs,env,process};
use agent::protocols::snmp::parse_snmp_walk;
use agent::error::Error;

fn main() {

    let path = match env::args().skip(1).next() {
	Some(p) => p,
	None => {
	    eprintln!("Usage: test_parse_snmpwalk <FILE>");
	    process::exit(1);
	}
    };

    match parse_snmp_walk(&fs::read(path).expect("I/O Error")) {
	Ok(m) => for (oid,val) in m {
	    println!("{} = {:?}", oid, val)
	},
	Err(Error::Protocol(e)) => {
	    eprintln!("{}", e);
	    process::exit(1);
	},
	Err(e) => {
	    eprintln!("Unexpected error: {:?}", e);
	    process::exit(1);
	}
    }
}
