/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::cmp::Ordering;
use std::path::PathBuf;
use std::{fs, process};

use clap::{ArgGroup, Parser};
use nom::branch::alt;
use nom::bytes::complete::{tag, take_while1};
use nom::character::complete::{char, digit1, hex_digit1};
use nom::combinator::{eof, map, opt, value};
use nom::multi::separated_list1;
use nom::sequence::{preceded, terminated};
use nom::IResult;

use agent_utils::Template;

/// SmartM Agent installer
///
/// Generate a shell script to install and configure the agent.
#[derive(clap::Parser)]
#[clap(author, version, about, group(
	ArgGroup::with_name("connection")
        .required(true)
        .arg("listen")
        .arg("connect")))]
struct Args {
    /// The distribution on which to install.
    #[clap(long)]
    dist: String,
    /// The development environment (dev/tst/acc/prd).
    #[clap(long, default_value = "prd")]
    env: String,
    /// Embed package files into the script to avoid the need for
    /// additional downloads (use for systems without internet access).
    #[clap(long)]
    embed: bool,
    /// Path to the CA certificate.
    #[clap(long)]
    ca: PathBuf,
    /// Path to the agent certificate.
    #[clap(long)]
    cert: PathBuf,
    /// Path to the agent key.
    #[clap(long)]
    key: PathBuf,
    /// The domain name of the broker (for certificate verification).
    #[clap(long)]
    broker: String,
    /// Be compatible with older broker.
    #[clap(long)]
    broker_compat: bool,
    /// Listen for broker connection on this address and port.
    #[clap(long)]
    listen: Option<String>,
    /// Connect to the broker on this address and port.
    #[clap(long)]
    connect: Option<String>,
}

/// Generate a shell script to install and configure the agent.
fn main() {
    let args = Args::parse();

    match args.dist.as_str() {
        "centos7" => {
            let ca_cert = fs::read_to_string(&args.ca).unwrap();
            let agent_cert = fs::read_to_string(&args.cert).unwrap();
            let agent_key = fs::read_to_string(&args.key).unwrap();
            let broker_compat = match args.broker_compat {
                true => " --broker-compat",
                false => "",
            };
            let connect_arg = match args.connect.as_deref() {
                Some(addr) => format!("--connect {}", addr),
                None => format!("--listen {}", args.listen.as_deref().unwrap()),
            };

            match args.embed {
                true => {
                    let repo = format!(
                        "/mnt/fs/Consultants/MonitorNow/Packages/rpm/{}/{}",
                        args.dist, args.env
                    );
                    let package = glob_latest(&format!(
                        "{}/smart-agent-[0-9]*.el7.x86_64.rpm",
                        repo
                    ))
                    .unwrap();
                    let version =
                        package.file_name().unwrap().to_str().unwrap();
                    let version = &version[12..version.len() - 15];

                    let libs_package = PathBuf::from(repo).join(format!(
                        "smart-agent-libs-{}.el7.x86_64.rpm",
                        version
                    ));
                    let agent_rpm = embeddable(fs::read(&package).unwrap());
                    let agent_libs_rpm =
                        embeddable(fs::read(libs_package).unwrap());
                    print!(
                        "{}",
                        Template::parse(std::include_str!(
                            "../scripts/centos7-embedded-rpms.sh"
                        ))
                        .fill(
                            [
                                ("RpmVersion", version),
                                ("SmartAgentRpmBase64", &agent_rpm),
                                ("SmartAgentLibsRpmBase64", &agent_libs_rpm),
                                ("CaCert", &ca_cert),
                                ("AgentCert", &agent_cert),
                                ("AgentKey", &agent_key),
                                ("BrokerArg", args.broker.as_str()),
                                ("BrokerCompat", broker_compat),
                                ("ConnectArg", &connect_arg),
                            ]
                            .iter()
                            .cloned()
                            .collect()
                        )
                    );
                }
                false => {
                    let repo_url = format!(
                        "https://mndev02/smart-agent-repo/{}/{}",
                        args.dist, args.env
                    );
                    print!(
                        "{}",
                        Template::parse(std::include_str!(
                            "../scripts/centos7-via-repo.sh"
                        ))
                        .fill(
                            [
                                ("RepoUrl", repo_url.as_str()),
                                ("CaCert", ca_cert.as_str()),
                                ("AgentCert", agent_cert.as_str()),
                                ("AgentKey", agent_key.as_str()),
                                ("BrokerArg", args.broker.as_str()),
                                ("BrokerCompat", broker_compat),
                                ("ConnectArg", connect_arg.as_str())
                            ]
                            .iter()
                            .cloned()
                            .collect()
                        )
                    );
                }
            }
        }
        _ => {
            eprintln!("unsupported distribution");
            process::exit(1);
        }
    }
}

