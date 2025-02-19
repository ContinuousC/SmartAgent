/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use expression::{EvalError, Expr};
use std::{env, process};

fn main() {
    match env::args()
        .skip(1)
        .map(|arg| Expr::parse(&arg))
        .collect::<Result<Vec<Expr>, EvalError>>()
    {
        Ok(exprs) => {
            for expr in exprs {
                println!(
                    "{}",
                    serde_json::to_string(&expr)
                        .expect("serialization failed!?")
                );
            }
        }
        Err(err) => {
            eprintln!("{}", err);
            process::exit(1)
        }
    }
}
