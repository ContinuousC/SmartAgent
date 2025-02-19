/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
    sync::Arc,
    time::SystemTime,
};

use agent_utils::TryAppend;
use chrono::{DateTime, Utc};
use etc_base::{ProtoDataFieldId, ProtoDataTableId, ProtoRow};
use handlebars::{Context, Handlebars};
use itertools::Itertools;
use log::{debug, error, trace};
use protocol::CounterDb;
use serde::{Deserialize, Serialize};
use tap::TapFallible;
use value::{Data, DataError, EnumType, EnumValue, IntEnumValue, Type, Value};

use crate::{
    config::CommandOutput,
    error::{DTEResult, TypeError, TypeResult},
    plugin::{Row, Table},
    DTError,
};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Input {
    pub data_tables: HashMap<ProtoDataTableId, TableSpec>,
    pub data_fields: HashMap<ProtoDataFieldId, FieldSpec>,
    pub data_table_fields: HashMap<ProtoDataTableId, HashSet<ProtoDataFieldId>>,
}

impl TryAppend for Input {
    fn try_append(&mut self, other: Self) -> agent_utils::Result<()> {
        self.data_tables.try_append(other.data_tables)?;
        self.data_fields.try_append(other.data_fields)?;
        self.data_table_fields.try_append(other.data_table_fields)?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct TableSpec {
    pub command_name: String,
    pub command_line: String,
    pub shell_type: ShellType,
    pub output_type: OutputType,
    pub singleton: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct FieldSpec {
    pub parameter_name: String,
    pub parameter_header: String,
    pub parameter_type: ParamType,
    pub is_key: bool,
    pub values: Option<EnumType>,
}

#[derive(
    Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ShellType {
    #[default]
    Powershell,
    Exchange,
}

#[derive(
    Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum OutputType {
    #[default]
    Csv,
    Json,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParamType {
    String,
    Integer,
    Float,
    Boolean,
    Time,
    Age,
    Enum,
    Counter,
    Difference,
}

impl TableSpec {
    pub fn parse_table(
        &self,
        table: Table,
        fields: HashMap<&ProtoDataFieldId, &FieldSpec>,
        counter_db: Arc<CounterDb>,
    ) -> Vec<ProtoRow> {
        table
            .into_iter()
            .enumerate()
            .map(|(idx, mut row)| {
                let key =
                    self.get_rowkey(&row, &fields).unwrap_or(idx.to_string());
                fields
                    .iter()
                    .map(|(field_id, field)| {
                        (
                            (*field_id).clone(),
                            row.remove(&field.parameter_header)
                                .ok_or(DataError::Missing)
                                .map(|val| {
                                    field.parse_value(val, &counter_db, &key)
                                })
                                .and_then(std::convert::identity),
                        )
                    })
                    .collect()
            })
            .collect()
    }
    fn get_rowkey(
        &self,
        row: &HashMap<String, String>,
        fields: &HashMap<&ProtoDataFieldId, &FieldSpec>,
    ) -> Option<String> {
        let keyparts = fields
            .values()
            .filter(|f| f.is_key)
            .filter_map(|f| row.get(&f.parameter_header))
            .map(|s| s.as_str())
            .sorted()
            .collect::<Vec<_>>();
        (!keyparts.is_empty()).then(|| keyparts.join("."))
    }
}

impl FieldSpec {
    pub fn get_type(&self) -> TypeResult<Type> {
        match self.parameter_type {
            ParamType::String => Ok(Type::UnicodeString),
            ParamType::Integer | ParamType::Difference => Ok(Type::Integer),
            ParamType::Float | ParamType::Counter => Ok(Type::Float),
            ParamType::Boolean => Ok(Type::Boolean),
            ParamType::Time => Ok(Type::Time),
            ParamType::Age => Ok(Type::Age),
            ParamType::Enum => match &self.values {
                None => {
                    Err(TypeError::EnumMissingVars(self.parameter_name.clone()))
                }
                Some(EnumType::Integer(ienum)) => {
                    Ok(Type::IntEnum(ienum.clone()))
                }
                Some(EnumType::String(senum)) => Ok(Type::Enum(senum.clone())),
            },
        }
    }

    pub fn parse_value(
        &self,
        value: String,
        counter_db: &Arc<CounterDb>,
        row_key: &str,
    ) -> Data {
        let key = format!("{}.{}", row_key, self.parameter_name);
        match self.parameter_type {
            ParamType::String => Ok(Value::UnicodeString(value)),
            ParamType::Integer => value
                .parse::<i64>()
                .map(Value::Integer)
                .map_err(|e| DataError::TypeError(e.to_string())),
            ParamType::Float => value
                .parse::<f64>()
                .or_else(|_| value.replace(',', ".").parse::<f64>())
                .map(Value::Float)
                .map_err(|e| DataError::TypeError(e.to_string())),
            ParamType::Boolean => match value.to_lowercase().as_str() {
                "true" => Ok(Value::Boolean(true)),
                "false" => Ok(Value::Boolean(false)),
                _ => Err(DataError::TypeError(format!(
                    "{value} is not a boolean"
                ))),
            },
            ParamType::Time => DateTime::parse_from_rfc3339(&value)
                .map(|dt| dt.with_timezone(&Utc))
                .map(Value::Time)
                .map_err(|e: chrono::ParseError| {
                    DataError::TypeError(e.to_string())
                }),
            ParamType::Age => todo!(),
            ParamType::Enum => match &self.values {
                None => Err(DataError::TypeError(
                    "No enum variables set".to_string(),
                )),
                Some(EnumType::Integer(ienum)) => {
                    let value = value
                        .parse::<i64>()
                        .map_err(|e| DataError::TypeError(e.to_string()))?;
                    IntEnumValue::new(ienum.clone(), value).map(Value::IntEnum)
                }
                Some(EnumType::String(senum)) => {
                    EnumValue::new(senum.clone(), value).map(Value::Enum)
                }
            },
            ParamType::Counter => value
                .parse::<u64>()
                .map_err(|e| DataError::TypeError(e.to_string()))
                .map(|value| {
                    counter_db.counter(key, value, SystemTime::now())
                })?,
            ParamType::Difference => value
                .parse::<u64>()
                .map_err(|e| DataError::TypeError(e.to_string()))
                .map(|value| {
                    counter_db.difference(key, value, SystemTime::now())
                })?,
        }
    }
}

impl Display for ShellType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Powershell => "powershell",
                Self::Exchange => "exchange powershell",
            }
        )
    }
}

