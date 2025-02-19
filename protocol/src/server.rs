/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use serde_json::Value;

pub struct ProtocolServer<T> {
    input: T::Input,
    plugin: T,
}

type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
enum Error {
    #[error("Invalid ETC input format: {0}")]
    InputFormat(serde_json::Error),
    #[error("Incompatible inputs: {0}")]
    InputIncompatible(agent_utils::Error),
    #[error("Invalid config format: {0}")]
    ConfigFormat(serde_json::Error),
    #[error("Protocol plugin error: {0}")]
    Plugin(Box<dyn Error + Send + Sync + 'static>),
}

#[async_trait]
impl<T: Plugin> ProtocolService for ProtocolServer<T> {
    fn protocol(&self) -> Protocol {
        Protocol(String::from(T::PROTOCOL))
    }

    async fn append_input(&mut self, input: Value) -> Result<()> {
        self.input
            .try_append(
                serde_json::from_value(value)
                    .map_err(Error::InputFormat)?,
            )
            .map_err(Error::InputIncompatible)
    }

    async fn show_queries(
        &self,
        query: &QueryMap,
        config: &Self::Config,
    ) -> Result<String> {
        self.plugin
            .show_queries(
                query,
                &self.input,
                &serde_json::from_value(config).map_err(Error::ConfigFormat),
            )
            .map_err(Error::Plugin)
    }

    async fn run_queries(
        &self,
        query: &QueryMap,
        config: Value,
    ) -> Result<DataMap> {
        self.plugin
            .run_queries(
                query,
                &self.input,
                &serde_json::from_value(config).map_err(Error::ConfigFormat),
            )
            .map_err(Error::Plugin)
    }
}
