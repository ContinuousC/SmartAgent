/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::{
    collections::HashSet, env, os::unix::prelude::MetadataExt, path::PathBuf,
};

use tokio::{fs, process::Command};

use thiserror::Error;

type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Currently not in an omd site")]
    NotOMD,
    #[error("Unable to spawn command: cmk {0}: {1}")]
    SpawnCmkCommand(String, tokio::io::Error),
    #[error("IO: {0}")]
    IO(#[from] std::io::Error),
}

pub fn omd_root() -> Result<PathBuf> {
    Ok(PathBuf::from(
        env::var("OMD_ROOT").map_err(|_e| Error::NotOMD)?,
    ))
}

pub fn get_spec_path(spec: &String) -> Result<PathBuf> {
    Ok(omd_root()?.join("local/share/mnow/agent/mps").join(spec))
}

pub async fn verifiy_spec_path(path: &PathBuf) -> Result<bool> {
    let st = fs::metadata(&path).await?;
    Ok(st.uid() == 0 && st.gid() == 0 && st.mode() & 0o777113 == 0o100000)
}

pub async fn get_hosts_from_tag(tag: &str) -> Result<HashSet<String>> {
    omd_root()?; // check if site user

    let child = Command::new("cmk")
        .arg("--list-tag")
        .arg(tag)
        .output()
        .await
        .map_err(|e| {
            Error::SpawnCmkCommand(format!("--list-tag {}", &tag), e)
        })?;

    Ok(String::from_utf8_lossy(&child.stdout)
        .split('\n')
        .map(|s| s.to_string())
        .collect())
}
