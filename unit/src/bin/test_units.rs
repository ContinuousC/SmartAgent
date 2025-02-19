/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use unit::parser::parse_composite_unit;
use unit::{Quantity, UnitError};

fn main() {
    if let Err(e) = test_quantities() {
        println!("Error: {}", e);
    }
}

fn test_quantities() -> Result<(), UnitError> {
    //let n = -1;
    let q = Quantity(5.0, parse_composite_unit("g/m^3")?);
    let u = parse_composite_unit("mg/cm^3")?;
    //let q3 = parse_composite_unit("min")?;

    println!("{} = {}", q, q.convert(&u)?);
    Ok(())
}

/*fn test_conversion() {

    println!("Unit size: {}", std::mem::size_of::<agent::unit::Unit>());
    println!("Quantity size: {}", std::mem::size_of::<agent::unit::Quantity>());

    let from = "mA/mV";
    let to = "A/V";
    let value = 1000000000.0;

    let from_unit = parse_composite_unit(from);
    let to_unit = parse_composite_unit(to);

    match (from_unit,to_unit) {
    (Some(from_unit),Some(to_unit)) => println!(
        "{} {} = {:.2} {}", value, from_unit,
        from_unit.convert(&to_unit, value).unwrap_or(f64::NAN), to_unit),
    _ => println!("invalid unit(s)")
    }

}
*/
