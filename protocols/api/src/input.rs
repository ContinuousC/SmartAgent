/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::collections::hash_map::Entry;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fmt::{self, Display};
use std::sync::Arc;

use agent_utils::TryAppend;
use etc_base::{DataFieldId, DataTableId};
use serde::{Deserialize, Serialize};
use value::Type;

use crate::error::{TypeError, TypeResult};

/* API-specific IDs. */

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub struct PluginId(pub String);

impl fmt::Display for PluginId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/* API Input specification. */

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "PascalCase")]
#[derive(Default)]
pub struct Input {
    pub data_tables: HashMap<DataTableId, TableSpec>,
    pub data_fields: HashMap<DataFieldId, FieldSpec>,
    #[serde(default)] // backwards compatibility with older spec files
    pub data_table_fields: HashMap<DataTableId, HashSet<DataFieldId>>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub struct TableSpec {
    /// Plugins can be built-in or external processes (called via the API API)
    pub(super) plugin: PluginId,
    /// A human-readable name for the command (table)
    pub(super) command_name: String,
    /// Plugin-specific command definition (url + json path, ...)
    pub(super) command_line: String,
    /// A human-readable description of the command (table)
    pub(super) command_description: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct FieldSpec {
    /// A human-readable name for the paramater (field)
    pub(super) parameter_name: String,
    /// Deprecated
    pub(super) parameter_header: String,
    /// Deprecated
    pub(super) parameter_type: ParameterType,
    pub(super) values: Option<ValueTypes>,
    pub(super) is_key: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum ValueTypes {
    Integer(
        #[serde(with = "agent_serde::arc_intkey_map")]
        Arc<BTreeMap<i64, String>>,
    ),
    String(Arc<BTreeSet<String>>),
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ParameterType {
    Float,
    Integer,
    String,
    Boolean,
    Time,
    Age,
    Enum,
    Counter,
    Difference,
    #[serde(alias = "ipaddr")]
    IpAddress,
}

impl FieldSpec {
    pub fn get_type(&self) -> TypeResult<Type> {
        match self.parameter_type {
            ParameterType::Float => Ok(Type::Float),
            ParameterType::Integer => Ok(Type::Integer),
            ParameterType::String => Ok(Type::UnicodeString),
            ParameterType::Boolean => Ok(Type::Boolean),
            ParameterType::Time => Ok(Type::Time),
            ParameterType::Age => Ok(Type::Age),
            ParameterType::Enum => Ok(
                match self.values.as_ref().ok_or(
                    TypeError::EnumMissingVariables(
                        self.parameter_name.clone(),
                    ),
                )? {
                    ValueTypes::Integer(values) => {
                        Type::IntEnum(values.clone())
                    }
                    ValueTypes::String(values) => Type::Enum(values.clone()),
                },
            ),
            ParameterType::Counter => Ok(Type::Float),
            ParameterType::Difference => Ok(Type::Integer),
            ParameterType::IpAddress => Ok(Type::Ipv4Address),
        }
    }
}

impl TryAppend for Input {
    fn try_append(&mut self, other: Self) -> agent_utils::Result<()> {
        self.data_tables.try_append(other.data_tables)?;
        self.data_fields.try_append(other.data_fields)?;
        for (k, v) in other.data_table_fields {
            match self.data_table_fields.entry(k) {
                Entry::Vacant(entry) => {
                    entry.insert(v);
                }
                Entry::Occupied(mut entry) => entry.get_mut().extend(v),
            }
        }
        Ok(())
    }
}

impl Display for ParameterType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                ParameterType::Float => "Float",
                ParameterType::Integer => "Integer",
                ParameterType::String => "String",
                ParameterType::Boolean => "Boolean",
                ParameterType::Time => "Time",
                ParameterType::Age => "Age",
                ParameterType::Enum => "Enum",
                ParameterType::Counter => "Counter",
                ParameterType::Difference => "Difference",
                ParameterType::IpAddress => "Ip Address",
            }
        )
    }
}
