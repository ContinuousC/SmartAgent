/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::net::IpAddr;
use std::net::Ipv4Addr;
use std::os::fd::AsRawFd;
use std::path::Path;
use std::path::PathBuf;
use std::process::Output;
use std::time::Duration;

use agent_utils::ip_lookup_one;
use log::debug;
use log::error;
use log::info;
use log::trace;
use tokio::io::AsyncWriteExt;
use tokio::net::UnixStream;

use agent_utils::KeyVault;
use powershell_protocol as ps;
use ps::NtlmCredentials;
use serde::Deserialize;
use serde::Serialize;
use tokio::process::Command;

use crate::error::DTResult;
use crate::error::WMIDTError;
use crate::Result;
use crate::WMIError;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Config {
    hostname: String,
    ipaddress: Option<Ipv4Addr>,
    credentials: Credentials,
    #[serde(default = "default_timeout")]
    timeout: u8,
    #[serde(default)]
    use_sudo: bool,
}

fn default_timeout() -> u8 {
    10
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DcomSession {
    address: IpAddr,
    username: String,
    password: String,
    domain: Option<String>,
    timeout: u8,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Credentials {
    PasswdFile(PathBuf),
    Ntlm(NtlmCredentials),
}

impl Config {
    pub async fn new_session(
        &self,
        key_vault: &KeyVault,
    ) -> Result<DcomSession> {
        let creds = self
            .credentials
            .get_credentials(key_vault, self.use_sudo)
            .await?;
        let ip = match self.ipaddress {
            Some(ip) => IpAddr::V4(ip),
            None => ip_lookup_one(&self.hostname).await?,
        };

        Ok(DcomSession {
            username: creds.username,
            password: creds.password,
            domain: creds.domain,

            address: ip,
            timeout: self.timeout,
        })
    }
}

impl Credentials {
    async fn get_credentials(
        &self,
        key_vault: &KeyVault,
        use_sudo: bool,
    ) -> Result<NtlmCredentials> {
        let creds = match self {
            Self::Ntlm(creds) => creds.clone(),
            Self::PasswdFile(path) => {
                Self::from_pwdfile(path, use_sudo).await?
            }
        };

        let (username, password) = ps::Credentials::from_keyvault(
            key_vault,
            creds.username,
            &creds.password,
        )
        .await?;

        Ok(NtlmCredentials {
            domain: creds.domain,
            username,
            password,
        })
    }

    async fn from_pwdfile(
        path: &Path,
        use_sudo: bool,
    ) -> Result<NtlmCredentials> {
        info!(
            "logging in using credential file {}: {}",
            if use_sudo { "(using sudo)" } else { "" },
            path.display()
        );
        let content = if use_sudo {
            Self::sudo_read_pwdfile(path).await?
        } else {
            tokio::fs::read_to_string(&path)
                .await
                .map_err(WMIError::IO)?
        };

        let mut params: BTreeMap<String, String> = BTreeMap::new();
        for line in content.lines() {
            if let Some(idx) = line.find('=') {
                params.insert(
                    line[0..idx].to_lowercase(),
                    line[idx + 1..].to_string(),
                );
            }
        }

        Ok(NtlmCredentials {
            username: params.remove("username").ok_or_else(|| {
                WMIError::MissingInAuthfile(String::from("username"))
            })?,
            password: params.remove("password").ok_or_else(|| {
                WMIError::MissingInAuthfile(String::from("password"))
            })?,
            domain: params.remove("domain"),
        })
    }

    async fn sudo_read_pwdfile(path: &Path) -> Result<String> {
        let output = tokio::process::Command::new("sudo")
            .arg("/usr/bin/cat")
            .arg(path)
            .output()
            .await
            .map_err(WMIError::IO)?;

        if !output.status.success() {
            Err(WMIError::Custom(format!(
                "could not read passwordfile with sudo: {}",
                String::from_utf8_lossy(&output.stderr)
            )))
        } else {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        }
    }
}

static DECOM_DELIMITER: &str = "|||";

impl DcomSession {
    fn user(&self) -> String {
        let mut user = self.username.clone();
        if let Some(d) = &self.domain {
            user = format!("{d}/{user}");
        }
        user
    }

    pub async fn get_wmiobject(
        &mut self,
        class: &str,
        namespace: &str,
        attributes: &[String],
    ) -> DTResult<Vec<HashMap<String, String>>> {
        debug!("requesting class {class} with attributes: {attributes:?}");

        let output = self.execute_wmic(class, namespace, attributes).await?;
        if !output.status.success() {
            let stderr = String::from_utf8(output.stderr)
                .map_err(WMIDTError::ParseUTF8)?;
            error!("wmic query failed: {stderr}");
            return Err(WMIDTError::QueryWmic(stderr));
        }

        Ok(Self::parse_lines(
            String::from_utf8(output.stdout).map_err(WMIDTError::ParseUTF8)?,
        ))
    }

    async fn execute_wmic(
        &self,
        class: &str,
        namespace: &str,
        attributes: &[String],
    ) -> DTResult<Output> {
        let (rx, mut tx) =
            UnixStream::pair().map_err(WMIDTError::SocketCreation)?;
        tx.write(format!("{}\n", self.password).as_bytes())
            .await
            .map_err(WMIDTError::WritePassword)?;
        trace!("logging in with {}: {}", &self.user(), &self.password);

        // rust sets the FD_CLOEXEC flag by default.
        // so we have to manually remove the flag wit libc::fcntl
        let rx_fd = rx.as_raw_fd();
        unsafe {
            let flags = libc::fcntl(rx_fd, libc::F_GETFD);
            libc::fcntl(rx_fd, libc::F_SETFD, flags & !libc::FD_CLOEXEC);
        }

        let mut command = Command::new("wmic");
        command
            .arg("-U")
            .arg(self.user())
            .arg("--namespace")
            .arg(namespace)
            .arg("--delimiter")
            .arg(DECOM_DELIMITER)
            .arg(format!("//{}[sign]", self.address))
            .arg(format!("select {} from {class}", attributes.join(",")))
            .env("PASSWD_FD", rx_fd.to_string());

        let std_cmd = command.as_std();
        debug!(
            "executing: {:?} {:?}",
            std_cmd.get_program(),
            std_cmd.get_args().collect::<Vec<_>>().join(" ".as_ref())
        );

        tokio::time::timeout(
            Duration::from_secs(self.timeout as u64),
            command.output(),
        )
        .await
        .map_err(|_| WMIDTError::WmicTimeout)?
        .map_err(WMIDTError::ExecuteWmic)
    }

    fn parse_lines(stdout: String) -> Vec<HashMap<String, String>> {
        trace!("result from command:\n{stdout}");

        let mut lines = stdout.lines().skip(1);
        let headerline = lines.next();
        if headerline.is_none() {
            return Vec::new();
        }
        let headers: Vec<_> =
            headerline.unwrap().split(DECOM_DELIMITER).collect();
        trace!("headers: {headers:?}");

        let fields = lines.collect::<Vec<_>>().join("\n");
        let mut fields: Vec<&str> = fields.split(DECOM_DELIMITER).collect();

        let mut num_fields = fields.len();
        let mut idx = 0;
        while idx != num_fields {
            let field = fields[idx];
            if (idx + 1) % headers.len() != 0 {
                idx += 1;
                continue;
            }

            if field.ends_with('\n') {
                idx += 1;
                continue;
            }

            if let Some(new_line) = field.rfind('\n') {
                let left = &field[..new_line];
                let right = &field[new_line + 1..];
                fields.remove(idx);
                fields.insert(idx, right);
                fields.insert(idx, left);
                num_fields += 1;
                idx += 2;
            }

            idx += 1;
        }

        let rows = fields
            .chunks_exact(headers.len())
            .map(|fields| {
                headers
                    .iter()
                    .zip(fields.iter())
                    .map(|(h, f)| (h.to_string(), f.to_string()))
                    .collect()
            })
            .collect();

        trace!("parsed rows: {rows:#?}");
        rows
    }
}