fn glob_latest(pattern: &str) -> Option<PathBuf> {
    let path = glob::glob(pattern)
        .expect("invalid glob pattern")
        .map(|p| p.unwrap())
        .max_by_key(|p| {
            Some(parse_rpm_file_name(p.file_name()?.to_str()?).ok()?.1)
        });
    if let Some(p) = &path {
        eprintln!("Found rpm: {}", p.display());
    }
    path
}

fn embeddable(data: Vec<u8>) -> String {
    base64::encode(data)
        .into_bytes()
        .chunks(80)
        .map(|line| String::from_utf8(line.to_vec()).unwrap())
        .collect::<Vec<_>>()
        .join("\n")
}

#[derive(Eq, PartialEq, Ord, PartialOrd, Debug)]
struct RpmFileName {
    name: String,
    version: RpmVersion,
    os: String,
    arch: String,
}

#[derive(Eq, PartialEq, Debug)]
struct RpmVersion {
    version: Version,
    tag: Option<Tag>,
    commits: Option<Commits>,
    dirty: bool,
    rpm_patch: u64,
}

// impl RpmVersion {
//     fn environment(&self) -> Environment {
//         match self.commits.is_some() || self.dirty {
//             true => Environment::Dev,
//             false => match &self.tag {
//                 Some(tag) => match tag.env {
//                     EnvTag::Dev => Environment::Dev,
//                     EnvTag::Tst => Environment::Tst,
//                     EnvTag::Acc => Environment::Acc,
//                 },
//                 None => Environment::Prd,
//             },
//         }
//     }
// }
impl Ord for RpmVersion {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}

impl PartialOrd for RpmVersion {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match self.version.partial_cmp(&other.version) {
            Some(Ordering::Equal) => {}
            ord => return ord,
        }
        /* Prefer prod > acc > tst > dev. */
        // match self.environment().partial_cmp(&other.environment()) {
        //     Some(core::cmp::Ordering::Equal) => {}
        //     ord => return ord,
        // }
        match (&self.tag, &other.tag) {
            (None, None) => {}
            (Some(tag), Some(other_tag)) => match tag.partial_cmp(other_tag) {
                Some(Ordering::Equal) => {}
                ord => return ord,
            },
            (None, Some(_)) => return Some(Ordering::Greater),
            (Some(_), None) => return Some(Ordering::Less),
        }
        match self.commits.partial_cmp(&other.commits) {
            Some(Ordering::Equal) => {}
            ord => return ord,
        }
        match self.dirty.partial_cmp(&other.dirty) {
            Some(Ordering::Equal) => {}
            ord => return ord,
        }
        self.rpm_patch.partial_cmp(&other.rpm_patch)
    }
}

#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Debug)]
struct Version(Vec<u64>);

#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Debug)]
struct Tag {
    env: EnvTag,
    n: Option<u64>,
}

#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Debug)]
struct Commits {
    ncommits: u64,
    commit: String,
}

#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Copy, Debug)]
enum EnvTag {
    Dev,
    Tst,
    Acc,
}

// #[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Copy, Debug)]
// enum Environment {
//     Dev,
//     Tst,
//     Acc,
//     Prd,
// }

fn parse_rpm_file_name(input: &str) -> IResult<&str, RpmFileName> {
    let (input, (name, (version, os, arch))) =
        take_non_greedy(input, parse_rpm_file_name_next)?;
    Ok((
        input,
        RpmFileName {
            name: name.to_string(),
            version,
            os,
            arch,
        },
    ))
}

