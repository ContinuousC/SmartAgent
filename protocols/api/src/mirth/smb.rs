/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::{collections::HashMap, os::fd::AsRawFd};

use agent_utils::KeyVault;
use log::{debug, trace, warn};
use nom::IResult;
use protocol::auth;
use tap::TapFallible;
use tokio::{io::AsyncWriteExt, net::UnixStream, sync::Semaphore};

use crate::mirth::{SmbError as Error, SmbResult as Result};

pub struct Client<'a> {
    username: String,
    password: String,
    server_mapping: &'a HashMap<String, String>,
    semaphore: Semaphore,
}

impl<'a> Client<'a> {
    pub async fn new(
        smb_auth: &'a auth::NtlmAuth,
        smb_opts: &'a super::SmbOpts,
        key_vault: &KeyVault,
    ) -> Result<Self> {
        let password = smb_auth.password.as_deref().unwrap_or_default();
        let (mut username, password) = crate::config::from_keyvault(
            key_vault,
            smb_auth.username.clone(),
            password,
        )
        .await?;

        if password.is_empty() {
            return Err(Error::NoPassword(username));
        }

        if let Some(domain) = &smb_auth.domain {
            username = format!("{domain}/{username}");
        }

        Ok(Self {
            server_mapping: &smb_opts.server_mapping,
            semaphore: Semaphore::new(smb_opts.max_concurrent),
            username,
            password,
        })
    }

    pub async fn listdir(&self, host: &str) -> Result<Vec<String>> {
        let permit = self.semaphore.acquire().await.unwrap();

        let (rx, mut tx) = UnixStream::pair().map_err(Error::SocketCreation)?;
        tx.write(format!("{}\n", self.password).as_bytes())
            .await
            .map_err(Error::WritePassword)?;

        // rust sets the FD_CLOEXEC flag by default.
        // so we have to manually remove the flag wit libc::fcntl
        let rx_fd = rx.as_raw_fd();
        unsafe {
            let flags = libc::fcntl(rx_fd, libc::F_GETFD);
            libc::fcntl(rx_fd, libc::F_SETFD, flags & !libc::FD_CLOEXEC);
        }

        let (server, share, directory) = Self::parse_smbpath(host)?;
        let server = self
            .server_mapping
            .get(server)
            .map(|s| s.as_str())
            .unwrap_or(server);
        trace!("server for host {host} mapped to {server}");

        let smb_command = match directory.is_empty() {
            true => "ls".to_string(),
            false => format!("ls {}/*", directory),
        };

        debug!("executing smb command: smbclient '//{server}/{share}' -U {} -c '{smb_command}'", &self.username);
        let output = tokio::process::Command::new("smbclient")
            .arg(format!("//{server}/{share}"))
            .arg("-U")
            .arg(&self.username)
            // .arg(format!("{}%{}", self.username, self.password))
            .arg("-c")
            .arg(smb_command)
            .env("PASSWD_FD", rx_fd.to_string())
            .output()
            .await
            .map_err(Error::ExecSmbClient)?;

        if let Err(e) = tx.shutdown().await {
            warn!("Could not shut down password socket: {e}");
        }
        drop(tx);
        drop(rx);
        drop(permit);

        let stdout = String::from_utf8(output.stdout)?;
        let stderr = String::from_utf8(output.stderr)?;

        output
            .status
            .success()
            .then(|| Self::parse_outout(&stdout))
            .ok_or_else(|| {
                Error::SmbClientFailed(
                    output.status,
                    format!("{stdout}\n{stderr}"),
                )
            })
            .tap_err(|e| warn!("smb command failed: {e}"))
    }

    fn parse_outout(stdout: &str) -> Vec<String> {
        stdout
            .lines()
            .filter_map(|l| {
                let split: Vec<_> = l.split_ascii_whitespace().collect();
                split
                    .iter()
                    .enumerate()
                    .find(|(_idx, s)| **s == "A")
                    .map(|(idx, _)| split[0..idx].join(" "))
            })
            .collect()
    }

