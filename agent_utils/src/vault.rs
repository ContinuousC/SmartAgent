/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::os::unix::io::{FromRawFd, RawFd};
use std::sync::Arc;

pub use key_reader::Creds;
use key_reader::{stream::nonblocking::CmdStream, Req, Res};
use log::debug;
use tokio::net::unix::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::UnixStream;
use tokio::sync::Mutex;

use crate::error::{Error, Result};

#[derive(Clone)]
pub enum KeyVault {
    Identity,
    KeyReader(KeyReaderVaultSock),
}

#[derive(Clone)]
pub struct KeyReaderVaultSock(
    Arc<Mutex<CmdStream<OwnedReadHalf, OwnedWriteHalf, Res, Req>>>,
);

impl KeyVault {
    pub fn new_identity() -> Self {
        Self::Identity
    }
    pub fn new_key_reader(fd: RawFd) -> Result<Self> {
        Ok(Self::KeyReader(KeyReaderVaultSock::new(fd)?))
    }

    pub async fn retrieve_creds(&self, key: String) -> Result<Option<Creds>> {
        match self {
            KeyVault::Identity => Ok(None),
            KeyVault::KeyReader(sock) => sock.retrieve_creds(key).await,
        }
    }

    pub async fn retrieve_password(&self, key: String) -> Result<String> {
        match self {
            KeyVault::Identity => Ok(key),
            KeyVault::KeyReader(sock) => sock.retrieve_password(key).await,
        }
    }
}

impl KeyReaderVaultSock {
    pub fn new(fd: RawFd) -> Result<Self> {
        let stream = unsafe { std::os::unix::net::UnixStream::from_raw_fd(fd) };
        stream.set_nonblocking(true)?;
        let (r, w) = UnixStream::from_std(stream)?.into_split();
        Ok(Self(Arc::new(Mutex::new(CmdStream::new(r, w)))))
    }

    pub async fn retrieve_creds(&self, key: String) -> Result<Option<Creds>> {
        debug!("Password Vault: Retrieving credentials for {}", &key);
        let mut locked_stream = self.0.lock().await;
        locked_stream.send(&Req::GetCreds(key)).await?;
        Ok(
            match locked_stream.recv().await?.ok_or(Error::MissingPWEntry)? {
                Res::Creds(creds) => Some(creds),
                Res::NotFound(_) => None,
            },
        )
    }

    pub async fn retrieve_password(&self, key: String) -> Result<String> {
        self.retrieve_creds(key)
            .await?
            .ok_or(Error::MissingPWEntry)?
            .password
            .ok_or(Error::MissingPassword)
    }
}