fn take_non_greedy<'a, F, O>(
    input: &'a str,
    mut next: F,
) -> IResult<&'a str, (&'a str, O)>
where
    F: FnMut(&'a str) -> IResult<&'a str, O>,
{
    for i in 0..input.len() {
        if let Ok((next_input, value)) = next(&input[i..]) {
            return Ok((next_input, (&input[0..i], value)));
        }
    }
    Err(nom::Err::Error(nom::error::Error::new(
        input,
        nom::error::ErrorKind::TakeTill1,
    )))
}

fn parse_rpm_file_name_next(
    input: &str,
) -> IResult<&str, (RpmVersion, String, String)> {
    let (input, _) = char('-')(input)?;
    let (input, version) = parse_rpm_version(input)?;
    let (input, os) = preceded(
        char('.'),
        map(take_while1(|c| c != '.'), |s: &str| s.to_string()),
    )(input)?;
    let (input, arch) = preceded(
        char('.'),
        map(take_while1(|c| c != '.'), |s: &str| s.to_string()),
    )(input)?;
    let (input, _) = terminated(tag(".rpm"), eof)(input)?;
    Ok((input, (version, os, arch)))
}

fn parse_rpm_version(input: &str) -> IResult<&str, RpmVersion> {
    let (input, version) = parse_version(input)?;
    let (input, env) = opt(preceded(char('.'), parse_tag))(input)?;
    let (input, commits) = opt(preceded(char('_'), parse_git_version))(input)?;
    let (input, dirty) = map(opt(tag("_dirty")), |v| v.is_some())(input)?;
    let (input, rpm_patch) =
        preceded(char('-'), map(digit1, |s: &str| s.parse().unwrap()))(input)?;
    Ok((
        input,
        RpmVersion {
            version,
            tag: env,
            commits,
            dirty,
            rpm_patch,
        },
    ))
}

fn parse_version(input: &str) -> IResult<&str, Version> {
    let (input, vs) = separated_list1(char('.'), parse_u64)(input)?;
    Ok((input, Version(vs)))
}

fn parse_tag(input: &str) -> IResult<&str, Tag> {
    let (input, env) = parse_env_tag(input)?;
    let (input, n) = opt(parse_u64)(input)?;
    Ok((input, Tag { env, n }))
}

fn parse_env_tag(input: &str) -> IResult<&str, EnvTag> {
    alt((
        value(EnvTag::Dev, tag("dev")),
        value(EnvTag::Tst, tag("tst")),
        value(EnvTag::Acc, tag("acc")),
    ))(input)
}

fn parse_git_version(input: &str) -> IResult<&str, Commits> {
    let (input, ncommits) = parse_u64(input)?;
    let (input, commit) =
        preceded(tag("_g"), map(hex_digit1, |s: &str| s.to_string()))(input)?;
    Ok((input, Commits { ncommits, commit }))
}

fn parse_u64(input: &str) -> IResult<&str, u64> {
    map(digit1, |s: &str| s.parse().unwrap())(input)
}

#[cfg(test)]
mod test {
    use crate::parse_rpm_file_name;

    #[test]
    fn version_sort() {
        let mut packages = vec![
            "smart-agent-0.99.2.dev_156_g37d5cf7-1.el7.x86_64.rpm",
            "smart-agent-0.99.2.dev_96_g8842743-1.el7.x86_64.rpm",
            "smart-agent-1.06.acc2_467_gfaa1616-1.el7.x86_64.rpm",
            "smart-agent-1.06.acc2_467_gfaa1616_dirty-1.el7.x86_64.rpm",
            "smart-agent-1.06.acc3-1.el7.x86_64.rpm",
            "smart-agent-1.06.acc3_32_gf91689f-1.el7.x86_64.rpm",
            "smart-agent-2.00_4_ge6c2339_dirty-1.el7.x86_64.rpm",
            "smart-agent-2.00.acc1_dirty-1.el7.x86_64.rpm",
            "smart-agent-2.00.acc4_dirty-1.el7.x86_64.rpm",
            "smart-agent-2.03.3.1.tst-1.el7.x86_64.rpm",
        ];
        packages.sort_by_cached_key(|s| {
            parse_rpm_file_name(s)
                .expect("failed to parse rpm file name")
                .1
        });
        assert_eq!(
            packages,
            vec![
                "smart-agent-0.99.2.dev_96_g8842743-1.el7.x86_64.rpm",
                "smart-agent-0.99.2.dev_156_g37d5cf7-1.el7.x86_64.rpm",
                "smart-agent-1.06.acc2_467_gfaa1616-1.el7.x86_64.rpm",
                "smart-agent-1.06.acc2_467_gfaa1616_dirty-1.el7.x86_64.rpm",
                "smart-agent-1.06.acc3-1.el7.x86_64.rpm",
                "smart-agent-1.06.acc3_32_gf91689f-1.el7.x86_64.rpm",
                "smart-agent-2.00.acc1_dirty-1.el7.x86_64.rpm",
                "smart-agent-2.00.acc4_dirty-1.el7.x86_64.rpm",
                "smart-agent-2.00_4_ge6c2339_dirty-1.el7.x86_64.rpm",
                "smart-agent-2.03.3.1.tst-1.el7.x86_64.rpm",
            ]
        );
    }
}
