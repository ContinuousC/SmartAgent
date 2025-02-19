/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

mod enum_buffer;
mod enum_database;
mod enum_process;
mod enum_resource;
mod enum_write_daemon;
pub(crate) mod generic;
mod get_dashboard;
mod get_ecpapp_svr;
mod get_ecpdata_svr;
mod get_global;
mod get_routine;
mod get_system;

pub use enum_buffer::BodyEnumBuffer;
pub use enum_database::BodyEnumDatabase;
pub use enum_process::BodyEnumProcess;
pub use enum_resource::BodyEnumResource;
pub use enum_write_daemon::BodyEnumWriteDaemon;
pub use generic::ApiResponse;

pub use get_dashboard::BodyDashboard;
pub use get_ecpapp_svr::BodyECPAppSvr;
pub use get_ecpdata_svr::BodyECPDataSvr;
pub use get_global::BodyGlobal;
pub use get_routine::BodyRoutine;
pub use get_system::BodySystem;

use protocol::CounterDb;
use std::sync::Mutex;
use std::{sync::Arc, time::SystemTime};

use value::{Data, DataError, Value};

use crate::input::{FieldSpec, ParameterType};

fn parse_val(
    field: &FieldSpec,
    counterdb: &Arc<Mutex<CounterDb>>,
    key: String,
    value: u64,
) -> Data {
    match field.parameter_type {
        ParameterType::Integer => Ok(Value::Integer(value as i64)),
        ParameterType::Float => Ok(Value::Float(value as f64)),
        ParameterType::Counter => {
            counterdb
                .lock()
                .unwrap()
                .counter(key, value, SystemTime::now())
        }
        ParameterType::Difference => {
            counterdb
                .lock()
                .unwrap()
                .difference(key, value, SystemTime::now())
        }
        _ => Err(DataError::TypeError(
            "Expected: integer, float, difference, counter".to_string(),
        )),
    }
}
