/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::env;
use std::process;

use expression::{EvalError, Expr};
use unit::Unit;
use value::Value;

fn main() {
    let mut args = env::args();
    let _ = args.next();
    let expr = match args.next() {
        Some(arg) => arg,
        None => {
            eprintln!("Please specify an expression!");
            process::exit(2);
        }
    };

    if let Err(e) = run(None, expr, args.next()) {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}

fn run(
    data: Option<Value>,
    expr: String,
    unit: Option<String>,
) -> Result<(), EvalError> {
    let expr = Expr::parse(&expr)?;
    let unit = match unit {
        Some(u) => Some(Unit::parse_composite(&u)?),
        None => None,
    };

    if let Some(d) = &data {
        println!("Data: {} ({})", d, d.get_type());
    }

    println!("Expression: {}", expr);
    println!(
        "Result type: {}",
        expr.check(data.as_ref().map(|d| d.get_type()).as_ref())?
    );

    match (expr.eval(data.map(Ok).as_ref())?, unit) {
        (Value::Quantity(q), Some(u)) => {
            println!("Result in {}: {}", u, q.convert(&u)?)
        }
        (Value::Quantity(q), _) => {
            println!("Result: {}", q);
            println!(
                "Normalized result: {}",
                q.normalize()
                    .expect("normalization failed (should not happen)")
            );
        }
        (val, _) => println!("Result: {}", val),
    }

    Ok(())
}
