/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::{fs,io,env,fmt,mem,process};
use std::path::Path;
use std::ffi::OsStr;
use std::io::Write;
use std::convert::From;
use std::collections::HashMap;

use clap::{App,Arg};
use serde::Deserialize;
use reqwest::{self,Url};
use reqwest::blocking::Client;


const DATA_DIR : &'static str   = "var/mnow/data";
const REJECT_SUBDIR : &'static str = "rejected";


fn main() {

    let matches = App::new("SmartM Data Forwarder ")
        .version(env!("CARGO_PKG_VERSION"))
        .author("Maarten Deprez <mdp@si-int.eu>")
        .about("Forwards stored elastic data to elasticsearch.")
	.arg(Arg::with_name("db-url").long("db").short("D")
	     .value_name("URL").takes_value(true).required(true)
	     .help("The url used to connect to elasticsearch"))
	.arg(Arg::with_name("username").long("username").short("U")
	     .value_name("USERNAME").takes_value(true).required(true)
	     .help("The username for the connection to elasticsearch"))
	.arg(Arg::with_name("password").long("password").short("P")
	     .value_name("PASSWORD").takes_value(true).required(true)
	     .help("The password for the connection to elasticsearch"))
	.arg(Arg::with_name("instance").long("instance").short("i")
	     .value_name("INSTANCE").takes_value(true).display_order(1000)
	     .help("The instance for which to retrieve data (default: main)"))
	.get_matches();

    if let Err(err) = run(matches.value_of("db-url").unwrap(),
			  matches.value_of("username").unwrap(),
			  matches.value_of("password").unwrap(),
			  matches.value_of("instance").unwrap_or("main")) {
	eprintln!("Error: {}", err);
	process::exit(1);
    }

}


fn run(url: &str, user: &str, pass: &str, inst: &str) -> Result<()> {

    let client = Client::new();
    let mut url = Url::parse(url)?;
    url.set_path("_bulk");
    url.set_query(None);

    let omd_root = env::var("OMD_ROOT")?;
    let data_dir = Path::new(&omd_root).join(DATA_DIR).join(inst);
    let reject_dir = data_dir.join(REJECT_SUBDIR);

    let pattern = format!("{}/[0-9]*.json", data_dir.display());
    let mut data_files = glob::glob(&pattern)?.collect::<std::result::Result<Vec<_>,_>>()?;
    data_files.sort_by_cached_key(|path| Path::new(path).file_stem().and_then(OsStr::to_str)
				  .and_then(|n| str::parse::<u64>(n).ok()));

    let mut files = Vec::new();
    let mut req = Vec::new();

    for data_file in data_files.iter() {

	req.extend(split_req(&fs::read(data_file)?).into_iter()
		   .map(|d| (data_file.as_path(),d)));
	files.push(data_file.as_path());

	if req.len() >= 1000 {
	    ship_files(&client, &url, user, pass, reject_dir.as_path(),
		       mem::replace(&mut files, Vec::new()),
		       mem::replace(&mut req, Vec::new()))?;
	}

    }

    if !files.is_empty() {
	ship_files(&client, &url, user, pass, reject_dir.as_path(),
		   files, req)?;
    }

    Ok(())

}

fn split_req(mut data: &[u8]) -> Vec<Vec<u8>> {

    let mut reqs = Vec::new();
    let mut n = 0;
    
    while let Some(i) = data.iter().position(|x| *x == b'\n' && {n += 1; n} % 2 == 0) {
	let (req,next) = data.split_at(i+1);
	reqs.push(req.to_vec());
	data = next;
    }

    reqs

}


fn ship_files(client: &Client, url: &Url, user: &str, pass: &str, reject_dir: &Path,
	      files: Vec<&Path>, req: Vec<(&Path,Vec<u8>)>) -> Result<()> {

    for file in files.iter() {
	eprintln!("Shipping {}...", file.display());
    }
    
    let body : Vec<u8> = req.iter().flat_map(|(_,d)| d.to_vec()).collect();
    let res : ESResult<BulkResponse> = client.post(url.clone())
	.header(reqwest::header::CONTENT_TYPE, "application/x-ndjson")
	.basic_auth(user, Some(pass)).body(body).send()?.json()?;

    match res {
	ESResult::Err(e) => Err(Error::Elastic(vec![e.error.trace])),
	ESResult::Unknown(v) => Err(Error::ElasticUnknown(v)),
	ESResult::Ok(r) => {

	    let mut failed = HashMap::new();

	    for ((file,data),res) in req.into_iter().zip(
		r.items.into_iter().map(|i| i.status().result)) {
		if let BulkItemResult::Err(err) = res {
		    eprintln!("Warning: {}: {}: {}", file.display(),
			      err.error.err_type, err.error.reason);
		    failed.entry(file).or_insert_with(Vec::new).push(data);
		}
	    }

	    if !failed.is_empty() {

		fs::create_dir_all(reject_dir)?;

		for (file,data) in failed {
		    let path = reject_dir.join(file.file_name().unwrap());
		    let mut writer = io::BufWriter::new(
			fs::OpenOptions::new().write(true)
			    .create_new(true).open(path)?);
		    for req in data {
			writer.write_all(&req)?;
		    }
		}

	    }

	    for file in files {
		fs::remove_file(file)?;
	    }

	    Ok(())

	}
    }

}


