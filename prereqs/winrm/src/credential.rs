/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::ffi::OsString;
use std::fmt;
use std::os::unix::process::CommandExt;
use std::process::Command;

use agent_utils::KeyVault;

use crate::args::Args;
use crate::error::{Error, Result};

#[derive(Debug)]
pub enum AuthMethod {
    Basic,
    Ntlm,
    Kerberos,
}

impl Default for AuthMethod {
    fn default() -> Self {
        Self::Ntlm
    }
}

impl fmt::Display for AuthMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Basic => "Basic",
                Self::Ntlm => "Ntlm",
                Self::Kerberos => "Kerberos",
            }
        )
    }
}

impl std::str::FromStr for AuthMethod {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(match s.to_lowercase().as_str() {
            "basic" => Self::Basic,
            "ntlm" => Self::Ntlm,
            "kerberos" => Self::Kerberos,
            _ => Err(Error::InvalidArg(s.to_string()))?,
        })
    }
}

#[derive(Debug, Default)]
pub struct Credential {
    pub username: Option<String>,
    pub password: Option<String>,
    pub domain: Option<String>,
}

impl Credential {
    pub async fn new(args: &Args) -> Result<Self> {
        if args.use_keyvault {
            Self::from_keyvault(args).await
        } else {
            Ok(Credential {
                username: args.username.as_ref().cloned(),
                password: args.password.as_ref().cloned(),
                domain: args.domain.as_ref().cloned(),
            })
        }
    }

    async fn from_keyvault(args: &Args) -> Result<Self> {
        let kvault = match args.auth_sock {
            Some(fd) => KeyVault::new_key_reader(fd)?,
            None => {
                let args: Vec<OsString> = std::env::args_os().skip(1).collect();
                return Err(Error::KeyReader(
                    Command::new("/usr/bin/key-reader")
                        .args(
                            [
                                OsString::from("connect"),
                                OsString::from("--"),
                                std::env::current_exe()?.as_os_str().to_owned(),
                                OsString::from("-C"),
                                OsString::from("SOCK"),
                            ]
                            .iter()
                            .chain(&args),
                        )
                        .exec(),
                ));
            }
        };
        let kr_entry = kvault
            .retrieve_creds(
                args.username
                    .as_ref()
                    .cloned()
                    .ok_or(Error::NoKVaultEntryGiven)?,
            )
            .await?
            .ok_or(Error::MissingKREntry)?;
        let mut kuser = kr_entry
            .username
            .as_ref()
            .ok_or(Error::MissingKRObject(String::from("username")))?
            .split('@');

        Ok(Credential {
            username: kuser.next().map(|s| s.to_string()),
            password: kr_entry.password,
            domain: kuser.next().map(|s| s.to_string()),
        })
    }

    pub fn username(&self) -> Result<String> {
        return self
            .username
            .as_ref()
            .cloned()
            .ok_or_else(|| Error::RequiredArg(String::from("username")));
    }
    pub fn password(&self) -> Result<String> {
        return self
            .password
            .as_ref()
            .cloned()
            .ok_or_else(|| Error::RequiredArg(String::from("password")));
    }
    pub fn domain(&self) -> Result<String> {
        return self
            .domain
            .as_ref()
            .cloned()
            .ok_or_else(|| Error::RequiredArg(String::from("domain")));
    }
}
