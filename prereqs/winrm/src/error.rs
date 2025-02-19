/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use colored::Colorize;
use log::{info, warn};
use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Received an invalid argument: {0}")]
    InvalidArg(String),
    #[error("A required argument was not set: {0}")]
    RequiredArg(String),
    #[error("Error with cmk: {0}")]
    Cmk(#[from] crate::cmk::Error),
    #[error("Failed to run key-reader: {0}")]
    KeyReader(#[source] std::io::Error),
    #[error("Invalid argument for {0}: {1}")]
    InvalidArgument(&'static str, clap::parser::MatchesError),
    #[error("{0}")]
    Utils(#[from] agent_utils::Error),
    #[error("IO: {0}")]
    IO(#[from] std::io::Error),
    #[error("No password given")]
    NoPasswordGiven,
    #[error("No entry for the keyvault given. when using the keyvault, enter the required entry in the username")]
    NoKVaultEntryGiven,
    #[error("Entry not found in keyvault")]
    MissingKREntry,
    #[error("Entry Does not contain {0}")]
    MissingKRObject(String),
    #[error("{0}")]
    Custom(String),
    #[error("Winrm Error: {0}")]
    WinRM(#[from] winrm_rs::Error),
    #[error("Unable to use the provided certificate: {0}")]
    Cert(String),
    #[error("Unable to parse the value '{0}' to a {1}")]
    ParseError(String, String),
    #[error("Powershell command failed. Exitcode: {0}")]
    CommandFailed(i32),
}

pub struct TestResult {
    hostname: String,
    failed_test: Option<String>,
    result: Result<()>,
}

impl TestResult {
    pub fn new(
        hostname: String,
        failed_test: Option<String>,
        result: Result<()>,
    ) -> Self {
        TestResult {
            hostname,
            failed_test,
            result,
        }
    }

    pub fn log_outcome(&self) {
        if let Some(test) = &self.failed_test {
            if let Err(e) = &self.result {
                warn!(
                    "Host '{}' {} a test ({}): {}",
                    self.hostname,
                    "failed".red(),
                    test,
                    e
                );
            } else {
                warn!(
                    "Host '{}' {} a test ({}), but i do not know why",
                    self.hostname,
                    "failed".red(),
                    test
                );
            }
        } else {
            info!(
                "Host '{}' has {} all tests!",
                self.hostname,
                "passed".green()
            );
        }
    }

    pub fn is_success(&self) -> bool {
        self.failed_test.is_none()
    }
}
