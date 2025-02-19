/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::mem::MaybeUninit;

use crate::{BaseUnit, BinPrefix, DecPrefix, FracPrefix, Prefix, SiPrefix};

use super::{
    ConductivityUnit, CurrentUnit, DimensionlessUnit, FanSpeedUnit,
    FrequencyUnit, InformationUnit, LengthUnit, MassUnit, OperationUnit,
    PotentialUnit, PowerUnit, ResistanceUnit, TemperatureUnit, TimeUnit,
};

macro_rules! units_len {
    ($unit:ident) => {
        1
    };
    ($unit:ident ( $prefix:ident ) ) => {
        units_len!($unit ( $prefix SCALE ))
    };
    ($unit:ident ( $prefix:ident $list:ident ) ) => {
        $prefix::$list.len()
    };
    ($unit:ident $(( $prefix:ident $($list:ident)? ))?, $($nextUnit:ident $(( $nextPrefix:ident $($nextList:ident)? ))?),+) => {
        units_len!($unit $( ( $prefix $($list)? ) )?) + units_len!($($nextUnit $(( $nextPrefix $($nextList)? ))?),+)
    };
}

macro_rules! add_units {
    ($r:ident, $i:expr, $ty:ident, $unit:ident) => {{
		$r[$i] = MaybeUninit::new($ty::$unit);
        $i + 1
    }};
    ($r:ident, $i:expr, $ty:ident, $unit:ident ( $prefix:ident )) => {
		add_units!($r, $i, $ty, $unit ( $prefix SCALE ))
	};
	($r:ident, $i:expr, $ty:ident, $unit:ident ( $prefix:ident $list:ident )) => {{
		let mut i = 0;
        while i < $prefix::$list.len() {
            $r[$i + i] = MaybeUninit::new($ty::$unit($prefix::$list[i]));
            i += 1;
        }
		$i + $prefix::$list.len()
    }};
    ($r:ident, $i:expr, $ty:ident, $unit:ident $(( $prefix:ident $($list:ident)? ))?, $( $nextUnit:ident $(( $nextPrefix:ident $($nextList:ident)? ))? ),+) => {{
        let i = add_units!($r, $i, $ty, $unit $(( $prefix $($list)? ))?);
		add_units!($r, i, $ty, $($nextUnit $(( $nextPrefix $($nextList)? ))?),+)
    }};
}

macro_rules! define_units {
    ($name:ident, $ty:ident, $($unit:ident $(( $prefix:ident $($list:ident)? ))?),+) => {
        pub(crate) static $name: [$ty; units_len!($(units $(($prefix $($list)?))?),+)] = {
            let mut r = [MaybeUninit::uninit(); units_len!($(units $(($prefix $($list)?))?),+)];
			let i = add_units!(r, 0, $ty, $($unit $(($prefix $($list)?))?),+);
			assert!(r.len() == i);
            unsafe { std::mem::transmute(r) }
        };
    };
}

define_units!(LENGTH_UNITS, LengthUnit, Meter(SiPrefix));
define_units!(MASS_UNITS, MassUnit, Gram(SiPrefix));
define_units!(
    TIME_UNITS,
    TimeUnit,
    Second(FracPrefix),
    Minute,
    Hour,
    Day,
    Week
);
define_units!(CURRENT_UNITS, CurrentUnit, Ampere(SiPrefix));
define_units!(
    TEMPERATURE_UNITS,
    TemperatureUnit,
    Celsius,
    Fahrenheit,
    Kelvin
);
define_units!(POTENTIAL_UNITS, PotentialUnit, Volt(SiPrefix));
define_units!(POWER_UNITS, PowerUnit, Watt(SiPrefix));
define_units!(RESISTANCE_UNITS, ResistanceUnit, Ohm(SiPrefix));
define_units!(CONDUCTIVITY_UNITS, ConductivityUnit, Siemens(SiPrefix));
define_units!(
    FREQUENCY_UNITS,
    FrequencyUnit,
    Hertz(SiPrefix),
    PerTime(TimeUnit LIST)
);
define_units!(
    INFORMATION_UNITS,
    InformationUnit,
    Bit(DecPrefix),
    Byte(BinPrefix)
);
define_units!(OPERATIONS_UNITS, OperationUnit, Operation(DecPrefix));
define_units!(FAN_SPEED_UNITS, FanSpeedUnit, RPM, RPS);
define_units!(
    DIMENSIONLESS_UNITS,
    DimensionlessUnit,
    Count(DecPrefix),
    Percent,
    Permille
);