/* Errors. */

type Result<T> = std::result::Result<T,Error>;

enum Error {
    IO(io::Error),
    Env(env::VarError),
    Glob(glob::GlobError),
    GlobPattern(glob::PatternError),
    Request(reqwest::Error),
    //JSON(serde_json::Error),
    Url(url::ParseError),
    Elastic(Vec<ESTrace>),
    ElasticUnknown(serde_json::Value),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> std::result::Result<(), fmt::Error> {
	match self {
	    Error::IO(e) => write!(f, "I/O error: {}", e),
	    Error::Env(e) => write!(f, "environment error: {}", e),
	    Error::Glob(e) => write!(f, "glob error: {}", e),
	    Error::GlobPattern(e) => write!(f, "glob pattern error: {}", e),
	    Error::Request(e) => write!(f, "request error: {}", e),
	    //Error::JSON(e) => write!(f, "JSON error: {}", e),
	    Error::Url(e) => write!(f, "Url parse error: {}", e),
	    Error::ElasticUnknown(e) => write!(f, "Unexpected elastic response: {}", e),
	    Error::Elastic(es)
		=> write!(f, "Elastic error(s): {}{}", es.iter().take(5).map(
		    |e| format!("{}: {}", e.err_type, e.reason))
			  .collect::<Vec<_>>()
			  .join(", "), match es.len() > 5 {
			      true => ", ...", false => "" }),	    
	}
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
	Self::IO(err)
    }
}

impl From<env::VarError> for Error {
    fn from(err: env::VarError) -> Self {
	Self::Env(err)
    }
}

impl From<glob::PatternError> for Error {
    fn from(err: glob::PatternError) -> Self {
	Self::GlobPattern(err)
    }
}

impl From<glob::GlobError> for Error {
    fn from(err: glob::GlobError) -> Self {
	Self::Glob(err)
    }
}

impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
	Self::Request(err)
    }
}

impl From<url::ParseError> for Error {
    fn from(err: url::ParseError) -> Self {
	Self::Url(err)
    }
}


/* Elastic bulk insert structures. */

#[derive(Deserialize,Debug)]
#[serde(untagged)]
pub enum ESResult<T> {
    Ok(T),
    Err(ESStatus),
    Unknown(serde_json::Value),
}


#[derive(Deserialize,Debug)]
pub struct ESStatus {
    pub status: u32,
    pub error: ESError,
}

#[derive(Deserialize,Debug)]
pub struct ESError {
    pub root_cause: Vec<ESTrace>,
    #[serde(flatten)]
    pub trace: ESTrace
}

#[derive(Deserialize,Debug)]
pub struct ESTrace {
    #[serde(rename = "type")]
    pub err_type: String,
    pub reason: String
}

#[derive(Deserialize,Debug)]
pub struct BulkResponse {
    pub took: u64,
    pub errors: bool,
    pub items: Vec<BulkItem>,
}

#[derive(Deserialize,Debug)]
#[serde(rename_all = "lowercase")]
pub enum BulkItem {
    Index(BulkItemStatus),
    Update(BulkItemStatus),
    Delete(BulkItemStatus),
}

#[derive(Deserialize,Debug)]
pub struct BulkItemStatus {
    pub _index: Option<String>,
    pub _id: Option<String>,
    //pub _version: u64,
    pub status: u64,
    #[serde(flatten)]
    pub result: BulkItemResult,
}

#[derive(Deserialize,Debug)]
#[serde(untagged)]
pub enum BulkItemResult {
    Ok(BulkItemOk),
    Err(BulkItemError)
}

#[derive(Deserialize,Debug)]
pub struct BulkItemOk {
    pub result: String
}

#[derive(Deserialize,Debug)]
pub struct BulkItemError {
    pub error: ESTrace
}


impl BulkItem {
    fn status(self) -> BulkItemStatus {
	match self {
	    Self::Index(s) | Self::Update(s) | Self::Delete(s)
		=> s
	}
    }
}
