/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::os::unix::io::RawFd;
use std::path::PathBuf;
use std::process;

use winrm_rs::{
    authentication::Authentication,
    authentication::{BasicAuth, KerberosAuth, NtlmAuth},
};

use clap::Parser;

use crate::scripts::CmdScript;
use crate::scripts::PsScript;
use crate::{
    cmk,
    credential::{AuthMethod, Credential},
    Error, Result,
};

/// Test winrm credentials and permissions by executing commands and retrieving datatables from an mp
/// The script tests hosts/tags in the following way:
///     - if you use get-wmiobject or get-ciminstance: request a shell
///     - request the wmiobject provided in the arguments, or Win32_Computersystem by default
///     - request all wmiobjects in an mp (to be added)
#[derive(Parser, Debug)]
#[clap(author, version, about)]
pub struct Args {
    /// Name of the host to be polled.
    #[clap(short = 'H', long)]
    pub hostname: Vec<String>,
    /// the ip address of the host. can be used in case the hostname cannot be resolved
    #[clap(short = 'I', long)]
    pub ipaddr: Option<std::net::Ipv4Addr>,
    /// Port of the winrm services.
    /// if this is not set, the default value of its protocol is used.
    #[clap(short = 'P', long)]
    pub port: Option<u16>,
    /// Name of the user that logs in. Do not add a domain.
    /// If this is a domain user, use the domain flag.
    /// This is not required when you are using kerberos.
    #[clap(
        short = 'u',
        long,
        required_if("auth-method", "Ntlm"),
        required_if("auth-method", "Basic")
    )]
    pub username: Option<String>,
    /// Password of the user. Use the use_keyvault flag if the password comes from a passwordvault.
    #[clap(short = 'p', long)]
    pub password: Option<String>,
    /// Authentication Mechanism used to log in.
    /// NOTE: kerberos requires a kinit to be run in advance!
    #[clap(short = 'm', long, default_value_t)]
    pub auth_method: AuthMethod,
    /// The domain the user belongs to. This is required for Ntlm and Kerberos
    #[clap(
        short = 'd',
        long,
        required_if("auth-method", "Ntlm"),
        required_if("auth-method", "Kerberos")
    )]
    pub domain: Option<String>,
    #[clap(long, parse(from_flag))]
    pub disable_ssl: bool,
    /// Location of the CA certificate, used to verify the certificate of the server.
    #[clap(short = 'c', long)]
    pub cacert: Option<PathBuf>,
    /// You should think very carefully before using this method.
    /// If invalid certificates are trusted, any certificate for any site will be trusted for use.
    /// This includes expired certificates. This introduces significant vulnerabilities, and should only be used as a last resort.
    #[clap(long, parse(from_flag))]
    pub danger_disable_certificate_verification: bool,
    /// You should think very carefully before you use this method.
    /// If hostname verification is not used, any valid certificate for any site will be trusted for use from any other.
    /// This introduces a significant vulnerability to man-in-the-middle attacks.
    #[clap(long, parse(from_flag))]
    pub danger_disable_hostname_verification: bool,
    /// Take credentials from the keyvault. The username will be used as entry in the keyvault.
    #[clap(short = 'K', long, parse(from_flag))]
    pub use_keyvault: bool,
    /// KeyReader socket fd to use to obtain credentials.
    #[clap(short = 'A', long, hidden(true))]
    pub auth_sock: Option<RawFd>,
    /// Wmi method used to request wmiobjects.
    /// GetWmiObject & GetCimInstance create a powershell-shell and execute their respective methods
    /// EnumerateCimInstance uses the built-in method in winrm to retrieve the cim instances
    /// EnumerateCimInstance is faster and more efficient, but still in beta.
    /// Dcom: Use the old dcom method of retrieving events. Use NTLM when using this method. (Not yet implemented)
    #[clap(long, default_value_t)]
    pub wmi_method: WmiMethod,
    /// The location of the CCache used for kerberos authentication.
    #[clap(short = 'C', long)]
    pub ccache: Option<String>,
    /// increase verbosity. Every additional v will increase the verbosity by one stage.
    /// verbose messages are send to stderr. turned of by default.
    #[clap(short = 'v', long, action = clap::ArgAction::Count)]
    pub verbosity: u8,
    /// timeout used on every request in seconds. by default, this is 10 seconds
    #[clap(short = 't', long, default_value = "10")]
    pub timeout: u64,
    /// The wmiobject that will be requested as test
    #[clap(long, default_value = "Win32_Computersystem")]
    pub wmi_object: Vec<String>,
    /// log the object eing retrieved from the hosts. this will be logged as debug
    #[clap(long, parse(from_flag))]
    pub log_object: bool,
    #[clap(long)]
    /// a powershell script, or path to a script that will be executed on the server, instead a wmi query
    pub ps_script: Option<PsScript>,
    /// a cmd script, or path to a script that will be executed on the server, instead a wmi query
    #[clap(long)]
    pub cmd_script: Option<CmdScript>,
    /// add the domain to the provided hostname. This can be used for when you test by tag in a site with hostnames instead of fqdns (for ssl verification)
    #[clap(long, parse(from_flag))]
    pub append_domain: bool,
    /// This will print the stdout of the the tested winrm command to stdout
    #[clap(long, parse(from_flag))]
    pub print_stdout: bool,
}

