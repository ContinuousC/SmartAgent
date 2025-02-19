/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::{fmt::Display, sync::Arc};

use log::{debug, warn};
use protocol::auth::BasicAuth;
use reqwest::{cookie::Jar, IntoUrl};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use tap::TapFallible;
use tokio::sync::OnceCell;

use super::{
    resource::{LxcStatus, Resource, VmStatus},
    DTEResult, DTError, Error, Plugin, Result,
};

#[derive(Debug)]
pub struct Client<'a> {
    inner: reqwest::Client,
    pub(crate) base_url: String,
    pub(crate) node: String,
    pub(crate) plugin: &'a Plugin,

    pub(crate) qemus: OnceCell<DTEResult<Vec<VmStatus>>>,
    pub(crate) lxcs: OnceCell<DTEResult<Vec<LxcStatus>>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct PveObjectResponse<T> {
    data: T,
}

impl<'a> Client<'a> {
    pub fn new(
        inner: reqwest::Client,
        node: &str,
        base_url: String,
        plugin: &'a Plugin,
    ) -> Self {
        let node = node
            .split_once('.')
            .map(|(before, _after)| before)
            .unwrap_or(node)
            .to_string();

        Self {
            qemus: OnceCell::new(),
            lxcs: OnceCell::new(),
            inner,
            base_url,
            node,
            plugin,
        }
    }

    pub(crate) async fn login(
        &self,
        auth: BasicAuth,
        cookiejar: Arc<Jar>,
    ) -> Result<()> {
        #[derive(Debug, Clone, Serialize, Deserialize)]
        struct Creds {
            #[serde(rename = "CSRFPreventionToken")]
            csrf_prevention_token: String,
            ticket: String,
            username: String,
        }

        let url = format!("{}/access/ticket", self.base_url);
        let data = serde_urlencoded::to_string(&auth).unwrap();
        debug!("sending post to {url}: {data}");

        let creds = self
            .inner
            .post(url)
            .body(data)
            .send()
            .await
            .map_err(Error::SendRequest)?
            .error_for_status()
            .map_err(Error::FailedLogin)?
            .json::<PveObjectResponse<Creds>>()
            .await
            .map_err(Error::DeserializeResponse)?
            .data;

        cookiejar.add_cookie_str(
            &format!("PVEAuthCookie={}", creds.ticket),
            &self.base_url.parse().unwrap(),
        );

        Ok(())
    }

    async fn request<T: DeserializeOwned, U: Display + IntoUrl>(
        &self,
        url: U,
    ) -> DTEResult<T> {
        let surl = url.to_string();
        debug!("requesting resource: {surl}");
        self.inner
            .get(url)
            .send()
            .await
            .map_err(DTError::SendRequest)?
            .error_for_status()
            .map_err(DTError::InvalidResponse)?
            .json::<PveObjectResponse<T>>()
            .await
            .map_err(DTError::DeserializeResponse)
            .map(|r| r.data)
            .tap_ok(|_| debug!("requesting {surl} was successfull"))
            .tap_err(|e| warn!("requesting {surl} was unsuccessfull: {e}"))
    }

    pub async fn request_list<T: DeserializeOwned, U: Display + IntoUrl>(
        &self,
        url: U,
    ) -> DTEResult<Vec<T>> {
        self.request(url).await
    }

    pub async fn request_resource<T: Resource>(&self) -> DTEResult<T> {
        self.request(format!("{}/{}", self.base_url, T::ENDPOINT))
            .await
    }

    pub async fn request_resourcelist<T: Resource>(&self) -> DTEResult<Vec<T>> {
        self.request_list(format!("{}/{}", self.base_url, T::ENDPOINT))
            .await
    }

    pub(crate) fn node_resource(&self, resource: &str) -> String {
        format!("{}/nodes/{}/{}", self.base_url, self.node, resource)
    }
    pub async fn request_noderesource<T: Resource>(&self) -> DTEResult<T> {
        self.request(self.node_resource(T::ENDPOINT)).await
    }
    pub async fn request_noderesources<T: Resource>(
        &self,
    ) -> DTEResult<Vec<T>> {
        self.request_list(self.node_resource(T::ENDPOINT)).await
    }

    pub(crate) async fn get_qemus(&self) -> DTEResult<&[VmStatus]> {
        self.qemus
            .get_or_init(|| self.request_noderesources())
            .await
            .as_deref()
            .map_err(|e| DTError::Custom(e.to_string()))
    }
    pub async fn get_qemuids(&self) -> DTEResult<Vec<u64>> {
        self.get_qemus()
            .await
            .map(|res| res.iter().map(|q| q.vmid).collect())
    }
    pub(crate) async fn get_lcxs(&self) -> DTEResult<&[LxcStatus]> {
        self.lxcs
            .get_or_init(|| self.request_noderesources())
            .await
            .as_deref()
            .map_err(|e| DTError::Custom(e.to_string()))
    }
    pub async fn get_lxcids(&self) -> DTEResult<Vec<u64>> {
        self.get_lcxs()
            .await
            .map(|res| res.iter().map(|q| q.vmid).collect())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub(crate) enum VmType {
    Qemu,
    Lxc,
}

impl TryFrom<&str> for VmType {
    type Error = DTError;

    fn try_from(value: &str) -> DTEResult<Self> {
        if value.contains("{qemuid}") {
            Ok(Self::Qemu)
        } else if value.contains("{lxcid}") {
            Ok(VmType::Lxc)
        } else {
            Err(DTError::InvalidVmType(value.to_string()))
        }
    }
}

impl Display for VmType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Lxc => "lcx",
                Self::Qemu => "qemu",
            }
        )
    }
}
