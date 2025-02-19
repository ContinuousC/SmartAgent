/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::sync::Arc;
use std::str::FromStr;
use std::path::{Path,PathBuf};
use std::collections::HashMap;
use std::time::Duration;
use std::io::BufReader;
use std::{process,fs};

use log::{debug, warn};
use agent::protocols;
use agent::specification::{Spec,Tag};
use agent::context::Verbosity;
use agent::config::{HostConfig,AgentConfig,AgentDataConfig};
use agent::error::Result;
use agent::agent_utils::TryAppend;

use tokio::sync::{watch,mpsc,Barrier,Semaphore};
//use tokio::fs::File;
//use tokio::io::AsyncReadExt;
use tokio::time::Instant;

use tdigest::TDigest;
use netsnmp::Oid;
use rand::Rng;


#[tokio::main]
async fn main() {
    if let Err(e) = main_inner().await {
	eprintln!("Error: {}", e);
	process::exit(1);
    }
}


async fn main_inner() -> Result<()> {

    let verbosity = Some(Verbosity::Debug);
    let mp_path = PathBuf::from("mps");

    let spec = benchmark(verbosity, "loading MPs",
			 || load_specs(verbosity, &mp_path))?;
    let hosts = vec![
	"172.17.0.3:1161",
	"172.17.0.4:1161",
	"172.17.0.5:1161",
	"172.17.0.6:1161",
	"172.17.0.7:1161",
	"172.17.0.8:1161",
	"172.17.0.9:1161",
	"172.17.0.10:1161",
    ].into_iter().cycle().take(1024)
	.map(|host| (host.to_string(), HostConfig {
	    tags: vec![Tag("snmp_interfaces".to_string())].into_iter().collect(),
	    checks: vec![].into_iter().collect(),
	    agent: AgentConfig {
		write_smartm_data: Some(AgentDataConfig {
		    instances: vec!["main".to_string()]
		}),
		use_password_vault: None,
		show_field_errors: false,
		show_table_info: false,
	    },
	    protocols: protocols::Config {
		snmp: protocols::snmp::Config {
		    auth: None,
		    bulk_host: true,
		    bulk_opts: protocols::snmp::BulkConfig::default(),
		    use_walk: false,
		    timing: None,
		    port: Some(1161)
		},
		ssh: protocols::ssh::Config::default(),
		azure: protocols::azure::Config::default(),
		api: protocols::api::Config::default()
	    }
	})).collect();

    let (term_sender,term_receiver) = watch::channel(false);
    let (_config_sender, config_receiver) = watch::channel(Arc::new(hosts));
    let (_spec_sender, spec_receiver) = watch::channel(Arc::new(spec));
    let (data_sender, _data_receiver) = mpsc::channel(1024);
    let (stat_sender, _stat_receiver) = mpsc::channel(1024);

    let sched = scheduler(config_receiver, spec_receiver,
			  data_sender, stat_sender,
			  term_receiver);

    tokio::signal::unix::signal(
        tokio::signal::unix::SignalKind::interrupt()
    ).unwrap().recv().await;

    term_sender.send(true).unwrap();
    sched.await?;

    eprintln!("Done!");
    Ok(())

}


/// Benchmark one step.
fn benchmark<T, F: Fn() -> T>(name: &'static str, f: F) -> T {
    let start = Instant::now();
    let r = f();
    let duration = Instant::now().duration_since(start);
    debug!(verbosity, "Benchmark: {} took {:.03}s",
	       name, duration.as_secs_f64());
    r
}


/// Load specification files.
fn load_specs(path: &Path) -> Result<Spec> {

    let mut spec = Spec::new();

    let spec_paths = glob::glob(&path.join("*.json").to_str().unwrap())?
	.collect::<std::result::Result<Vec<_>,_>>()?;

    for path in spec_paths {

	/*if let Some(PasswordVault::KeePass) = config.agent.use_password_vault {
	    let st = fs::metadata(&path)?;
	    if st.uid() != 0 || st.gid() != 0 || st.mode() & 0o777113 != 0o100000 {
		warn!(verbosity, "Ignoring specification with invalid owner or permissions: {}", path.display());
		continue;
	    }
	}*/

	match fs::File::open(&path) {
	    Err(e) => warn!(verbosity, "Failed to open MP specification {}: {}",
				   path.display(), e),
	    Ok(file) => {
		let reader = BufReader::new(file);
		match Spec::from_file(reader) {
		    Err(e) => warn!(verbosity, "Failed to decode MP specification {} \
						       : {}", path.display(), e),
		    Ok(new_spec) => spec.try_append(new_spec)?
		}
	    }
	}

    }

    Ok(spec)

}


