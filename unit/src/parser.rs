/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use nom::{
    self,
    branch::alt,
    character::complete::{digit1, space0},
    combinator::{opt, value},
    error::ErrorKind,
    multi::{fold_many1, separated_list1},
    number::complete::double,
    sequence::{preceded, terminated, tuple},
    IResult,
};

use crate::Quantity;

use super::{BinPrefix, DecPrefix, FracPrefix, SiPrefix};
use super::{
    ConductivityUnit, CurrentUnit, DimensionlessUnit, FanSpeedUnit,
    FrequencyUnit, InformationUnit, LengthUnit, MassUnit, OperationUnit,
    PotentialUnit, PowerUnit, ResistanceUnit, TemperatureUnit, TimeUnit,
};
use super::{Unit, UnitError, NEUTRAL_UNIT};

/// Parse a string to a quantity.
pub fn parse_quantity(input: &str) -> Result<Quantity, UnitError> {
    match quantity(input) {
        Ok(("", q)) => q,
        Ok((r, _)) => {
            Err(UnitError::ParseError(format!("Leftover input: {}", r)))
        }
        Err(err) => Err(UnitError::ParseError(format!("{}", err))),
    }
}

/// Parse a string to a (possibly composite) unit.
pub fn parse_composite_unit(input: &str) -> Result<Unit, UnitError> {
    if input.is_empty() {
        return Ok(NEUTRAL_UNIT);
    }

    match composite_unit(input) {
        Ok(("", q)) => q,
        Ok((r, _)) => {
            Err(UnitError::ParseError(format!("Leftover input: {}", r)))
        }
        Err(err) => Err(UnitError::ParseError(format!("{}", err))),
    }
}

/// Parse a string to a simple unit (no multiplication,
/// division or exponentiation).
pub fn parse_unit(input: &str) -> Result<Unit, UnitError> {
    if input.is_empty() {
        return Ok(NEUTRAL_UNIT);
    }

    match unit(input) {
        Ok(("", u)) => Ok(u),
        Ok((r, _)) => {
            Err(UnitError::ParseError(format!("Leftover input: {}", r)))
        }
        Err(err) => Err(UnitError::ParseError(format!("{}", err))),
    }
}

/// Parser for units.
pub fn unit(input: &str) -> IResult<&str, Unit> {
    alt((
        nonprefixed_unit,
        si_prefix(si_unit),
        frac_prefix(frac_unit),
        dec_prefix(dec_unit),
        bin_prefix(bin_unit),
    ))(input)
}

/// Parser for composite units.
pub fn composite_unit(input: &str) -> IResult<&str, Result<Unit, UnitError>> {
    let (input, (num, denom)) = tuple((
        opt(unit_list('*')),
        opt(preceded(char('/'), unit_list('/'))),
    ))(input)?;

    match (num, denom) {
        (Some(Ok(n)), None) => Ok((input, Ok(n))),
        (None, Some(Ok(d))) => Ok((input, d.powi_unwrapped(-1))),
        (Some(Ok(n)), Some(Ok(d))) => Ok((input, n.div_unwrapped(d))),
        (Some(Err(e)), _) => Ok((input, Err(e))),
        (_, Some(Err(e))) => Ok((input, Err(e))),
        (None, None) => Err(nom::Err::Error(nom::error::Error {
            input,
            code: ErrorKind::Alt,
        })),
    }
}

/// Parser for quantities (number and unit).
pub fn quantity(input: &str) -> IResult<&str, Result<Quantity, UnitError>> {
    let (input, (num, _, unit)) =
        tuple((double, space0, composite_unit))(input)?;
    Ok((input, unit.map(|u| Quantity(num, u))))
}

pub fn valid_composite_unit(input: &str) -> IResult<&str, Unit> {
    match composite_unit(input)? {
        (input, Ok(unit)) => Ok((input, unit)),
        _ => Err(nom::Err::Error(nom::error::Error {
            input,
            code: ErrorKind::Alt,
        })),
    }
}

fn unit_list(
    sep: char,
) -> impl Fn(&str) -> IResult<&str, Result<Unit, UnitError>> {
    move |input| {
        let (input, units) =
            separated_list1(char(sep), tuple((unit, opt(power))))(input)?;
        Ok((
            input,
            units.into_iter().try_fold(NEUTRAL_UNIT, |q, (u, n)| {
                q.mul_unwrapped(u.powi_unwrapped(n.unwrap_or(1))?)
            }),
        ))
    }
}

fn power(input: &str) -> IResult<&str, i32> {
    alt((hat_power, superscript_power))(input)
}

fn hat_power(input: &str) -> IResult<&str, i32> {
    let (input, (s, n)) =
        preceded(char('^'), tuple((opt(sign), digit1)))(input)?;
    Ok((input, s.unwrap_or(1) * n.parse::<i32>().unwrap()))
}

