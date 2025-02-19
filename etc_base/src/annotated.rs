/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::fmt::Display;

use serde::{Deserialize, Serialize};

use logger::Verbosity;

/// Value with associated warnings
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Annotated<T, W: Display> {
    pub value: T,
    pub warnings: Vec<Warning<W>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Warning<T: Display> {
    pub verbosity: Verbosity,
    pub message: T,
}

impl<T: Display> Warning<T> {
    pub fn new(verbosity: Verbosity, message: T) -> Self {
        Self { verbosity, message }
    }
    pub fn debug(message: T) -> Self {
        Self::new(Verbosity::Debug, message)
    }
    pub fn info(message: T) -> Self {
        Self::new(Verbosity::Info, message)
    }
    pub fn warn(message: T) -> Self {
        Self::new(Verbosity::Warning, message)
    }
    pub fn log(&self) {
        match self.verbosity {
            Verbosity::Debug => log::debug!("{}", self.message),
            Verbosity::Info => log::info!("{}", self.message),
            Verbosity::Warning => log::warn!("{}", self.message),
        }
    }
}

/// Fallible value with associated warnings.
pub type AnnotatedResult<T, W, E> = Result<Annotated<T, W>, E>;

impl<T, W: Display> Annotated<T, W> {
    pub fn map<U, F>(self, f: F) -> Annotated<U, W>
    where
        F: FnOnce(T) -> U,
    {
        Annotated {
            value: f(self.value),
            warnings: self.warnings,
        }
    }

    pub fn map_warning<V: Display, F>(self, mut f: F) -> Annotated<T, V>
    where
        F: FnMut(W) -> V,
    {
        Annotated {
            value: self.value,
            warnings: self
                .warnings
                .into_iter()
                .map(|Warning { verbosity, message }| Warning {
                    verbosity,
                    message: f(message),
                })
                .collect(),
        }
    }
}
