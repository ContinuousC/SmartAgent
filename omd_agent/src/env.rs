/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::env;
use std::path::PathBuf;

use fs4::tokio::AsyncFileExt;
use log::debug;

use crate::{
    config::HostConfig,
    context::{Mode, Options},
    error::{Error, Result},
};
use etc_base::{PackageName, PackageVersion};
use tokio::fs::{self, File};

const AGENT_PATH: &str = "local/share/mnow/agent/mps";
const PARSERS_PATH: &str = "local/share/mnow/agent/parsers";
const CONFIG_PATH: &str = "var/mnow/config";
const DATA_PATH: &str = "var/mnow/data";
const CACHE_PATH: &str = "var/mnow/state";
const PACKAGE_META_PATH: &str = "var/mnow/packages";
pub(super) const ERRORS_PATH: &str = "tmp/mnow/cache";

pub fn omd_root() -> Result<PathBuf> {
    Ok(PathBuf::from(
        env::var("OMD_ROOT").map_err(|_e| Error::NotOMD)?,
    ))
}

pub fn get_site_name() -> Result<String> {
    env::var("OMD_SITE").map_err(|_e| Error::NotOMD)
}

pub fn get_cache_path() -> Result<PathBuf> {
    Ok(omd_root()?.join(CACHE_PATH))
}

pub fn get_parsers_path() -> Result<PathBuf> {
    Ok(omd_root()?.join(PARSERS_PATH))
}

pub fn get_data_path() -> Result<PathBuf> {
    Ok(omd_root()?.join(DATA_PATH))
}

pub fn get_specs_path() -> Result<PathBuf> {
    Ok(omd_root()?.join(AGENT_PATH))
}

pub fn get_mp_specs() -> Result<Vec<PathBuf>> {
    let agent_pattern =
        format!("{}/*.json", omd_root()?.join(AGENT_PATH).display());
    Ok(glob::glob(&agent_pattern)?
        .collect::<std::result::Result<Vec<_>, _>>()?)
}

pub fn get_pckg_meta(pckg_name: &PackageName) -> Result<PathBuf> {
    Ok(omd_root()?
        .join(PACKAGE_META_PATH)
        .join(pckg_name.0.clone()))
}

pub async fn get_pckg_version(
    _pckg_name: &PackageName,
) -> Result<PackageVersion> {
    panic!("get_pckg_version is not yet implemented in this version of the omd agent");
    // Ok(PackageVersion(fs::read_to_string(get_pckg_meta(pckg_name)?.join("version")).await?))
}

pub async fn load_config(opts: &Options) -> Result<HostConfig> {
    let config_file = omd_root()?.join(format!(
        "{}/{}/{}.json",
        CONFIG_PATH,
        match opts.mode {
            Mode::Inventory => "inventory",
            Mode::Active => "active",
            Mode::Current => "current",
        },
        &opts.host_name
    ));

    debug!("loading configfile: {:?}", &config_file);
    let file = File::open(&config_file).await?;
    file.lock_shared()?;
    Ok(serde_json::from_str(
        &fs::read_to_string(&config_file).await?,
    )?)
}
