/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use unit::Unit;

#[derive(Default, Debug)]
pub struct TypeOpts {
    // Disable implicit casts between binary and unicode strings.
    pub strict_strings: bool,
}

#[derive(Default, Clone, Debug)]
pub struct FormatOpts {
    pub autoscale: bool,
    pub precision: Option<u8>,
    pub unit: Option<Unit>,
}
