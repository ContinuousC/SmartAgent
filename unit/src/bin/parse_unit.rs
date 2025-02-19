/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use clap::Parser;
use std::{env, process};
use unit::{Unit, UnitError};

#[derive(Parser)]
#[clap(version = env!("CARGO_PKG_VERSION"))]
/// SmartM Unit Parser
///
/// Parses human-readable units and outputs a JSON representation.
struct Args {
    #[clap(long, short)]
    /// Output the unit's domain.
    dimension: bool,
    /// The unit to parse.
    unit: Vec<String>,
}

fn main() {
    // let matches = App::new("SmartM Unit Parser")
    //     .version(env!("CARGO_PKG_VERSION"))
    //     .author("Maarten Deprez <mdp@si-int.eu>")
    //     .about("Parses human-readable units and outputs a JSON representation.")
    //     .arg(
    //         Arg::with_name("dimension")
    //             .long("dimension")
    //             .short("d")
    //             .help("Output the unit's domain."),
    //     )
    //     .arg(
    //         Arg::with_name("unit")
    //             .required(true)
    //             .multiple(true)
    //             .help("The unit to parse."),
    //     )
    //     .get_matches();

    let args = Args::parse();

    match args
        .unit
        .iter()
        .map(|s| Unit::parse_composite(s))
        .collect::<Result<Vec<Unit>, UnitError>>()
    {
        Ok(units) => match args.dimension {
            false => {
                for unit in units {
                    println!(
                        "{}",
                        serde_json::to_string(&unit)
                            .expect("serialization failed!?")
                    );
                }
            }
            true => {
                for unit in units {
                    println!(
                        "{}",
                        serde_json::to_string(&unit.dimension())
                            .expect("serialization failed!?")
                    );
                }
            }
        },
        Err(err) => {
            eprintln!("{}", err);
            process::exit(1)
        }
    }
}
