/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use crate::error::Result;
//use agent_utils::TryGetFrom;
use etc::Spec;
use protocol::ErrorCategory;

/// "Data Table" and "Protocol" to use in dependency check.
pub fn to_data_table_and_protocol(
    cat: &ErrorCategory,
    _spec: &Spec,
) -> Result<(String, String)> {
    match cat {
        ErrorCategory::DataTable(id) => {
            Ok((id.1.to_string(), id.0.to_string()))
        }
        ErrorCategory::Protocol(id) => Ok((id.to_string(), id.to_string())),
        ErrorCategory::Agent => {
            Ok(("Agent".to_string(), "General".to_string()))
        }
        ErrorCategory::Query => {
            Ok(("Query".to_string(), "General".to_string()))
        }
        ErrorCategory::ETC => Ok(("ETC".to_string(), "General".to_string())),
    }
}