async fn scheduler(mut config_receiver: watch::Receiver<Arc<HashMap<String,HostConfig>>>,
		   mut spec_receiver: watch::Receiver<Arc<Spec>>,
		   _data_sender: mpsc::Sender<String>,
		   _stat_sender: mpsc::Sender<String>,
		   mut term_receiver: watch::Receiver<bool>)
		   -> Result<()> {

    let snmp = Arc::new(netsnmp::init("SmartM Agent"));
    //snmp.set_debug(true);

    let mut run = true;

    /* In tokio >=0.3 .recv() (returning the initial value on first call)
     * is replaced by .changed() (returning "()") and we probably won't
     * have to do the following to purge the "initial change" notification.
     */
    /*let _ = spec_receiver.recv().await.unwrap();
    let _ = config_receiver.recv().await.unwrap();
    let _ = term_receiver.recv().await.unwrap();*/

    while run {

	let _spec = spec_receiver.borrow().clone();
	let config = config_receiver.borrow().clone();

	eprintln!("Running with {} hosts... (press ctrl+c to stop)", config.len());

	let barrier = Arc::new(Barrier::new(config.len() + 1));
	let semaphore = Arc::new(Semaphore::new(32));

	//tokio::task::spawn(net_control("enp3s0", 20000000f64, 30000000f64,
	//                   semaphore.clone()));

	eprintln!("Creating workers...");
	let workers : Vec<_> = config.iter().map(
	    |(host,_config)| tokio::task::spawn(worker(host.to_string(), snmp.clone(),
						       barrier.clone(), semaphore.clone(),
						       term_receiver.clone()))
	).collect();

	barrier.wait().await;
	eprintln!("Go!");


	tokio::select!{
	    t = term_receiver.changed() => { run = t.is_err() || *term_receiver.borrow() },
	    c = config_receiver.changed() => { run = c.is_err() },
	    s = spec_receiver.changed() => { run = s.is_err() },
	};

	let mut digests = Vec::new();
	for worker in workers {
	    digests.push(worker.await.unwrap());
	}

	let (digests1,digests2) : (Vec<_>,Vec<_>) = digests.into_iter().enumerate()
	    .partition(|(n,_digest)| n % 64 < 32);

	let digest1 = TDigest::merge_digests(digests1.into_iter().map(|(_,d)| d).collect());
	let digest2 = TDigest::merge_digests(digests2.into_iter().map(|(_,d)| d).collect());

	println!("Period (for timeout checks): {:?} -- {:?} -- {:?}",
		 Duration::from_nanos(digest1.estimate_quantile(0.05) as u64),
		 Duration::from_nanos(digest1.estimate_quantile(0.50) as u64),
		 Duration::from_nanos(digest1.estimate_quantile(0.95) as u64));

	println!("Period (for non-timeout checks): {:?} -- {:?} -- {:?}",
		 Duration::from_nanos(digest2.estimate_quantile(0.05) as u64),
		 Duration::from_nanos(digest2.estimate_quantile(0.50) as u64),
		 Duration::from_nanos(digest2.estimate_quantile(0.95) as u64));

    }

    Ok(())

}

