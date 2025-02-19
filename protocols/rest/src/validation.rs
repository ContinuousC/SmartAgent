/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use super::error::RESTError;

use jsonschema::JSONSchema;
use serde_json::Value;

pub fn validate_json(
    json: &String,
    schema: &Value,
) -> Result<Value, RESTError> {
    let json = serde_json::from_str(json)?;
    let schema = JSONSchema::compile(schema)
        .map_err(|e| RESTError::CompilationError(e.to_string()))?;
    if let Err(errors) = schema.validate(&json) {
        let mut errs: Vec<String> = Vec::new();
        for error in errors {
            errs.push(format!("{:?}", error));
        }
        return Err(RESTError::ValidationError(errs));
    }
    Ok(json)
}
