/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::str::FromStr;
//use std::net::IpAddr;

use nom::{
    bytes::complete::take_while1,
    character::complete::char,
    combinator::{eof, map_res, opt},
    sequence::{preceded, terminated},
    Finish, IResult,
};

use super::error::Result;

/// Structure to receive parsed host argument.
pub struct Host<'a> {
    user: Option<&'a str>,
    host_name: &'a str,
    port: u32,
}

impl<'a> Host<'a> {
    pub fn parse(arg: &'a str) -> Result<Self> {
        Ok(Finish::finish(terminated(parse_host, eof)(arg))?.1)
    }

    pub fn conn_string(&self) -> String {
        format!("{}:{}", self.host_name, self.port)
    }

    pub fn port(&self) -> u32 {
        self.port
    }

    pub fn host_name(&self) -> &str {
        self.host_name
    }

    pub fn user(&self) -> &str {
        self.user.unwrap_or("root")
    }

    /*pub async fn ip_addr(&self) -> Result<IpAddr> {
    Ok(tokio::net::lookup_host(self.conn_string()).await
       .map_err(|e| Error::ResolutionFailed(self.host_name.to_string(),e))?.next()
       .ok_or_else(|| Error::ResolutionEmpty(self.host_name.to_string()))?.ip())
    }*/
}

fn parse_host(input: &str) -> IResult<&str, Host> {
    let (input, user) =
        opt(terminated(take_while1(|c| c != '@'), char('@')))(input)?;
    let (input, host_name) = take_while1(|c| c != ':')(input)?;
    let (input, port) = opt(map_res(
        preceded(char(':'), take_while1(|c: char| c.is_ascii_digit())),
        u32::from_str,
    ))(input)?;
    Ok((
        input,
        Host {
            user,
            host_name,
            port: port.unwrap_or(22),
        },
    ))
}
