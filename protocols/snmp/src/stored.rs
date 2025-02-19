/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::collections::HashMap;
use nom::{IResult,
	  error::{ParseError,ContextError,VerboseError,ErrorKind,VerboseErrorKind},
	  bytes::complete::{tag,take_while},
	  character::complete::{char,digit1,hex_digit1,alphanumeric1},
	  sequence::{tuple,preceded,terminated,delimited},
	  combinator::{cut,opt,iterator},
	  multi::{many0,many1},
	  branch::alt};
use nom_locate::LocatedSpan;
use netsnmp::{Oid,Value};
use crate::error::{Result,Error};
use super::SNMPError;


type I<'a> = LocatedSpan<&'a [u8]>;

pub fn parse_snmp_walk<'a>(input: &'a [u8]) -> Result<HashMap<Oid,Value>> {
    match parse_walk::<VerboseError<I<'a>>>(I::new(input)) {
	Ok((s,m)) => match s.fragment() {
	    &b"" => Ok(m),
	    r => Err(Error::Protocol(SNMPError::IncompleteParseStored(
		s.location_line(), String::from_utf8_lossy(&r[0..15]).to_string()
	    ).into()))
	},
	Err(nom::Err::Error(e)) | Err(nom::Err::Failure(e))
	    => Err(Error::Protocol(SNMPError::StoredParseError(
		e.errors.into_iter().map(|(s,k)| format!(
		    "\tline {}:{}: {}", s.location_line(), s.get_utf8_column(),
		    match k {
			VerboseErrorKind::Nom(e) => format!("expected: {:?}", e),
			VerboseErrorKind::Context(c) => format!("expected: {}", c),
			VerboseErrorKind::Char(c) => format!("expected: {}", c),
		    }
		)).collect::<Vec<_>>().join("\n")
	    ).into())),
	Err(nom::Err::Incomplete(_))
	    => Err(Error::Protocol(SNMPError::IncompleteStoredWalk.into()))
    }
}


fn parse_walk<'a,E:Clone>(input: I<'a>) -> IResult<I<'a>,HashMap<Oid,Value>,E>
where E: ParseError<I<'a>> + ContextError<I<'a>> {
    let mut m = HashMap::new();
    let mut iter = iterator(input, parse_var);
    for (oid,value) in &mut iter {
	m.insert(oid, value);
    }
    let (input,_) = iter.finish()?;
    Ok((input, m))
}

fn parse_var<'a,E>(input: I<'a>) -> IResult<I<'a>,(Oid,Value),E>
where E: ParseError<I<'a>> + ContextError<I<'a>> {
    let (input,oid) = parse_oid(input)?;
    let (input,_) = cut(tag(" = "))(input)?;
    let (input,value) = cut(parse_value)(input)?;
    let (input,_) = cut(char('\n'))(input)?;
    Ok((input,(oid,value)))
}

fn parse_oid<'a,E>(input: I<'a>) -> IResult<I<'a>,Oid,E>
where E: ParseError<I<'a>> {
    let (input,ns) = many1(preceded(char('.'), decimal_u64))(input)?;
    Ok((input,Oid::from_slice(&ns)))
}

