/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::path::Path;
use std::process::Command;
use std::os::unix::net::UnixStream;
use std::os::unix::io::{FromRawFd/*, RawFd*/};
use std::io::{Write,BufRead,BufReader};

use clap::{App,Arg};
use nix::sys::socket::{self,AddressFamily,SockType,
		       SockFlag/*,SockAddr,UnixAddr*/};


fn main() {

    let matches = App::new("Protocol plugin tester.")
        .version(env!("CARGO_PKG_VERSION"))
        .author("Maarten Deprez <mdp@si-int.eu>")
        .author("Vincent Stuyck <vst@si-int.eu>")
        .about("Run protocol plugin methods.")
	.arg(Arg::with_name("plugin").help("The plugin to call."))
	.subcommand(App::new("input").about("Describe available data."))
	.subcommand(App::new("query").about("Retrieve data from a host.")
		    .arg(Arg::with_name("params"))
		    .arg(Arg::with_name("data_table"))
		    .arg(Arg::with_name("data_field").multiple(true)))
        .get_matches();

    let (server,client) = socket::socketpair(AddressFamily::Unix, SockType::Stream, None,
					     SockFlag::empty()).unwrap();
    let mut server = unsafe { UnixStream::from_raw_fd(server) };
    
    let plugin_path = Path::new(&matches.value_of("plugin").unwrap())
	.canonicalize().unwrap();
    println!("Loading plugin from {}", plugin_path.display());
    
    let plugin = Command::new(plugin_path)
	.arg(&format!("{}", client))
	.spawn().expect("Failed to execute plugin!");
    std::mem::drop(unsafe { UnixStream::from_raw_fd(client) });

    if let Some(query) = matches.subcommand_matches("query") {

	println!("Requesting {} ({}) from {} ({})",
		 query.value_of("data_table").unwrap(),
		 query.values_of("data_field").unwrap()
		 .collect::<Vec<_>>().join(", "),
		 matches.value_of("plugin").unwrap(),
		 query.value_of("params").unwrap());
	write!(server, "Please get {}...\n", query.value_of("data_table").unwrap()).unwrap();

	let mut line = String::new();
	BufReader::new(server).read_line(&mut line).unwrap();
	println!("Got \"{}\"", line);

    }

    std::mem::drop(plugin);
    
}
