/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::io;
use std::process;
use std::io::{Read,Write};

use nom::{
    IResult,
    AsChar,
    InputLength,
    InputTake,
    InputTakeAtPosition,
    multi::{many1},
    bytes::complete::{tag,take_till1},
    character::complete::{char,digit1,space1,newline},
};

use serde::Serialize;

use agent::Value;


#[derive(Serialize,Debug)]
pub struct Tables {
    df: Vec<Df>
}

#[derive(Serialize,Debug)]
pub struct Df {
    pub filesystem: Value,
    pub size: Value,
    pub used: Value,
    pub avail: Value,
    pub mount_point: Value,
}


fn main() {
    if let Err(err) = run() {
	let _ = io::stderr().write(&err.as_bytes());
	process::exit(1);
    }
}

fn run() -> Result<(),String> {
    let mut data = String::new();
    io::stdin().read_to_string(&mut data).map_err(|e| format!("Failed to read input: {}", e))?;
    io::stdout().write(&serde_json::to_string(&parse_df(&data)?).map_err(
	|e| format!("JSON encoding failed: {}", e))?.as_bytes()).map_err(
	|e| format!("Failed to write to stdout: {}", e))?;
    Ok(())
}


pub fn parse_df(input: &str) -> Result<Tables,String> {
    match parse(input) {
	Ok(("",res)) => Ok(Tables {df: res}),
	Ok((rest,_)) => Err(format!("Leftover input: {}", rest)),
	Err(e) => Err(format!("{}", e)),
    }
}


fn parse(s: &str) -> IResult<&str,Vec<Df>> {
    let (s,_) = verify_headers(s)?;
    let (s,res) = many1(parse_row)(s)?;
    Ok((s,res))
}

fn verify_headers(s: &str) -> IResult<&str,()> {
    let (s,_) = tag("Filesystem")(s)?;
    let (s,_) = space1(s)?;
    let (s,_) = tag("1K-blocks")(s)?;
    let (s,_) = space1(s)?;
    let (s,_) = tag("Used")(s)?;
    let (s,_) = space1(s)?;
    let (s,_) = tag("Available")(s)?;
    let (s,_) = space1(s)?;
    let (s,_) = tag("Use%")(s)?;
    let (s,_) = space1(s)?;
    let (s,_) = tag("Mounted on")(s)?;
    let (s,_) = newline(s)?;
    Ok((s,()))
}

fn parse_row(s: &str) -> IResult<&str,Df> {

    parse_nongreedy_string1(|filesystem, s: &str| {

	let (s,_) = space1(s)?;
	let (s,size) = parse_int(s)?;
	let (s,_) = space1(s)?;
	let (s,used) = parse_int(s)?;
	let (s,_) = space1(s)?;
	let (s,avail) = parse_int(s)?;
	let (s,_) = space1(s)?;
	let (s,_used_perc) = parse_int(s)?;
	let (s,_) = char('%')(s)?;
	let (s,_) = space1(s)?;
	let (s,mount_point) = take_till1(|c| c == '\n')(s)?;
	let (s,_) = newline(s)?;

	Ok((s, Df {
	    filesystem: Value::String(filesystem.as_bytes().to_vec()),
	    mount_point: Value::String(mount_point.as_bytes().to_vec()),
	    size: Value::Integer(size as i64),
	    used: Value::Integer(used as i64),
	    avail: Value::Integer(avail as i64),
	}))

    })(s)

}

fn parse_nongreedy_string1<P,I,O>(next: P) -> impl Fn(I) -> IResult<I,O>
where P: Fn(I,I) -> IResult<I,O>, I: InputLength + InputTake + InputTakeAtPosition + Copy,
      <I as InputTakeAtPosition>::Item: AsChar + PartialEq<char> + Clone
{

    let cond = |c| c == ' ' || c == '\t' || c == '\n';
    
    move |s0:I| {

	let (mut s,mut r) = take_till1(cond)(s0)?;
	let mut l = r.input_len();
	
	loop {

	    if let Ok(x) = next(r,s) {
		return Ok(x)
	    }

	    let (s1,q) = space1(s)?;
	    let (s2,p) = take_till1(cond)(s1)?;

	    l += q.input_len() + p.input_len();
	    r = s0.take(l);
	    s = s2;

	}

    }
}

/*fn parse_size(s: &str) -> IResult<&str,u64> {
    let (s,n) = parse_float(s)?;
    let (s,unit) = alt((
	value(1024*1024*1024, char('G')),
	value(1024*1024, char('M')),
	value(1024, char('K')),
	value(1,take(0usize))))(s)?;
    Ok((s, (n * unit as f64) as u64))
}*/

fn parse_int(s: &str) -> IResult<&str,u64> {
    let (s,i) = digit1(s)?;
    Ok((s,i.parse().unwrap()))
}

/*fn parse_float(s: &str) -> IResult<&str,f64> {
    let (s,i) = digit1(s)?;
    if let (s,Some(_)) = opt(char('.'))(s)? {
	let (s,f) = digit1(s)?;
	Ok((s,i.parse::<u64>().unwrap() as f64
	    + f.parse::<u64>().unwrap() as f64
	    / 10f64.powf(f.len() as f64)))
    } else {
	Ok((s,i.parse().unwrap()))
    }
}*/
