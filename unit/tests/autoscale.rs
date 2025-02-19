/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use unit::Quantity;

#[test]
fn autoscale_info() {
    assert_eq!(
        Quantity::parse("5120kB").unwrap().autoscale().unwrap(),
        Quantity::parse("5MB").unwrap()
    );
    assert_eq!(
        Quantity::parse("0TB").unwrap().autoscale().unwrap(),
        Quantity::parse("0B").unwrap()
    );
    assert_eq!(
        Quantity::parse("0.000001TB").unwrap().autoscale().unwrap(),
        Quantity::parse("1.048576MB").unwrap()
    );
}

#[test]
fn autoscale_time() {
    assert_eq!(
        Quantity::parse("1800s").unwrap().autoscale().unwrap(),
        Quantity::parse("30min").unwrap()
    );
    assert_eq!(
        Quantity::parse("3600s").unwrap().autoscale().unwrap(),
        Quantity::parse("1h").unwrap()
    );
    assert_eq!(
        Quantity::parse(".001s").unwrap().autoscale().unwrap(),
        Quantity::parse("1ms").unwrap()
    );
    assert_eq!(
        Quantity::parse(".00001s").unwrap().autoscale().unwrap(),
        Quantity::parse("10.000000000000002μs").unwrap() // floating-point rounding...
    );
    assert_eq!(
        Quantity::parse(".00000000000000000000000001s")
            .unwrap()
            .autoscale()
            .unwrap(),
        Quantity::parse(".010000000000000002ys").unwrap() // floating-point rounding...
    );
    assert_eq!(
        Quantity::parse("123456789s").unwrap().autoscale().unwrap(),
        Quantity::parse("204.12828869047618weeks").unwrap()
    );
    assert_eq!(
        Quantity::parse("0h").unwrap().autoscale().unwrap(),
        Quantity::parse("0s").unwrap()
    );
}