fn parse_value<'a,E>(input: I<'a>) -> IResult<I<'a>,Value,E>
where E: ParseError<I<'a>> + ContextError<I<'a>> {
    if let Ok((input,_)) = tag::<_,_,E>(b"INTEGER: " as &[u8])(input) {
	let (input,val) = alt((terminated(decimal_i64,
					  opt(tuple((char(' '),alphanumeric1)))),
			       delimited(tuple((alphanumeric1,char('('))),
					 decimal_i64,char(')'))))(input)?;
	Ok((input,Value::Integer(val)))
    } else if let Ok((input,_)) = tag::<_,_,E>(b"Integer64: " as &[u8])(input) {
	let (input,val) = terminated(decimal_i64,
				     opt(tuple((char(' '),alphanumeric1))))(input)?;
	Ok((input,Value::Integer64(val)))
    } else if let Ok((input,_)) = tag::<_,_,E>(b"Unsigned64: " as &[u8])(input) {
	let (input,val) = terminated(decimal_u64,
				     opt(tuple((char(' '),alphanumeric1))))(input)?;
	Ok((input,Value::Unsigned64(val)))
    } else if let Ok((input,_)) = tag::<_,_,E>(b"Gauge32: " as &[u8])(input) {
	let (input,val) = terminated(decimal_u64,
				     opt(tuple((char(' '),alphanumeric1))))(input)?;
	Ok((input,Value::Gauge(val)))
    } else if let Ok((input,_)) = tag::<_,_,E>(b"Gauge64: " as &[u8])(input) {
	let (input,val) = terminated(decimal_u64,
				     opt(tuple((char(' '),alphanumeric1))))(input)?;
	Ok((input,Value::Gauge(val)))
    } else if let Ok((input,_)) = tag::<_,_,E>(b"Counter32: " as &[u8])(input) {
	let (input,val) = terminated(decimal_u64,
				     opt(tuple((char(' '),alphanumeric1))))(input)?;
	Ok((input,Value::Counter(val)))
    } else if let Ok((input,_)) = tag::<_,_,E>(b"Counter64: " as &[u8])(input) {
	let (input,val) = terminated(decimal_u64,
				     opt(tuple((char(' '),alphanumeric1))))(input)?;
	Ok((input,Value::Counter(val)))
    } else if let Ok((input,_)) = tag::<_,_,E>(b"STRING: "  as &[u8])(input) {
	let (input,line) = take_while(|c| c != b'\n')(input)?;
	Ok((input,Value::OctetStr(line.fragment().to_vec())))
    } else if let Ok((input,_)) = tag::<_,_,E>(b"Hex-STRING: " as &[u8])(input) {
	let (input,val) = many0(terminated(hexadecimal_u8,char(' ')))(input)?;
	Ok((input,Value::OctetStr(val.to_vec())))
    } else if let Ok((input,_)) = tag::<_,_,E>(b"Oid: " as &[u8])(input) {
	let (input,oid) = parse_oid(input)?;
	Ok((input,Value::Oid(oid)))
    } else if let Ok((input,_)) = tag::<_,_,E>(b"Timeticks: " as &[u8])(input) {
	let (input,_) = char('(')(input)?;
	let (input,val) = decimal_u64(input)?;
	let (input,_) = char(')')(input)?;
	let (input,_) = take_while(|c| c != b'\n')(input)?;
	Ok((input,Value::TimeTicks(val)))
    } else if let Ok((input,_)) = tag::<_,_,E>(b"IpAddress: " as &[u8])(input) {
	let (input,a) = decimal_u8(input)?;
	let (input,_) = char('.')(input)?;
	let (input,b) = decimal_u8(input)?;
	let (input,_) = char('.')(input)?;
	let (input,c) = decimal_u8(input)?;
	let (input,_) = char('.')(input)?;
	let (input,d) = decimal_u8(input)?;
	Ok((input,Value::IpAddress( (a as u32) << 24 | (b as u32) << 16
				     | (c as u32) << 8 | (d as u32) )))
    } else if let Ok((input,_)) = tag::<_,_,E>(b"\"\"" as &[u8])(input) {
	Ok((input,Value::OctetStr(Vec::new()))) // ???
    } else {
	Err(nom::Err::Error(E::add_context(
	    input, "SNMP type", E::from_error_kind(input,ErrorKind::Alt))))
    }
}


fn decimal_u8<'a,E>(input: I<'a>) -> IResult<I<'a>,u8,E>
where E: ParseError<I<'a>> {
    let (input,ds) = digit1(input)?;
    Ok((input,ds.fragment().into_iter().fold(0, |n,d| n * 10 + (d - b'0'))))
}

fn decimal_u64<'a,E>(input: I<'a>) -> IResult<I<'a>,u64,E>
where E: ParseError<I<'a>> {
    let (input,ds) = digit1(input)?;
    Ok((input,ds.fragment().into_iter().fold(0, |n,d| n * 10 + (d - b'0') as u64)))
}

fn decimal_i64<'a,E>(input: I<'a>) -> IResult<I<'a>,i64,E>
where E: ParseError<I<'a>> {
    let (input,sign) = opt(char('-'))(input)?;
    let (input,ds) = digit1(input)?;
    Ok((input, match sign { Some('-') => -1, _ => 1 }
	* ds.fragment().into_iter().fold(0, |n,d| n * 10 + (d - b'0') as i64)))
}

fn hexadecimal_u8<'a,E>(input: I<'a>) -> IResult<I<'a>,u8,E>
where E: ParseError<I<'a>> {
    let (input,ds) = hex_digit1(input)?;
    Ok((input, ds.fragment().into_iter().fold(0, |n,d| n * 10 + match d {
	b'0'..=b'9' => d - b'0',
	b'A'..=b'F' => d - b'F',
	b'a'..=b'f' => d - b'a',
	_ => unreachable!()
    })))
}