impl Args {
    pub fn init_logger(&self) {
        if let Err(e) = simplelog::TermLogger::init(
            match self.verbosity {
                0 => simplelog::LevelFilter::Info,
                1 => simplelog::LevelFilter::Debug,
                2.. => simplelog::LevelFilter::Trace,
            },
            simplelog::ConfigBuilder::new()
                .add_filter_ignore_str("serde_xml_rs")
                .add_filter_ignore_str("handlebars")
                .add_filter_ignore_str("want")
                .add_filter_ignore_str("mio")
                // .add_filter_ignore_str("winrm_rs")
                .build(),
            simplelog::TerminalMode::Stdout,
            simplelog::ColorChoice::Auto,
        ) {
            eprintln!("Error: failed to initialize logging: {}", e);
            process::exit(1);
        }
    }

    pub async fn get_winrm_credentials(
        &self,
        hostname: &String,
    ) -> Result<Authentication> {
        let credentials = Credential::new(self).await?;
        Ok(match self.auth_method {
            AuthMethod::Basic => Authentication::Basic(BasicAuth::new(
                credentials.username()?,
                credentials.password()?,
            )),
            AuthMethod::Ntlm => Authentication::Ntlm(NtlmAuth::new(
                credentials.username()?,
                credentials.domain()?,
                credentials.password()?,
            )),
            AuthMethod::Kerberos => {
                Authentication::Kerberos(KerberosAuth::new(
                    hostname.split('.').next().unwrap_or(hostname),
                    credentials.domain()?,
                    self.ccache.as_ref().cloned(),
                ))
            }
        })
    }

    pub async fn get_hosts(&self) -> Result<HashSet<String>> {
        let mut hostnames = HashSet::new();
        let mut by_tag = HashMap::new();

        for name in self.hostname.iter() {
            if let Some(tag) = name.strip_prefix('@') {
                by_tag.insert(
                    tag.to_string(),
                    cmk::get_hosts_from_tag(tag).await?,
                );
            } else {
                hostnames.insert(name.to_string());
            }
        }

        Ok(hostnames
            .union(
                &by_tag
                    .into_values()
                    .reduce(|acum, elem| {
                        acum.intersection(&elem).cloned().collect()
                    })
                    .unwrap_or(HashSet::new()),
            )
            .filter(|h| !h.is_empty())
            .cloned()
            .collect())
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum WmiMethod {
    GetWmiObject,
    GetCimInstance,
    EnumerateCimInstance,
}

impl Default for WmiMethod {
    fn default() -> Self {
        Self::GetWmiObject
    }
}

impl fmt::Display for WmiMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::GetWmiObject => "GetWmiObject",
                Self::GetCimInstance => "GetCimInstance",
                Self::EnumerateCimInstance => "EnumerateCimInstance",
            }
        )
    }
}

impl std::str::FromStr for WmiMethod {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(match s.to_lowercase().as_str() {
            "getwmiobject" => Self::GetWmiObject,
            "getciminstance" => Self::GetCimInstance,
            "enumerateciminstance" => Self::EnumerateCimInstance,
            _ => Err(Error::InvalidArg(s.to_string()))?,
        })
    }
}
