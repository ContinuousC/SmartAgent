/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::{watch, RwLock};

use agent_utils::TryAppend;
use etc_base::{PackageName, PackageVersion};
use protocol::PluginManager;

use super::error::{Error, Result};
use super::etc::Etc;
use super::package::Package;
use super::spec::Spec;

pub struct EtcManager {
    packages: RwLock<HashMap<PackageName, (PackageVersion, Package)>>,
    spec_sender: watch::Sender<Arc<Spec>>,
    spec_receiver: watch::Receiver<Arc<Spec>>,
}

impl EtcManager {
    pub fn new() -> Self {
        let (spec_sender, spec_receiver) =
            watch::channel(Arc::new(Spec::default()));

        Self {
            packages: RwLock::new(HashMap::new()),
            spec_sender,
            spec_receiver,
        }
    }

    pub async fn load_pkg(
        &self,
        name: PackageName,
        version: PackageVersion,
        spec: String,
        plugins: &PluginManager,
    ) -> Result<()> {
        let spec = serde_json::from_str(&spec)
            .map_err(|e| Error::PackageData(name.clone(), e))?;
        let mut packages = self.packages.read().await.clone();
        packages.insert(name.clone(), (version, spec));
        self.reload_pkgs(packages, plugins).await
    }

    pub async fn unload_pkg(
        &self,
        name: PackageName,
        plugins: &PluginManager,
    ) -> Result<()> {
        let mut packages = self.packages.read().await.clone();
        packages.remove(&name);
        self.reload_pkgs(packages, plugins).await
    }

    pub async fn loaded_pkgs(
        &self,
    ) -> Result<HashMap<PackageName, PackageVersion>> {
        Ok(self
            .packages
            .read()
            .await
            .iter()
            .map(|(name, (version, _))| (name.clone(), version.clone()))
            .collect())
    }

    pub async fn spec(&self) -> Arc<Spec> {
        self.spec_receiver.borrow().clone()
    }

    pub async fn spec_receiver(&self) -> watch::Receiver<Arc<Spec>> {
        self.spec_receiver.clone()
    }

    async fn reload_pkgs(
        &self,
        packages: HashMap<PackageName, (PackageVersion, Package)>,
        plugins: &PluginManager,
    ) -> Result<()> {
        let mut inputs = Vec::new();
        let mut etc = Etc::default();

        for (_, spec) in packages.values() {
            inputs.push(spec.input.clone());
            etc.try_append(spec.etc.clone())?;
        }

        let spec = Spec {
            etc,
            input: plugins.load_inputs(inputs).await?,
        };

        let mut packages_write = self.packages.write().await;

        self.spec_sender.send(Arc::new(spec))?;
        *packages_write = packages;

        Ok(())
    }
}
