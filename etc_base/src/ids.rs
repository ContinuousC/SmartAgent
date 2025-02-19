/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::convert::TryFrom;
use std::fmt;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use agent_utils::Key;
use agent_utils::TryAppend;

/* Key newtypes for type-safe key usage. */

#[derive(
    Serialize,
    Deserialize,
    Debug,
    Clone,
    Key,
    Hash,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
)]
#[serde(transparent)]
pub struct Protocol(pub String);

#[derive(
    Serialize,
    Deserialize,
    Debug,
    Clone,
    Key,
    Hash,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
)]
#[serde(transparent)]
pub struct PackageName(pub String);

#[derive(
    Serialize,
    Deserialize,
    Debug,
    Clone,
    Key,
    Hash,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
)]
#[serde(transparent)]
pub struct PackageVersion(pub String);

/// Protocol-specific data table id.
#[derive(
    Serialize,
    Deserialize,
    Debug,
    Clone,
    Key,
    Hash,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
)]
#[serde(transparent)]
pub struct ProtoDataTableId(pub String);

/// Protocol-specific data field id.
#[derive(
    Serialize,
    Deserialize,
    Debug,
    Clone,
    Key,
    Hash,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
)]
#[serde(transparent)]
pub struct ProtoDataFieldId(pub String);

#[derive(
    Serialize,
    Deserialize,
    Debug,
    Clone,
    //    Key,
    Hash,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
)]
#[serde(try_from = "String")]
#[serde(into = "String")]
pub struct DataTableId(pub Protocol, pub ProtoDataTableId);

#[derive(
    Serialize,
    Deserialize,
    Debug,
    Clone,
    //    Key,
    Hash,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
)]
#[serde(try_from = "String")]
#[serde(into = "String")]
pub struct DataFieldId(pub Protocol, pub ProtoDataFieldId);

impl TryFrom<String> for DataTableId {
    type Error = ConvertError;
    fn try_from(val: String) -> Result<Self, ConvertError> {
        match val.split_once('_') {
            Some((proto, id)) => Ok(Self(
                Protocol(proto.to_string()),
                ProtoDataTableId(id.to_string()),
            )),
            None => Err(ConvertError::MissingTableProtoPrefix(val.to_string())),
        }
    }
}

impl TryFrom<String> for DataFieldId {
    type Error = ConvertError;
    fn try_from(val: String) -> Result<Self, ConvertError> {
        match val.split_once('_') {
            Some((proto, id)) => Ok(Self(
                Protocol(proto.to_string()),
                ProtoDataFieldId(id.to_string()),
            )),
            None => Err(ConvertError::MissingFieldProtoPrefix(val.to_string())),
        }
    }
}

impl From<DataTableId> for String {
    fn from(val: DataTableId) -> Self {
        format!("{}_{}", val.0 .0, val.1 .0)
    }
}

impl From<DataFieldId> for String {
    fn from(val: DataFieldId) -> Self {
        format!("{}_{}", val.0 .0, val.1 .0)
    }
}

impl fmt::Display for DataTableId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DataTableId {}_{}", self.0 .0, self.1 .0)
    }
}

impl fmt::Display for DataFieldId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DataFieldId {}_{}", self.0 .0, self.1 .0)
    }
}

impl agent_utils::Key for DataTableId {}
impl agent_utils::Key for DataFieldId {}

#[derive(
    Serialize,
    Deserialize,
    Debug,
    Clone,
    Key,
    Hash,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
)]
#[serde(transparent)]
pub struct QueryId(String);

#[derive(
    Serialize,
    Deserialize,
    Debug,
    Clone,
    Key,
    Hash,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
)]
#[serde(transparent)]
pub struct MPId(String);

#[derive(
    Serialize,
    Deserialize,
    Debug,
    Clone,
    Key,
    Hash,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
)]
#[serde(transparent)]
pub struct CheckId(pub String);

#[derive(
    Serialize,
    Deserialize,
    Debug,
    Clone,
    Key,
    Hash,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
)]
#[serde(transparent)]
pub struct TableId(pub String);

#[derive(
    Serialize,
    Deserialize,
    Debug,
    Clone,
    Key,
    Hash,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
)]
#[serde(transparent)]
pub struct FieldId(pub String);

#[derive(
    Serialize,
    Deserialize,
    Debug,
    Clone,
    Key,
    Hash,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
)]
#[serde(transparent)]
pub struct JoinKey(String);

#[derive(
    Serialize,
    Deserialize,
    Debug,
    Clone,
    Key,
    Hash,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
)]
#[serde(transparent)]
pub struct Tag(pub String);

#[derive(Error, Debug)]
pub enum ConvertError {
    #[error("Missing protocol prefix in data table id: {0}")]
    MissingTableProtoPrefix(String),
    #[error("Missing protocol prefix in data field id: {0}")]
    MissingFieldProtoPrefix(String),
}