fn superscript_power(input: &str) -> IResult<&str, i32> {
    let (input, (s, n)) =
        tuple((opt(superscript_sign), superscript_digit1))(input)?;
    Ok((input, s.unwrap_or(1) * n))
}

fn superscript_sign(input: &str) -> IResult<&str, i32> {
    alt((value(-1, char('⁻')), value(1, char('⁺'))))(input)
}

fn superscript_digit1(input: &str) -> IResult<&str, i32> {
    fold_many1(superscript_digit, || 0, |n, i| n * 10 + i)(input)
}

fn superscript_digit(input: &str) -> IResult<&str, i32> {
    alt((
        value(0, char('⁰')),
        value(1, char('¹')),
        value(2, char('²')),
        value(3, char('³')),
        value(4, char('⁴')),
        value(5, char('⁵')),
        value(6, char('⁶')),
        value(7, char('⁷')),
        value(8, char('⁸')),
        value(9, char('⁹')),
    ))(input)
}

fn sign(input: &str) -> IResult<&str, i32> {
    alt((value(-1, char('-')), value(1, char('+'))))(input)
}

/* Macro's used to build the parser. */

macro_rules! units {
    ( $input:ident, [ $($unit:expr, $parser:expr),+] ) => {
	units!($input, Err(ErrorKind::Alt), [ $($unit,$parser),* ])
    };
    ( $input:ident, $default:expr, [ $($unit:expr, $parser:expr),+] ) => {
	$(match terminated($parser,unit_end)($input) {
	    Err(nom::Err::Error(_)) => {},
	    Ok((input,_)) => return Ok((input,$unit)),
	    Err(err) => return Err(err)
	})*;
	match $default {
	    Ok(result) => return Ok(($input,result)),
	    Err(kind) => return Err(nom::Err::Error(nom::error::Error {
		input: $input, code: kind }))
	}
    }
}

macro_rules! prefixes {
    ( $unit:ident, $default:expr, [ $( $prefix:expr, $parser:expr ),+ ] ) => {
	move |input| {
	    $(if let Ok((input,_)) = $parser(input) {
		match $unit(input, $prefix) {
		    Err(nom::Err::Error(_)) => {},
		    res => return res
		}
	    })*
	    return $unit(input, $default);
	}
    }
}

/* Unit parsers. The prefixed versions take the prefix
 * found by the prefix parser. */

fn nonprefixed_unit(input: &str) -> IResult<&str, Unit> {
    units!(
        input,
        [
            Unit::Time(TimeUnit::Minute),
            alt((tag("minutes"), tag("minute"), tag("min"))),
            Unit::Time(TimeUnit::Hour),
            alt((tag("hours"), tag("hour"), tag("h"))),
            Unit::Time(TimeUnit::Day),
            alt((tag("days"), tag("day"))),
            Unit::Time(TimeUnit::Week),
            alt((tag("weeks"), tag("week"))),
            Unit::Temperature(TemperatureUnit::Kelvin),
            char('K'),
            Unit::Temperature(TemperatureUnit::Celsius),
            tag("°C"),
            Unit::Temperature(TemperatureUnit::Fahrenheit),
            tag("°F"),
            Unit::FanSpeed(FanSpeedUnit::RPM),
            tag("rpm"),
            Unit::FanSpeed(FanSpeedUnit::RPS),
            tag("rps"),
            Unit::Power(PowerUnit::DBmW),
            tuple((tag("dBm"), opt(char('W')))),
            Unit::Dimensionless(DimensionlessUnit::Percent),
            char('%'),
            Unit::Dimensionless(DimensionlessUnit::Permille),
            char('‰')
        ]
    );
}

fn si_unit(input: &str, prefix: SiPrefix) -> IResult<&str, Unit> {
    units!(
        input,
        [
            Unit::Length(LengthUnit::Meter(prefix)),
            char('m'),
            Unit::Mass(MassUnit::Gram(prefix)),
            char('g'),
            Unit::Current(CurrentUnit::Ampere(prefix)),
            char('A'),
            Unit::Potential(PotentialUnit::Volt(prefix)),
            char('V'),
            Unit::Power(PowerUnit::Watt(prefix)),
            char('W'),
            Unit::Resistance(ResistanceUnit::Ohm(prefix)),
            char('Ω'),
            Unit::Conductivity(ConductivityUnit::Siemens(prefix)),
            char('S'),
            Unit::Frequency(FrequencyUnit::Hertz(prefix)),
            tag("Hz")
        ]
    );
}

fn frac_unit(input: &str, prefix: FracPrefix) -> IResult<&str, Unit> {
    units!(input, [Unit::Time(TimeUnit::Second(prefix)), tag("s")]);
}

fn dec_unit(input: &str, prefix: DecPrefix) -> IResult<&str, Unit> {
    units!(
        input,
        [
            Unit::Bandwidth(
                InformationUnit::Bit(prefix),
                TimeUnit::Second(FracPrefix::Unit)
            ),
            tag("bps"),
            Unit::Operations(OperationUnit::Operation(prefix)),
            tuple((tag("op"), opt(char('s')))),
            Unit::Information(InformationUnit::Bit(prefix)),
            char('b')
        ]
    );
}

