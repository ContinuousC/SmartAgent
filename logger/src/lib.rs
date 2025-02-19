/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Clone,
    Copy,
    Debug,
)]
#[serde(rename_all = "snake_case")]
pub enum Verbosity {
    Warning,
    Info,
    Debug,
}

impl fmt::Display for Verbosity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Warning => write!(f, "Warning"),
            Self::Info => write!(f, "Info"),
            Self::Debug => write!(f, "Debug"),
        }
    }
}

#[macro_export]
macro_rules! log_warning {
    ($verbosity:expr,$($arg:tt)+) => {
	if let Some(v) = $verbosity {
	    if v >= logger::Verbosity::Warning {
		eprint!("Warning: ");
		eprintln!($($arg)*)
	    }
	}
    }
}

#[macro_export]
macro_rules! log_info {
    ($verbosity:expr,$($arg:tt)+) => {
	if let Some(v) = $verbosity {
	    if v >= logger::Verbosity::Info {
		eprint!("Info: ");
		eprintln!($($arg)*)
	    }
	}
    }
}

#[macro_export]
macro_rules! log_debug {
    ($verbosity:expr,$($arg:tt)+) => {
	if let Some(v) = $verbosity {
	    if v >= logger::Verbosity::Debug {
		eprint!("Debug: ");
		eprintln!($($arg)*)
	    }
	}
    }
}
