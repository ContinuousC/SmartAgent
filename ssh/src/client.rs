/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use thrussh::client::Handler;

use super::error::{Error, Result};

pub struct Client {}

impl Client {
    pub fn new() -> Self {
        Self {}
    }
}

impl Handler for Client {
    type Error = Error;
    type FutureUnit =
        futures::future::Ready<Result<(Self, thrussh::client::Session)>>;
    type FutureBool = futures::future::Ready<Result<(Self, bool)>>;

    fn finished_bool(self, b: bool) -> Self::FutureBool {
        futures::future::ready(Ok((self, b)))
    }

    fn finished(self, session: thrussh::client::Session) -> Self::FutureUnit {
        futures::future::ready(Ok((self, session)))
    }

    fn check_server_key(
        self,
        server_public_key: &thrussh_keys::key::PublicKey,
    ) -> Self::FutureBool {
        println!("check_server_key: {:?}", server_public_key);
        self.finished_bool(true)
    }
}
