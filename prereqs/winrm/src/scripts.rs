/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::fmt;
use std::path::PathBuf;

use log::trace;
use tokio::sync::Mutex;

use winrm_rs::Session;

use crate::{Error, Result};

lazy_static::lazy_static! {
    static ref PSSCRIPT: Mutex<Option<String>> = Mutex::new(None);
}

#[derive(Debug)]
pub enum Script {
    Location(PathBuf),
    Text(String),
}

#[derive(Debug)]
pub struct PsScript(Script);
#[derive(Debug)]
pub struct CmdScript(Script);

impl Script {
    pub async fn get_script(&self) -> Result<*const String> {
        match &self {
            Self::Text(s) => Ok(s),
            Self::Location(p) => Self::script_from(p).await,
        }
    }
    async fn script_from(p: &PathBuf) -> Result<*const String> {
        let mut lock = PSSCRIPT.lock().await;
        Ok(if let Some(s) = lock.as_ref() {
            s
        } else {
            let content = &tokio::fs::read(p).await?;
            let content = String::from_utf8_lossy(content);
            lock.get_or_insert(content.to_string())
        })
    }
}

impl fmt::Display for Script {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Script::Location(p) => p.to_string_lossy(),
                Script::Text(s) => std::borrow::Cow::Borrowed(s.as_str()),
            }
        )
    }
}

impl std::str::FromStr for Script {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let path = PathBuf::from(s);
        Ok(if PathBuf::from(s).exists() {
            Self::Location(path)
        } else {
            Self::Text(s.to_string())
        })
    }
}

impl PsScript {
    pub async fn test(&self, session: &mut Session) -> (String, Result<()>) {
        let shell = match session.shell().await {
            Ok(s) => s,
            Err(e) => {
                return (String::from("Shell creation"), Err(Error::WinRM(e)))
            }
        };

        let script = match self.0.get_script().await {
            Ok(s) => unsafe { &*s },
            Err(e) => return (String::from("Reading script"), Err(e)),
        };

        let out = session.run_ps(&shell, script).await;
        trace!("output from host: {:?}", &out);
        if let Err(e) = session.close_shell(shell).await {
            return (String::from("Closing shell"), Err(Error::WinRM(e)));
        }

        match out {
            Ok(out) => {
                if out.exitcode == 0 {
                    (out.stdout.join("\n"), Ok(()))
                } else {
                    (out.stderr, Err(Error::CommandFailed(out.exitcode)))
                }
            }
            Err(e) => (String::from("Execute script"), Err(Error::WinRM(e))),
        }
    }
}

impl CmdScript {
    pub async fn test(&self, session: &mut Session) -> (String, Result<()>) {
        let shell = match session.shell().await {
            Ok(s) => s,
            Err(e) => {
                return (String::from("Shell creation"), Err(Error::WinRM(e)))
            }
        };

        let script = match self.0.get_script().await {
            Ok(s) => unsafe { &*s },
            Err(e) => return (String::from("Reading script"), Err(e)),
        };

        let out = session.run_cmd(&shell, script, Vec::new()).await;
        trace!("output from host: {:?}", &out);
        if let Err(e) = session.close_shell(shell).await {
            return (String::from("Closing shell"), Err(Error::WinRM(e)));
        }

        match out {
            Ok(out) => {
                if out.exitcode == 0 {
                    (out.stdout.join("\n"), Ok(()))
                } else {
                    (
                        out.stderr.join("\n"),
                        Err(Error::CommandFailed(out.exitcode)),
                    )
                }
            }
            Err(e) => (String::from("Execute script"), Err(Error::WinRM(e))),
        }
    }
}

impl fmt::Display for PsScript {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl std::str::FromStr for PsScript {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Script::from_str(s).map(Self)
    }
}

impl fmt::Display for CmdScript {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl std::str::FromStr for CmdScript {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Script::from_str(s).map(Self)
    }
}
