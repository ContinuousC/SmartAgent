/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::str::FromStr;
use std::io::{Read,Write};
use std::time::{Duration,Instant};
use std::os::unix::net::UnixStream;
use std::os::unix::io::{RawFd,FromRawFd,IntoRawFd,AsRawFd};
use std::collections::HashMap;
use std::convert::TryFrom;
use std::cell::RefCell;
use std::sync::Arc;
use std::rc::Rc;
use std::thread;
use tdigest::TDigest;
use tokio::sync::{watch,oneshot};
use crossbeam::channel;
use netsnmp::{Oid,Pdu,SyncQuery};


#[tokio::main]
async fn main() {

    let hosts : Vec<_> = vec![
	"172.17.0.3:1161",
	"172.17.0.4:1161",
	"172.17.0.5:1161",
	"172.17.0.6:1161",
	"172.17.0.7:1161",
	"172.17.0.8:1161",
	"172.17.0.9:1161",
	"172.17.0.10:1161",
    ].into_iter().cycle()
	.take(256).collect();

    eprintln!("Running with {} hosts... (press ctrl+c to stop)", hosts.len());

    let (term_sender,term_receiver) = watch::channel(false);
    let snmp = NetSNMP::init();

    let workers : Vec<_> = hosts.iter().map(
	|host| tokio::task::spawn(worker(host, snmp.clone(), term_receiver.clone()))
    ).collect();
    
    let mut signal = tokio::signal::unix::signal(
        tokio::signal::unix::SignalKind::interrupt()
    ).unwrap();
    signal.recv().await;
    term_sender.send(true).unwrap();

    for worker in workers {
	worker.await.unwrap();
    }

    //snmp.worker.join().unwrap();
    
    println!("Done!");

}


struct NetSNMP {
    //worker: thread::JoinHandle<()>,
    sender: channel::Sender<SNMPQuery>,
    waker: RawFd,
}

enum SNMPQuery {
    //Get(&'static str, Oid, oneshot::Sender<netsnmp::Result<Option<netsnmp::Value>>>),
    Walk(&'static str, Oid, oneshot::Sender<netsnmp::Result<Vec<netsnmp::Value>>>)
}

struct WalkState {
    root: Oid,
    last: Oid,
    max_reps: i64,
    vals: Vec<netsnmp::Value>,
    response: oneshot::Sender<netsnmp::Result<Vec<netsnmp::Value>>>,
    cleanup_sessions: Rc<RefCell<Vec<*const netsnmp::api::snmp_session>>>
}

impl NetSNMP {

    fn init() -> Arc<Self> {

	let (query_sender,query_receiver) = channel::unbounded();
	let (wake_sender, wake_receiver) = UnixStream::pair().unwrap();

	thread::spawn(|| Self::worker(
	    query_receiver,
	    wake_receiver.into_raw_fd()));
	
	Arc::new(Self {
	    //worker: ,
	    waker: wake_sender.into_raw_fd(),
	    sender: query_sender,
	})
	
    }

    /*async fn get(&self, host: &'static str, oid: Oid)
		 -> netsnmp::Result<Option<netsnmp::Value>> {
	let (sender,receiver) = oneshot::channel();
	self.sender.send(SNMPQuery::Get(host, oid, sender)).unwrap();
	unsafe {
	    let mut stream = UnixStream::from_raw_fd(self.waker);
	    stream.write_all(&[0]).unwrap();
	    stream.into_raw_fd();
	}
	receiver.await.unwrap()
    }*/