    pub fn parse_smbpath(path: &str) -> Result<(&str, &str, &str)> {
        const ILLEGAL_FILENAME_CHARACTERS: &str = r#"<>:"?\|*"#;
        const ALLOWED_DNSNAME_CHARACTERS: &str =
            r#"abcdefghijklmnopqrstuvwxyz0123456789-."#;

        let illegal_char = || Error::IllegalCharacters(path.to_string());

        fn nom_parser(path: &str) -> IResult<&str, (&str, &str, &str)> {
            use nom::{
                branch::alt,
                bytes::complete::{tag, take_until, take_while},
                combinator::{eof, rest},
                sequence::{pair, preceded, separated_pair},
            };

            let (_, (server, (share, dir))) = preceded(
                tag("//"),
                separated_pair(
                    take_until("/"),
                    tag("/"),
                    pair(
                        take_while(|c| c != '/'),
                        preceded(alt((tag("/"), eof)), rest),
                    ),
                ),
            )(path)?;

            Ok(("", (server, share, dir)))
        }

        let (_, (server, share, dir)) = nom_parser(path).map_err(|e| {
            Error::InvalidSmbPath(path.to_string(), e.to_string())
        })?;

        server
            .chars()
            .all(|c: char| {
                ALLOWED_DNSNAME_CHARACTERS
                    .contains(c.to_lowercase().next().unwrap())
            })
            .then_some(())
            .ok_or_else(illegal_char)?;

        share
            .chars()
            .all(|c| !ILLEGAL_FILENAME_CHARACTERS.contains(c))
            .then_some(())
            .ok_or_else(illegal_char)?;

        dir.chars()
            .all(|c| !ILLEGAL_FILENAME_CHARACTERS.contains(c))
            .then_some(())
            .ok_or_else(illegal_char)?;

        Ok((server, share, dir))
    }
}

#[cfg(test)]
mod tests {
    use crate::mirth::smb::Client;

    #[test]
    fn parse_vanas_inventory() {
        const TESTCASE: &str = "//SRVMIRTHFS001/from_vanas/inventaris";
        let (server, share, dir) = Client::parse_smbpath(TESTCASE).unwrap();
        assert_eq!(server, "SRVMIRTHFS001");
        assert_eq!(share, "from_vanas");
        assert_eq!(dir, "inventaris");
    }

    #[test]
    fn parse_vanas_inventory_multidir() {
        const TESTCASE: &str =
            "//SRVMIRTHFS001/from_vanas/inventaris/inventaris";
        let (server, share, dir) = Client::parse_smbpath(TESTCASE).unwrap();
        assert_eq!(server, "SRVMIRTHFS001");
        assert_eq!(share, "from_vanas");
        assert_eq!(dir, "inventaris/inventaris");
    }

    #[test]
    fn parse_vanas_inventory_nodir1() {
        const TESTCASE: &str = "//SRVMIRTHFS001/from_vanas/";
        let (server, share, dir) = Client::parse_smbpath(TESTCASE).unwrap();
        assert_eq!(server, "SRVMIRTHFS001");
        assert_eq!(share, "from_vanas");
        assert_eq!(dir, "");
    }

    #[test]
    fn parse_vanas_inventory_nodir2() {
        const TESTCASE: &str = "//SRVMIRTHFS001/from_vanas";
        let (server, share, dir) = Client::parse_smbpath(TESTCASE).unwrap();
        assert_eq!(server, "SRVMIRTHFS001");
        assert_eq!(share, "from_vanas");
        assert_eq!(dir, "");
    }

    #[test]
    fn parse_vanas_administration() {
        const TESTCASE: &str = "//{SRVMIRTHFS001}/from_vanas/toedieningen";
        assert!(Client::parse_smbpath(TESTCASE).is_err())
    }
}
