/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

mod format;
mod schema;

use std::collections::HashMap;

use wasm_bindgen::prelude::*;

use dbschema::{DbSchema, DbTable, VersioningType};
use etc::QueryMode;

#[wasm_bindgen]
pub fn format_metric(metric: JsValue, field_spec: JsValue) -> JsValue {
    throw_errors(move || {
        serde_wasm_bindgen::to_value(&format::FormattedField::from_metric(
            &serde_wasm_bindgen::from_value(metric)
                .map_err(|e| e.to_string())?,
            &serde_wasm_bindgen::from_value(field_spec)
                .map_err(|e| e.to_string())?,
        )?)
        .map_err(|e| e.to_string())
    })
}

#[wasm_bindgen]
pub fn format(
    value: JsValue,
    rel_value: JsValue,
    field_spec: JsValue,
) -> String {
    throw_errors(move || {
        format::format(
            serde_wasm_bindgen::from_value(value).map_err(|e| e.to_string())?,
            serde_wasm_bindgen::from_value(rel_value)
                .map_err(|e| e.to_string())?,
            serde_wasm_bindgen::from_value(field_spec)
                .map_err(|e| e.to_string())?,
        )
    })
}

// #[wasm_bindgen]
// pub fn convert(value: f64, unit: &JsValue) -> f64 {
//     throw_errors(|| {
//         format::convert(
//             value,
//             JsValue::into_serde(unit).map_err(|e| e.to_string())?,
//         )
//         .map_err(|e| e.to_string())
//     })
// }

#[wasm_bindgen]
pub fn metric_schemas(pkg: JsValue) -> JsValue {
    throw_errors(move || {
        serde_wasm_bindgen::to_value(
            &schema::load_schemas(
                serde_wasm_bindgen::from_value(pkg)
                    .map_err(|e| e.to_string())?,
                QueryMode::Monitoring,
            )
            .map_err(|e| e.to_string())?
            .into_iter()
            .filter_map(|(key, schema)| match schema {
                DbSchema::Struct(schema) => Some((
                    key,
                    DbTable {
                        versioning: VersioningType::SingleTimeline,
                        force_update: false,
                        schema,
                    },
                )),
                _ => None,
            })
            .collect::<HashMap<_, _>>(),
        )
        .map_err(|e| e.to_string())
    })
}

#[wasm_bindgen]
pub fn discovery_schemas(pkg: JsValue) -> JsValue {
    throw_errors(move || {
        serde_wasm_bindgen::to_value(
            &schema::load_schemas(
                serde_wasm_bindgen::from_value(pkg)
                    .map_err(|e| e.to_string())?,
                QueryMode::Discovery,
            )
            .map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())
    })
}

fn throw_errors<F, T>(fun: F) -> T
where
    F: FnOnce() -> Result<T, String>,
{
    match (fun)() {
        Ok(v) => v,
        Err(e) => wasm_bindgen::throw_str(e.as_str()),
    }
}