    async fn walk(&self, host: &'static str, oid: Oid)
		  -> netsnmp::Result<Vec<netsnmp::Value>> {
	let (sender,receiver) = oneshot::channel();
	self.sender.send(SNMPQuery::Walk(host, oid, sender)).unwrap();
	unsafe {
	    let mut stream = UnixStream::from_raw_fd(self.waker);
	    stream.write_all(&[0]).unwrap();
	    stream.into_raw_fd();
	}
	receiver.await.unwrap()
    }
    
    fn worker(queries: channel::Receiver<SNMPQuery>, waker: RawFd) {

	let snmp = netsnmp::init("SmartM Agent");
	//snmp.set_debug(true);
	let mut waker = unsafe { UnixStream::from_raw_fd(waker) };
	waker.set_nonblocking(true).unwrap();

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

	let max_reps = 3;
	
	/*netsnmp::Auth::V3(netsnmp::V3Auth {
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
	    context: None,
	    context_engine: None,
	    security_engine: None,
	    destination_engine: None
	});*/

	let mut active_sessions = HashMap::new();
	let cleanup_sessions = Rc::new(RefCell::new(Vec::new()));

	loop {

	    snmp.read_or_wake(waker.as_raw_fd());
	    while waker.read(&mut [0;16usize]).map_or(false, |n| n > 0) { }

	    while let Ok(query) = queries.try_recv() {
		match query {

		    /*SNMPQuery::Get(host, oid, response) => {

			//println!("SNMP: received get query for {} @ {}", oid, host);

			let mut response = Some(response);
			let cleanup_sessions_ref = cleanup_sessions.clone();

			let mut session = snmp.session()
			    .set_peer(host.as_bytes()).unwrap()
			    .set_auth(&auth).unwrap()
			    .set_callback(move |op, sess, _x, pdu| match op {
				netsnmp::CallbackOp::Connect => {
				    println!("Connect!");
				    Ok(())
				},
				netsnmp::CallbackOp::ReceivedMessage => {
				    //println!("Received message!");
				    response.take().unwrap().send(
					match pdu.variables().into_iter().next() {
					    Some(var) => match var.get_value() {
						Ok(val) => Ok(Some(val)),
						Err(_) => {
						    panic!("Received errtype: {:?}", var.get_name());
						}
					    },
					    None => Ok(None),
					}
				    ).unwrap();
				    cleanup_sessions_ref.borrow_mut().push(sess.as_raw());
				    //sess.shutdown();
				    Ok(())
				},
				netsnmp::CallbackOp::Disconnect => {
				    println!("Disconnect!");
				    Ok(())
				},
				_ => {
				    println!("{:?}", op);
				    response.take().unwrap().send(Err(netsnmp::Error::General(String::from("timeout?"))));
				    Ok(())
				}
			    }).open().unwrap();

			session.async_send_get(&oid);
			active_sessions.insert(session.as_raw(),session);

		    },*/

		    SNMPQuery::Walk(host, oid, response) => {

			let mut session = snmp.session()
			    .set_peer(host.as_bytes()).unwrap()
			    .set_auth(&auth).unwrap()
			    .set_callback_static(Self::walker, Box::into_raw(
				Box::new(WalkState {
				    cleanup_sessions: cleanup_sessions.clone(),
				    response: response,
				    max_reps: max_reps,
				    root: oid.clone(),
				    last: oid.clone(),
				    vals: Vec::new(), 
				})) as *mut std::ffi::c_void)
			    .open_multi().unwrap();

			let pdu = Pdu::get_bulk(1, max_reps).add_oid(&oid);
			session.send(pdu).unwrap();
			active_sessions.insert(session.as_raw(),session);

		    }

		}
		
	    }

	    for session in cleanup_sessions.borrow_mut().drain(..) {
		active_sessions.remove(&session);
	    }

	}

    }

    extern "C" fn walker(op: i32, session: *mut netsnmp::api::snmp_session, _x: i32,
			 pdu: *mut netsnmp::api::snmp_pdu,
			 magic: *mut std::ffi::c_void) -> i32 {

	let session = unsafe { netsnmp::MultiSessionPtr::from_mut(session) };
	let pdu = unsafe { netsnmp::PduPtr::from_ptr(pdu) };
	let state: &mut WalkState = unsafe {
	    &mut *(magic as *mut WalkState)
	};

	match netsnmp::CallbackOp::try_from(op) {
	    Ok(netsnmp::CallbackOp::Connect) => {
		println!("Connect!");
	    },
	    Ok(netsnmp::CallbackOp::ReceivedMessage) => {
	    
		let mut done = false;
		for var in pdu.variables() {
		    let oid = var.get_name();
		    if !state.root.contains(&oid) {
			done = true;
			break;
		    }
		    state.last = oid;
		    state.vals.push(match var.get_value() {
			Ok(val) => val,
			Err(_) => {
			    panic!("Received errtype: {:?}", var.get_name());
			}
		    });
		}

		if done {
		    let state = unsafe {
			Box::from_raw(magic as *mut WalkState)
		    };
		    state.response.send(
			Ok(state.vals)
		    ).unwrap();
		    state.cleanup_sessions.borrow_mut().push(session.as_raw());
		} else {
		    let pdu = Pdu::get_bulk(0, state.max_reps).add_oid(&state.last);
		    session.send(pdu).unwrap();
		}
	    },
	    Ok(netsnmp::CallbackOp::Disconnect) => {
		println!("Disconnect!");
	    },
	    _ => {
		println!("{:?}", op);
		let state = unsafe {
		    Box::from_raw(magic as *mut WalkState)
		};
		state.response.send(Err(netsnmp::Error::General(String::from("timeout?")))).unwrap();
		state.cleanup_sessions.borrow_mut().push(session.as_raw());
	    }
	}
	1
    }

}


async fn worker(host: &'static str, snmp: Arc<NetSNMP>, mut term: watch::Receiver<bool>) {

    //println!("Starting worker for {}...", host);

    let mut digest = TDigest::new_with_size(10);
    let period = Duration::from_millis(3);
    let mut last = None;

    loop {

	let now = Instant::now();


	let next = match last {
	    None => now,
	    Some(last) => now.max(last + period)
	};

	let sleep = tokio::time::sleep(next.duration_since(now));

	if let Some(last) = last {
	    digest = digest.merge_sorted(vec![
		next.duration_since(last).as_nanos() as f64
	    ]);
	}

	last = Some(next);

	tokio::select!{

	    _ = term.changed() => if *term.borrow() {
		println!("Period for {}: {:?} -- {:?} -- {:?}", host,
			 Duration::from_nanos(digest.estimate_quantile(0.05) as u64),
			 Duration::from_nanos(digest.estimate_quantile(0.50) as u64),
			 Duration::from_nanos(digest.estimate_quantile(0.95) as u64));
		return;
	    },

	    _ = sleep => {

		//println!("Running SNMP queries for {}...", host);
		//let oid = Oid::from_str(".1.3.6.1.2.1.1.6.0").unwrap();
		let oid = Oid::from_str(".1.3.6.1.2.1.1.9.1.2").unwrap();
		let _result = snmp.walk(host, oid).await.unwrap();
		//println!("Result: {:?}", result);
		//assert_eq!(result.unwrap(), Some(netsnmp::Value::OctetStr(vec![83, 105, 116, 116, 105, 110, 103, 32, 111, 110, 32, 116, 104, 101, 32, 68, 111, 99, 107, 32, 111, 102, 32, 116, 104, 101, 32, 66, 97, 121])));

	    }

	}
    }

}
