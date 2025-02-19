/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use nom::{
    branch::alt,
    bytes::complete::{tag, take_while1},
    character::complete::{char, digit1},
    combinator::{eof, map, map_res, opt, recognize, value},
    multi::many0,
    sequence::{terminated, tuple},
    Finish, IResult,
};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use tokio::process::Command;

use super::error::{Error, ParseBytesError, ParseBytesResult, Result};

#[derive(
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Debug,
    Clone,
    Copy,
)]
#[serde(rename_all = "lowercase")]
pub enum NPingMode {
    Icmp,
    Tcp,
    Udp,
    Arp,
    #[serde(rename = "tcp-connect")]
    TcpConnect,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct NPingStats {
    pub rtt: RttStats,
    pub pkts: PacketStats,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RttStats {
    pub max_rtt: Option<f64>,
    pub min_rtt: Option<f64>,
    pub avg_rtt: Option<f64>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PacketStats {
    pub sent_pkts: u64,
    pub sent_bytes: u64,
    pub rcvd_pkts: u64,
    pub rcvd_bytes: u64,
    pub lost_pkts: u64,
    pub lost_pkts_rel: f64,
}

pub async fn nping_host(host: &str, mode: NPingMode) -> Result<NPingStats> {
    let mut nping_cmd = Command::new("nping");
    nping_cmd
        .arg(match mode {
            NPingMode::Arp => "--arp",
            NPingMode::Icmp => "--icmp",
            NPingMode::Tcp => "--tcp",
            NPingMode::Udp => "--udp",
            NPingMode::TcpConnect => "--tcp-connect",
        })
        .arg(host)
        .kill_on_drop(true);

    let nping_out = nping_cmd.output().await?;

    match nping_out.status.code() {
        Some(0) => Ok(parse_nping_output(nping_out.stdout.as_slice())?),
        c => Err(Error::NonZeroExitStatus(
            c,
            String::from_utf8_lossy(&nping_out.stderr).into_owned(),
        )),
    }
}

fn parse_nping_output(input: &[u8]) -> Result<NPingStats> {
    match Finish::finish(terminated(nping_output, eof)(input)) {
        Ok((_, stats)) => Ok(stats),
        Err(e) => {
            let loc = String::from_utf8_lossy(e.input);
            Err(Error::Parse(
                "nping",
                format!(
                    "{} at \"{}{}\"",
                    e.code.description(),
                    loc.chars().take(10).collect::<String>(),
                    if loc.len() > 10 { "..." } else { "" }
                ),
            ))
        }
    }
}

fn nping_output(input: &[u8]) -> IResult<&[u8], NPingStats> {
    /* Example output:
     *
     * Starting Nping 0.7.80 ( https://nmap.org/nping ) at 2021-05-06 11:33 CEST
     * Max rtt: 0.056ms | Min rtt: 0.014ms | Avg rtt: 0.033ms
     * Raw packets sent: 5 (140B) | Rcvd: 5 (140B) | Lost: 0 (0.00%)
     * Nping done: 1 IP address pinged in 4.09 seconds
     */

    let (input, _) = skip_header(input)?;
    let (input, _) = skip_packets(input)?;
    let (input, rtt) = rtt_stats(input)?;
    let (input, pkts) = alt((packet_stats, connection_stats))(input)?;
    let (input, _) = skip_summary(input)?;

    Ok((input, NPingStats { rtt, pkts }))
}

fn skip_header(input: &[u8]) -> IResult<&[u8], ()> {
    /* Starting Nping 0.7.80 ( https://nmap.org/nping ) at 2021-05-06 11:33 CEST */
    let (input, _) = many0(char('\n'))(input)?;
    let (input, _) = tag("Starting Nping ")(input)?;
    let (input, _) = take_while1(|c| c != b'\n')(input)?;
    let (input, _) = char('\n')(input)?;
    Ok((input, ()))
}

fn skip_packets(input: &[u8]) -> IResult<&[u8], ()> {
    let (input, _) = many0(skip_packet)(input)?;
    let (input, _) = tag(" \n")(input)?;
    Ok((input, ()))
}

fn skip_packet(input: &[u8]) -> IResult<&[u8], ()> {
    /* SENT (0.0368s) ICMP [127.0.0.1 > 127.0.0.1 Echo request (type=8/code=0) \
     * id=65315 seq=1] IP [ttl=64 id=57837 iplen=28 ]
     * RCVD (0.0368s) ICMP [127.0.0.1 > 127.0.0.1 Echo reply (type=0/code=0) \
     * id=65315 seq=1] IP [ttl=64 id=6419 iplen=28 ]
     */
    let (input, _) = alt((tag(b"SENT "), tag(b"RCVD ")))(input)?;
    let (input, _) = take_while1(|c| c != b'\n')(input)?;
    let (input, _) = char('\n')(input)?;
    Ok((input, ()))
}

fn skip_summary(input: &[u8]) -> IResult<&[u8], ()> {
    /* Nping done: 1 IP address pinged in 4.09 seconds */
    let (input, _) = tag("Nping done: ")(input)?;
    let (input, _) = take_while1(|c| c != b'\n')(input)?;
    let (input, _) = char('\n')(input)?;
    /*let (input,_addrs) = integer(input)?;
    let (input,_) = tag(" IP address pinged in ")(input)?;
    let (input,_time) = double(input)?;
    let (input,_) = tag(" seconds\n")(input)?;*/
    Ok((input, ()))
}

fn rtt_stats(input: &[u8]) -> IResult<&[u8], RttStats> {
    /* Max rtt: 0.115ms | Min rtt: 0.026ms | Avg rtt: 0.082ms */
    let (input, _) = tag("Max rtt: ")(input)?;
    let (input, max_rtt) = maybe(duration)(input)?;
    let (input, _) = tag(" | Min rtt: ")(input)?;
    let (input, min_rtt) = maybe(duration)(input)?;
    let (input, _) = tag(" | Avg rtt: ")(input)?;
    let (input, avg_rtt) = maybe(duration)(input)?;
    let (input, _) = char('\n')(input)?;
    Ok((
        input,
        RttStats {
            max_rtt,
            min_rtt,
            avg_rtt,
        },
    ))
}

fn packet_stats(input: &[u8]) -> IResult<&[u8], PacketStats> {
    /* Raw packets sent: 5 (140B) | Rcvd: 5 (140B) | Lost: 0 (0.00%) */
    let (input, _) = tag("Raw packets sent: ")(input)?;
    let (input, sent_pkts) = integer(input)?;
    let (input, _) = tag(" (")(input)?;
    let (input, sent_bytes) = bytes(input)?;
    let (input, _) = tag(") | Rcvd: ")(input)?;
    let (input, rcvd_pkts) = integer(input)?;
    let (input, _) = tag(" (")(input)?;
    let (input, rcvd_bytes) = bytes(input)?;
    let (input, _) = tag(") | Lost: ")(input)?;
    let (input, lost_pkts) = integer(input)?;
    let (input, _) = tag(" (")(input)?;
    let (input, lost_pkts_rel) = percentage(input)?;
    let (input, _) = tag(")\n")(input)?;
    Ok((
        input,
        PacketStats {
            sent_pkts,
            sent_bytes,
            rcvd_pkts,
            rcvd_bytes,
            lost_pkts,
            lost_pkts_rel,
        },
    ))
}

fn connection_stats(input: &[u8]) -> IResult<&[u8], PacketStats> {
    /* TCP connection attempts: 5 | Successful connections: 5 | Failed: 0 (0.00%) */
    let (input, _) = tag("TCP connection attempts: ")(input)?;
    let (input, sent_pkts) = integer(input)?;
    let (input, _) = tag(" | Successful connections: ")(input)?;
    let (input, rcvd_pkts) = integer(input)?;
    let (input, _) = tag(" | Failed: ")(input)?;
    let (input, lost_pkts) = integer(input)?;
    let (input, _) = tag(" (")(input)?;
    let (input, lost_pkts_rel) = percentage(input)?;
    let (input, _) = tag(")\n")(input)?;
    Ok((
        input,
        PacketStats {
            sent_pkts,
            sent_bytes: 0,
            rcvd_pkts,
            rcvd_bytes: 0,
            lost_pkts,
            lost_pkts_rel,
        },
    ))
}

fn duration(input: &[u8]) -> IResult<&[u8], f64> {
    /* 0.056ms */
    let (input, n) = double(input)?;
    let (input, m) = value(0.001, tag("ms"))(input)?;
    Ok((input, n * m))
}

fn bytes(input: &[u8]) -> IResult<&[u8], u64> {
    /* 140B, 1.344KB */
    alt((
        terminated(integer, tag("B")),
        map(terminated(double, tag("KB")), |n| {
            (n * (1 << 10) as f64) as u64
        }),
        map(terminated(double, tag("MB")), |n| {
            (n * (1 << 20) as f64) as u64
        }),
        map(terminated(double, tag("GB")), |n| {
            (n * (1 << 30) as f64) as u64
        }),
    ))(input)
}

fn percentage(input: &[u8]) -> IResult<&[u8], f64> {
    /* 0.00% */
    map(terminated(double, tag(b"%")), |n| n)(input)
}

fn double(input: &[u8]) -> IResult<&[u8], f64> {
    /* A floating-point number without extras. */
    map_res(
        alt((
            recognize(tuple((digit1, opt(tuple((char('.'), digit1)))))),
            recognize(tuple((char('.'), digit1))),
        )),
        parse_bytes,
    )(input)
}

fn integer(input: &[u8]) -> IResult<&[u8], u64> {
    map_res(digit1, parse_bytes)(input)
}

fn maybe<'a, P, R>(
    parser: P,
) -> impl FnMut(&'a [u8]) -> IResult<&'a [u8], Option<R>>
where
    P: Fn(&'a [u8]) -> IResult<&'a [u8], R>,
    R: Clone,
{
    alt((value(None, tag("N/A")), map(parser, Some)))
}

fn parse_bytes<T>(input: &[u8]) -> ParseBytesResult<T, T::Err>
where
    T: FromStr,
    T::Err: std::error::Error,
{
    std::str::from_utf8(input)
        .map_err(ParseBytesError::Utf8)?
        .parse()
        .map_err(ParseBytesError::Parse)
}

#[cfg(test)]
mod tests {

    use super::parse_nping_output;

    const EXAMPLES: &[&str] = &[
        r#"
Starting Nping 0.7.80 ( https://nmap.org/nping ) at 2021-05-06 16:20 CEST
SENT (0.0555s) ICMP [127.0.0.1 > 127.0.0.1 Echo request (type=8/code=0) id=48353 seq=1] IP [ttl=64 id=20173 iplen=28 ]
RCVD (0.0555s) ICMP [127.0.0.1 > 127.0.0.1 Echo reply (type=0/code=0) id=48353 seq=1] IP [ttl=64 id=60263 iplen=28 ]
SENT (1.0558s) ICMP [127.0.0.1 > 127.0.0.1 Echo request (type=8/code=0) id=48353 seq=2] IP [ttl=64 id=20173 iplen=28 ]
RCVD (1.0560s) ICMP [127.0.0.1 > 127.0.0.1 Echo reply (type=0/code=0) id=48353 seq=2] IP [ttl=64 id=60270 iplen=28 ]
SENT (2.0573s) ICMP [127.0.0.1 > 127.0.0.1 Echo request (type=8/code=0) id=48353 seq=3] IP [ttl=64 id=20173 iplen=28 ]
RCVD (2.0576s) ICMP [127.0.0.1 > 127.0.0.1 Echo reply (type=0/code=0) id=48353 seq=3] IP [ttl=64 id=60438 iplen=28 ]
SENT (3.0589s) ICMP [127.0.0.1 > 127.0.0.1 Echo request (type=8/code=0) id=48353 seq=4] IP [ttl=64 id=20173 iplen=28 ]
RCVD (3.0591s) ICMP [127.0.0.1 > 127.0.0.1 Echo reply (type=0/code=0) id=48353 seq=4] IP [ttl=64 id=60557 iplen=28 ]
SENT (4.0603s) ICMP [127.0.0.1 > 127.0.0.1 Echo request (type=8/code=0) id=48353 seq=5] IP [ttl=64 id=20173 iplen=28 ]
RCVD (4.0605s) ICMP [127.0.0.1 > 127.0.0.1 Echo reply (type=0/code=0) id=48353 seq=5] IP [ttl=64 id=60621 iplen=28 ]
 
Max rtt: 0.104ms | Min rtt: 0.033ms | Avg rtt: 0.069ms
Raw packets sent: 5 (140B) | Rcvd: 5 (140B) | Lost: 0 (0.00%)
Nping done: 1 IP address pinged in 4.09 seconds
"#,
        r#"
Starting Nping 0.7.80 ( https://nmap.org/nping ) at 2021-05-06 16:21 CEST
SENT (0.0643s) ICMP [192.168.10.30 > 128.0.0.1 Echo request (type=8/code=0) id=18365 seq=1] IP [ttl=64 id=30143 iplen=28 ]
SENT (1.0647s) ICMP [192.168.10.30 > 128.0.0.1 Echo request (type=8/code=0) id=18365 seq=2] IP [ttl=64 id=30143 iplen=28 ]
SENT (2.0661s) ICMP [192.168.10.30 > 128.0.0.1 Echo request (type=8/code=0) id=18365 seq=3] IP [ttl=64 id=30143 iplen=28 ]
SENT (3.0673s) ICMP [192.168.10.30 > 128.0.0.1 Echo request (type=8/code=0) id=18365 seq=4] IP [ttl=64 id=30143 iplen=28 ]
SENT (4.0688s) ICMP [192.168.10.30 > 128.0.0.1 Echo request (type=8/code=0) id=18365 seq=5] IP [ttl=64 id=30143 iplen=28 ]
 
Max rtt: N/A | Min rtt: N/A | Avg rtt: N/A
Raw packets sent: 5 (140B) | Rcvd: 0 (0B) | Lost: 5 (100.00%)
Nping done: 1 IP address pinged in 5.13 seconds
"#,
        r#"
Starting Nping 0.7.80 ( https://nmap.org/nping ) at 2021-05-10 15:38 CEST
SENT (0.0011s) Starting TCP Handshake > 127.0.0.1:80
RCVD (0.0011s) Handshake with 127.0.0.1:80 completed
SENT (1.0022s) Starting TCP Handshake > 127.0.0.1:80
RCVD (1.0022s) Handshake with 127.0.0.1:80 completed
SENT (2.0033s) Starting TCP Handshake > 127.0.0.1:80
RCVD (2.0033s) Handshake with 127.0.0.1:80 completed
SENT (3.0044s) Starting TCP Handshake > 127.0.0.1:80
RCVD (3.0044s) Handshake with 127.0.0.1:80 completed
SENT (4.0055s) Starting TCP Handshake > 127.0.0.1:80
RCVD (4.0055s) Handshake with 127.0.0.1:80 completed
 
Max rtt: 0.024ms | Min rtt: 0.006ms | Avg rtt: 0.009ms
TCP connection attempts: 5 | Successful connections: 5 | Failed: 0 (0.00%)
Nping done: 1 IP address pinged in 4.01 seconds
"#,
    ];

    /* Needs tokio features "rt" and "macros" */
    /*#[tokio::test]
    async fn ping_localhost() {
    let _ = nping_host("localhost", NPingMode::TcpConnect)
        .await.expect("nping failed");
    }*/

    #[test]
    fn parse_examples() {
        for output in EXAMPLES {
            parse_nping_output(output.as_bytes())
                .expect("Failed to parse example output");
        }
    }
}
