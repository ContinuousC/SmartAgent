/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::collections::HashSet;
use std::env;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;

use crate::error::LivestatusError;

pub struct LivestatusSocket {
    pub socket: UnixStream,
}

impl LivestatusSocket {
    pub async fn new() -> Result<LivestatusSocket, LivestatusError> {
        let omd_root = match env::var("OMD_ROOT") {
            Ok(s) => Ok(s),
            Err(_) => Err(LivestatusError::NotOMD),
        }?;
        let socket =
            match UnixStream::connect(format!("{}/tmp/run/live", omd_root))
                .await
            {
                Ok(s) => Ok(s),
                Err(e) => Err(LivestatusError::ConnectionError(e)),
            }?;
        Ok(LivestatusSocket { socket })
    }

    pub async fn exec_query(
        &mut self,
        query: String,
    ) -> Result<String, LivestatusError> {
        if let Err(e) = self.socket.write_all(query.as_bytes()).await {
            return Err(LivestatusError::WriteError(e));
        }
        let mut response = String::new();
        if let Err(e) = self.socket.read_to_string(&mut response).await {
            return Err(LivestatusError::ReadError(e));
        }
        Ok(response)
    }

    pub async fn get_hosts(
        &mut self,
    ) -> Result<HashSet<String>, LivestatusError> {
        let output = self
            .exec_query(String::from("GET hosts\nColumns: name\n\n"))
            .await?;
        Ok(output
            .split('\n')
            .map(|s| s.to_string())
            .filter(|s| !s.is_empty())
            .collect::<HashSet<String>>())
    }
}
