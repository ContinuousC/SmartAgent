/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::{collections::HashMap, fs, iter::FromIterator};

use agent_api::{js_agent_service_stub, py_agent_service_stub};
use broker_api::js_broker_service_stub;
use clap::{crate_version, App, Arg};
use rpc::Template;

fn main() {
    let matches = App::new("Generate SmartAgent api definitions.")
        .version(crate_version!())
        .arg(
            Arg::with_name("file")
                .help("Path to a source template file.")
                .multiple(true)
                .required(true),
        )
        .get_matches();

    for path in matches.values_of("file").unwrap() {
        if let Some(base) = path.strip_suffix(".tmpl.js") {
            let data = fs::read_to_string(path).unwrap();
            let tmpl = Template::parse(&data);
            let src = tmpl.fill(HashMap::from_iter([
                ("AgentService", js_agent_service_stub()),
                ("BrokerService", js_broker_service_stub()),
            ]));
            fs::write(&format!("{}.js", base), src).unwrap();
        } else if let Some(base) = path.strip_suffix(".tmpl.py") {
            let data = fs::read_to_string(path).unwrap();
            let tmpl = Template::parse(&data);
            let src = tmpl.fill(HashMap::from_iter([(
                "AgentService",
                py_agent_service_stub(),
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