async fn worker(host: String, snmp: Arc<netsnmp::NetSNMP>,
		barrier: Arc<Barrier>, _semaphore: Arc<Semaphore>,
		mut term: watch::Receiver<bool>) -> TDigest {

    //println!("Starting worker for {}...", host);

    let mut digest = TDigest::new_with_size(10);
    let target_period = Duration::from_millis(30000);
    let mut period = target_period;
    let mut last = None;

    let oids = vec![Oid::from_str(".1.3.6.1.2.1.1.9.1.2").unwrap()];
    let auth = match false {
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

    let mut session = None;
    let mut task_times = Vec::with_capacity(10);
    let mut task_time_i = 0;

    /*eprintln!("probe_engine_id {}", host);
    session.probe_engine_id();
    eprintln!("/probe_engine_id {}", host);*/

    //barrier.wait().await;
    //eprintln!("Start querying!");

    let open = snmp.session()
	.set_peer(host.as_bytes()).unwrap()
	.set_auth(&auth).unwrap()
	.set_async_probe(true)
	.open_single();

    match open {
	Ok(open) => {
	    session = Some(open);
	},
	Err(err) => {
	    println!("Warning: failed to open session for {}: {}", &host, err);
	}
    }

    //assert!(term.recv().await == Some(false));
    barrier.wait().await;

    loop {

	let now = Instant::now();
	let next = match last {
	    None => now + Duration::from_secs_f64(
		period.as_secs_f64() * rand::thread_rng().gen::<f64>()),
	    Some(last) => now.max(last + period)
	};

	tokio::select!{
	    _ = term.changed() => return digest,
	    _ = tokio::time::sleep(next.duration_since(now)) => {}
	};


	/*let mut lock = Some(tokio::select!{
	lock = semaphore.acquire() => lock,
	_ = term.recv() => return digest
        });*/
	//let mut lock : Option<tokio::sync::SemaphorePermit> = None;

	let now = Instant::now();
	let stabilized_now = match now.duration_since(next)
	    < Duration::from_millis(1) {
		true => next,
		false => now
	    };

	if let Some(last) = last {
	    digest = digest.merge_sorted(vec![
		stabilized_now.duration_since(last).as_nanos() as f64
	    ]);
	}

	last = Some(stabilized_now);

	if session.is_none() {

	    let open = snmp.session()
		.set_peer(host.as_bytes()).unwrap()
		.set_auth(&auth).unwrap()
		.set_async_probe(true)
		.open_single();

	    match open {
		Ok(open) => {
		    session = Some(open);
		},
		Err(err) => {
		    println!("Warning: failed to open session for {}: {}", &host, err);
		}
	    }

	}

	if let Some(session) = session.as_mut() {
	    let root = oids[0].clone();
	    let mut last = oids[0].clone();
	    let mut vals = Vec::new();
	    'walk : loop {

		let gets = vec![];
		let walks = vec![last.clone()];
		let query = session.get_bulk_async(&gets, &walks, 3);
		/*let mut lock_time = tokio::time::delay_for(Duration::from_millis(50));
		tokio::pin!(query);*/

		let res = query.await; /*loop {
		tokio::select!{
		_ = &mut lock_time, if lock.is_some() => {
		//println!("dropping lock...");
		lock.take();
	        },
		r = &mut query => break r
	        }
			};*/

		match res {
		    Ok(pdu) => for var in pdu.variables() {
			last = var.get_name();
			match root.contains(&last) {
			    true => vals.push(var.get_value().unwrap()),
			    false => break 'walk
			}
		    },
		    Err(err) => {
			match err {
			    netsnmp::Error::General(e)
				if e == "Engineid probe failed." => {},
			    netsnmp::Error::General(e)
				if e == "timeout" => {},
			    _ => { eprintln!("Warning: received error from {}: {}", host, err) }
			};
			break 'walk
		    }
		}
	    }
	    //println!("Result: {} vars", vals.len());

	    let task_time = Instant::now().duration_since(now);

	    match task_times.len() < task_times.capacity() {
		true => task_times.push(task_time),
		false => {
		    task_times[task_time_i] = task_time;
		    task_time_i = (task_time_i + 1) % task_times.capacity();
		}
	    }

	    if task_times.len() > 3 {
		let avg_time = task_times.iter().sum::<Duration>()
		    / task_times.len() as u32;
		let f = avg_time.as_secs_f64() / period.as_secs_f64();
		if period > target_period && f < 0.8 || f > 1.0 {
		    period = target_period.max(avg_time);
		    println!("Adjusting period --> {:?}", period);
		}
	    }

	}

	//std::mem::drop(lock);

    }

}


/*async fn net_control(dev: &'static str, tx_target: f64, rx_target: f64,
		     semaphore: Arc<Semaphore>) {

    let period = Duration::from_millis(100);
    let mut last = None;
    let mut i = 0;
    let mut c = 16;

    loop {

	i += 1;
	let now = Instant::now();
	let tx1 = read_counter(&format!("/sys/class/net/{}/statistics/tx_bytes", dev)).await;
	let rx1 = read_counter(&format!("/sys/class/net/{}/statistics/rx_bytes", dev)).await;

	if let Some((t0, tx0, rx0)) = last {

	    let tx = 8f64 * (tx1 - tx0) as f64 / now.duration_since(t0).as_secs_f64();
	    let rx = 8f64 * (rx1 - rx0) as f64 / now.duration_since(t0).as_secs_f64();

	    if i % 10 == 0 {
		println!("\nStatistics for {}:\n- tx: {} Mbps\n- rx: {} Mbps",
			 dev, tx / 1000000f64, rx / 1000000f64);
	    }

	    if tx / tx_target > 1.1 && c > 12 {
		println!("Too busy... decreasing concurrency (now at {})!", c - 1);
		semaphore.acquire().await.forget();
		c -= 1;
	    } else if tx / tx_target < 0.9 && semaphore.available_permits() <= 3 {
		println!("Too calm... increasing concurrency (now at {})!", c + 1);
		semaphore.add_permits(1);
		c += 1;
	    }

	}

	last = Some((now, tx1, rx1));

	tokio::time::delay_until(now + period).await;

    }

}

async fn read_counter(path: &str) -> usize {
    let mut buf = String::new();
    let mut f = File::open(path).await.unwrap();
    f.read_to_string(&mut buf).await.unwrap();
    buf[..buf.len() - 1].parse().unwrap()
}
*/
