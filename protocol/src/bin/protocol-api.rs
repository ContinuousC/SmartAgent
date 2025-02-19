/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::{collections::HashMap, fs, iter::FromIterator};

use clap::Parser;

use protocol::service::{js_protocol_service, py_protocol_service};
use rpc::Template;

/// Generate protocol service definitions.
#[derive(Parser)]
#[clap(version)]
struct Args {
    /// Path(s) to template files.
    #[clap(required = true)]
    file: Vec<String>,
}

fn main() {
    let args = Args::parse();

    for path in &args.file {
        if let Some(base) = path.strip_suffix(".tmpl.js") {
            let data = fs::read_to_string(path).unwrap();
            let tmpl = Template::parse(&data);
            let src = tmpl.fill(HashMap::from_iter([(
                "ProtocolService",
                js_protocol_service(),
            )]));
            fs::write(&format!("{}.js", base), src).unwrap();
        } else if let Some(base) = path.strip_suffix(".tmpl.py") {
            let data = fs::read_to_string(path).unwrap();
            let tmpl = Template::parse(&data);
            let src = tmpl.fill(HashMap::from_iter([(
                "ProtocolService",
                py_protocol_service(),
            )]));
            fs::write(&format!("{}.py", base), src).unwrap();
        } else {
            eprintln!(
                "Warning: ignoring file with unsupported suffix: {}",
                path
            );
        }
    }
}