lazy_static::lazy_static! {
    static ref TEMPLATE_REGISTERY: Handlebars<'static> = {
        let mut reg = Handlebars::new();
        reg.register_escape_fn(handlebars::no_escape);

        reg
    };
}

impl ShellType {
    pub fn parse_command(
        &self,
        script: &str,
        outtype: OutputType,
        ctx: &Context,
    ) -> DTEResult<String> {
        let render_tenplate = |templ: String| {
            debug!("parsing script-template: {templ}");
            trace!("with context: {ctx:?}");

            TEMPLATE_REGISTERY
                .render_template_with_context(&templ, ctx)
                .map_err(DTError::RenderError)
                .tap_ok(|s| trace!("generated script:\n{s}"))
                .tap_err(|e| error!("failed to parse script: {e}"))
        };

        match self {
            Self::Powershell => render_tenplate(format!(
                "Invoke-Command -ScriptBlock {{ {script} }} | {}",
                outtype.as_function()
            )),
            Self::Exchange => {
                let template = format!(
                    r#"$Username = '{{{{username}}}}'
                       $Password = '{{{{password}}}}'

                       $Password = ConvertTo-SecureString -AsPlainText -Force -String $Password
                       $Credential = New-Object -TypeName System.Management.Automation.PSCredential -ArgumentList $Username,$Password
                       $Session = New-PSSession -ConfigurationName Microsoft.Exchange -Credential $Credential `
                                       -ConnectionUri http://$(hostname)/PowerShell/ -Authentication Kerberos
                       Import-PSSession $Session -DisableNameChecking | Out-Null

                       Invoke-Command -ScriptBlock {{ {script} }} | {}"#,
                    outtype.as_function()
                );
                render_tenplate(template)
            }
        }
    }
}

impl OutputType {
    fn as_function(&self) -> &str {
        match self {
            Self::Csv => "ConvertTo-Csv -NoTypeInformation",
            Self::Json => "ConvertTo-Json -Compress -EnumsAsStrings",
        }
    }

    pub fn parse_table(&self, output: CommandOutput) -> DTEResult<Table> {
        let output = output.into_result()?;

        match self {
            Self::Csv => csv::ReaderBuilder::new()
                .delimiter(b',')
                .has_headers(true)
                .from_reader(output.stdout.as_bytes())
                .deserialize::<Row>()
                .map(|r| {
                    let r: std::result::Result<Row, _> = r;
                    r
                })
                .collect::<std::result::Result<Table, csv::Error>>()
                .map_err(DTError::CsvDeserialize),
            Self::Json => serde_json::from_str(&output.stdout)
                .map_err(DTError::JsonDeserialize),
        }
    }
}
