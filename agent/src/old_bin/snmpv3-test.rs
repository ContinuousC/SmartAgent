/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::str::FromStr;
use netsnmp::Oid;

#[tokio::main]
async fn main() {

    let host = "172.17.0.11:1161";
    let auth = match true {
	false => netsnmp::Auth::V2c(netsnmp::V2cAuth {
	    community: String::from("public")
	}),
	true => netsnmp::Auth::V3(netsnmp::V3Auth {
	    level: netsnmp::V3Level::AuthPriv {
		auth: netsnmp::V3AuthParams {
		    user: String::from("mnow"),
		    protocol: netsnmp::V3AuthProtocol::SHA,
		    password: String::from("qrT7FZkmXC9jitlTpN35y6UyTbxxUvNf")
		},
		privacy: netsnmp::V3PrivParams {
		    protocol: netsnmp::V3PrivProtocol::AES,
		    password: String::from("qrT7FZkmXC9jitlTpN35y6UyTbxxUvNf")
		}
	    },
	    context: None, context_engine: None,
	    security_engine: None, destination_engine: None
	})
    };

    let snmp = netsnmp::init("SmartM Agent");
    //snmp.set_debug(true);
    
    let mut session = {
	snmp.session()
	    .set_peer(host.as_bytes()).unwrap()
	    .set_auth(&auth).unwrap()
	    .open_single().unwrap()
    };

    println!("Received {:?}", session.get_async(
	&Oid::from_str(".1.3.6.1.2.1.1.9.1.2.1").unwrap()
    ).await.unwrap().unwrap().get_value());

}
