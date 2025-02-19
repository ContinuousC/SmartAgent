/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use serde::{Deserialize, Deserializer, Serialize};

use etc_base::{DataFieldId, DataTableId, ProtoDataFieldId};
use expression::Expr;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum Source {
    #[serde(deserialize_with = "deserialize_data")]
    Data(DataTableId, DataFieldId, Option<Expr>),
    Formula(Expr),
    Config,
}

/// Added in v1.07 / mnChecks_SmartM v0.99.42
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum Source2 {
    #[serde(deserialize_with = "deserialize_data")]
    Data(DataTableId, DataFieldId, Option<Expr>),
    Formula(Expr),
    Config(Option<Expr>),
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
enum DataCompat {
    V3(DataTableId, DataFieldId, Option<Expr>),
    V2(DataTableId, ProtoDataFieldId, Option<Expr>),
    V1(DataTableId, ProtoDataFieldId),
}

fn deserialize_data<'de, D>(
    deserializer: D,
) -> Result<(DataTableId, DataFieldId, Option<Expr>), D::Error>
where
    D: Deserializer<'de>,
{
    match DataCompat::deserialize(deserializer)? {
        DataCompat::V3(table, field, expr) => Ok((table, field, expr)),
        DataCompat::V2(table, field, expr) => {
            let proto = table.0.clone();
            Ok((table, DataFieldId(proto, field), expr))
        }
        DataCompat::V1(table, field) => {
            let proto = table.0.clone();
            Ok((table, DataFieldId(proto, field), None))
        }
    }
}