fn bin_unit(input: &str, prefix: BinPrefix) -> IResult<&str, Unit> {
    units!(
        input,
        [
            Unit::Bandwidth(
                InformationUnit::Byte(prefix),
                TimeUnit::Second(FracPrefix::Unit)
            ),
            tag("Bps"),
            Unit::Information(InformationUnit::Byte(prefix)),
            char('B')
        ]
    );
}

/* Prefix parsers. These take a unit parser so that
 * they can backtrack on failure. */

fn si_prefix<F>(unit: F) -> impl Fn(&str) -> IResult<&str, Unit>
where
    F: Fn(&str, SiPrefix) -> IResult<&str, Unit>,
{
    prefixes!(
        unit,
        SiPrefix::Unit,
        [
            SiPrefix::Yocto,
            char('y'),
            SiPrefix::Zepto,
            char('z'),
            SiPrefix::Atto,
            char('a'),
            SiPrefix::Femto,
            char('f'),
            SiPrefix::Pico,
            char('p'),
            SiPrefix::Nano,
            char('n'),
            SiPrefix::Micro,
            alt((char('µ'), char('μ'))),
            SiPrefix::Milli,
            char('m'),
            SiPrefix::Centi,
            char('c'),
            SiPrefix::Deci,
            char('d'),
            SiPrefix::Deca,
            tag("da"),
            SiPrefix::Hecto,
            char('h'),
            SiPrefix::Kilo,
            char('k'),
            SiPrefix::Mega,
            char('M'),
            SiPrefix::Giga,
            char('G'),
            SiPrefix::Tera,
            char('T'),
            SiPrefix::Peta,
            char('P'),
            SiPrefix::Exa,
            char('E'),
            SiPrefix::Zetta,
            char('Z'),
            SiPrefix::Yotta,
            char('Y')
        ]
    )
}

fn frac_prefix<F>(unit: F) -> impl Fn(&str) -> IResult<&str, Unit>
where
    F: Fn(&str, FracPrefix) -> IResult<&str, Unit>,
{
    prefixes!(
        unit,
        FracPrefix::Unit,
        [
            FracPrefix::Yocto,
            char('y'),
            FracPrefix::Zepto,
            char('z'),
            FracPrefix::Atto,
            char('a'),
            FracPrefix::Femto,
            char('f'),
            FracPrefix::Pico,
            char('p'),
            FracPrefix::Nano,
            char('n'),
            FracPrefix::Micro,
            alt((char('µ'), char('μ'))),
            FracPrefix::Milli,
            char('m')
        ]
    )
}

fn dec_prefix<F>(unit: F) -> impl Fn(&str) -> IResult<&str, Unit>
where
    F: Fn(&str, DecPrefix) -> IResult<&str, Unit>,
{
    prefixes!(
        unit,
        DecPrefix::Unit,
        [
            DecPrefix::Kilo,
            char('k'),
            DecPrefix::Mega,
            char('M'),
            DecPrefix::Giga,
            char('G'),
            DecPrefix::Tera,
            char('T'),
            DecPrefix::Peta,
            char('P'),
            DecPrefix::Exa,
            char('E'),
            DecPrefix::Zetta,
            char('Z'),
            DecPrefix::Yotta,
            char('Y')
        ]
    )
}

fn bin_prefix<F>(unit: F) -> impl Fn(&str) -> IResult<&str, Unit>
where
    F: Fn(&str, BinPrefix) -> IResult<&str, Unit>,
{
    prefixes!(
        unit,
        BinPrefix::Unit,
        [
            BinPrefix::Kilo,
            char('k'),
            BinPrefix::Mega,
            char('M'),
            BinPrefix::Giga,
            char('G'),
            BinPrefix::Tera,
            char('T'),
            BinPrefix::Peta,
            char('P'),
            BinPrefix::Exa,
            char('E'),
            BinPrefix::Zetta,
            char('Z'),
            BinPrefix::Yotta,
            char('Y')
        ]
    )
}

/* Verify that all unit characters have been consumed. */
fn unit_end(input: &str) -> IResult<&str, ()> {
    match input.chars().next().map_or(false, |c| c.is_alphabetic()) {
        true => Err(nom::Err::Error(nom::error::Error {
            input,
            code: ErrorKind::Eof,
        })),
        false => Ok((input, ())),
    }
}

/* Monomorphised versions of char and tag. */

fn char<'r>(t: char) -> impl Fn(&'r str) -> IResult<&'r str, char> {
    nom::character::complete::char(t)
}

fn tag<'r>(t: &'static str) -> impl Fn(&'r str) -> IResult<&'r str, &'r str> {
    nom::bytes::complete::tag(t)
}
