/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    fmt::Display,
    hash::Hash,
    sync::Arc,
};

use agent_utils::{KeyVault, TryAppend};
use etc_base::{ProtoDataFieldId, ProtoDataTableId};
use itertools::Itertools;

use serde::{Deserialize, Serialize};

use value::{Data, DataError, EnumValue, IntEnumValue, Type, Value};
use wmi_protocol::WmiCounter;

use crate::{config::Config, error::Result, Error};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "PascalCase")]
pub struct Input {
    pub data_tables: HashMap<ProtoDataTableId, TableSpec>,
    pub data_fields: HashMap<ProtoDataFieldId, FieldSpec>,
}

impl TryAppend for Input {
    fn try_append(&mut self, other: Self) -> agent_utils::Result<()> {
        self.data_tables.extend(other.data_tables);
        self.data_fields.extend(other.data_fields);
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct TableSpec {
    /// A vector containing the selected fields from the table
    pub fields: HashSet<ProtoDataFieldId>,
    /// pretty much anything that is not the first select
    /// this is the statement that generates our table
    #[serde(rename = "SQLTableName")]
    pub sql_table_name: String,
    /// a humanfriendly name of the table.
    /// this is ussually the name of the table you query
    /// if you are using joins and/subqueries; than its time to get creative
    #[serde(rename = "SQLTableQuery")]
    pub sql_table_query: Option<String>,
    /// plugin to be used to create connection strings and table transformations
    pub plugin: Option<SqlPlugin>,
    /// a query used to retrieve all databses from the server.
    /// if this value is set, not only will de databases of the server be retrieved.
    /// but the table_query will be used on all databases.
    /// the databases should be the only item retrieved in this query
    pub database_query: Option<String>,
    /// indicates whther a table is a singleton or not
    pub is_table: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub struct FieldSpec {
    /// the select statement used to retrieve this field
    /// eg: DB_NAME() as database_name = {"DB_NAME() as database_name": database_name}
    pub column_request: String,
    /// the header of the field. this can be the same as the select statement, or the AS string
    pub column_name: String,
    /// The datatype of the field
    pub parameter_type: SqlDataType,
    /// type of counter
    pub counter_type: Option<WmiCounter>,
    /// indicates whther the field is a key of the table or not
    pub is_key: bool,
    /// possible values of the type if this field is an enum
    pub values: Option<ValueTypes>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum ValueTypes {
    Integer(Arc<BTreeMap<i64, String>>),
    String(Arc<BTreeSet<String>>),
}

impl ValueTypes {
    pub fn get_type(&self) -> Type {
        match self {
            Self::Integer(mapping) => Type::IntEnum(mapping.clone()),
            Self::String(values) => Type::Enum(values.clone()),
        }
    }

    pub fn parse_val(&self, value: &str) -> Data {
        Ok(match self {
            Self::String(values) => {
                Value::Enum(EnumValue::new(values.clone(), value.to_string())?)
            }
            Self::Integer(mapping) => {
                let value = value.parse().map_err(|e| {
                    DataError::TypeError(format!(
                        "Cannot parse {value} to an integer-enum: {e}"
                    ))
                })?;
                Value::IntEnum(IntEnumValue::new(mapping.clone(), value)?)
            }
        })
    }
}

impl Hash for FieldSpec {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.column_request.hash(state);
    }
}

impl TableSpec {
    pub fn to_query(&self, fields: &HashSet<&FieldSpec>) -> Result<String> {
        Ok(format!(
            "SELECT {}\n{}",
            fields
                .iter()
                .map(|f| f.column_request.as_str())
                .collect::<Vec<_>>()
                .join(", "),
            match self.sql_table_name.is_empty()
                || self.sql_table_name == "None"
            {
                true => String::new(),
                false => format!("FROM {}", self.sql_table_name),
            }
        ))
    }
}

impl FieldSpec {
    pub fn to_select_statement(&self) -> String {
        format!("{} AS {}", self.column_request, self.column_name)
    }

    pub fn get_type(&self) -> Result<Type> {
        Ok(match self.parameter_type {
            SqlDataType::String => Type::UnicodeString,
            SqlDataType::Enum => self
                .values
                .as_ref()
                .map(|vt| vt.get_type())
                .ok_or(Error::NoValueType)?,
            SqlDataType::Integer | SqlDataType::Difference => Type::Integer,
            SqlDataType::Float | SqlDataType::Counter => Type::Float,
            SqlDataType::Bool => Type::Boolean,
            SqlDataType::DateTime => Type::Time,
            SqlDataType::Age => Type::Age,
            SqlDataType::Binary => {
                unimplemented!("Binary datatype not yet implemented")
            }
        })
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SqlDataType {
    #[serde(
        alias = "CHAR",
        alias = "VARCHAR",
        alias = "TINYTEXT",
        alias = "TEXT",
        alias = "MEDIUMTEXT",
        alias = "LONGTEXT"
    )]
    String,
    #[serde(
        alias = "BINARY",
        alias = "VARBINARY",
        alias = "TINYBLOB",
        alias = "BLOB",
        alias = "MEDIUMBLOB",
        alias = "LONGBLOB"
    )]
    Binary,
    #[serde(alias = "ENUM", alias = "SET")]
    Enum,
    #[serde(
        alias = "BIT",
        alias = "TINYINT",
        alias = "SMALLINT",
        alias = "MEDIUMINT",
        alias = "INT",
        alias = "INTEGER",
        alias = "BIGINT"
    )]
    Integer,
    #[serde(
        alias = "FLOAT",
        alias = "DOUBLE",
        alias = "DOUBLE PRECISION",
        alias = "DECIMAL",
        alias = "DEC",
        alias = "NUMBER"
    )]
    Float,
    #[serde(alias = "BOOL", alias = "BOOLEAN")]
    Bool,
    #[serde(
        alias = "DATE",
        alias = "DATETIME",
        alias = "TIMESTAMP",
        alias = "TIME",
        alias = "YEAR"
    )]
    DateTime,
    Age,
    Counter,
    Difference,
}

impl Display for SqlDataType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                SqlDataType::String => "String",
                SqlDataType::Binary => "Binary",
                SqlDataType::Enum => "enum",
                SqlDataType::Integer => "Integer",
                SqlDataType::Float => "Float",
                SqlDataType::Bool => "Bool",
                SqlDataType::DateTime => "DateTime",
                SqlDataType::Age => "Age",
                SqlDataType::Counter => "Counter",
                SqlDataType::Difference => "Difference",
            }
        )
    }
}

#[derive(
    Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default,
)]
pub enum SqlPlugin {
    #[default]
    #[serde(alias = "ODBC")]
    Odbc,
    #[serde(alias = "MSSQL")]
    Mssql,
    Oracle,
}

impl SqlPlugin {
    pub async fn get_plugin(
        &self,
        prot_plugin: &crate::Plugin,
    ) -> Result<Arc<dyn crate::sqlplugin::SqlPlugin>> {
        Ok(match self {
            SqlPlugin::Odbc => Arc::new(
                crate::sqlplugin::odbc::Plugin::from_protocol_plugin(
                    prot_plugin,
                )
                .await?,
            ),
            SqlPlugin::Mssql => Arc::new(
                crate::sqlplugin::mssql::Plugin::from_protocol_plugin(
                    prot_plugin,
                )
                .await?,
            ),
            SqlPlugin::Oracle => Arc::new(
                crate::sqlplugin::oracle::Plugin::from_protocol_plugin(
                    prot_plugin,
                )
                .await?,
            ),
        })
    }
}

impl Display for SqlPlugin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                SqlPlugin::Odbc => "ODBC",
                SqlPlugin::Mssql => "MSSQL",
                SqlPlugin::Oracle => "Oracle",
            }
        )
    }
}

#[derive(Debug, Default, Clone)]
pub struct ConnectionString {
    inner: BTreeMap<String, String>,
}

impl ConnectionString {
    fn new() -> Self {
        Self::default()
    }
    pub fn with_arg(mut self, key: &str, value: impl Display) -> Self {
        self.inner.insert(key.to_string(), value.to_string());
        self
    }
    pub fn with_args(
        mut self,
        args: impl IntoIterator<Item = (String, String)>,
    ) -> Self {
        self.inner.extend(args);
        self
    }
    pub fn with_arg_if_set(
        mut self,
        key: &str,
        opt: &Option<impl Display>,
    ) -> Self {
        self.add_if_set(key, opt);
        self
    }
    pub fn with_arg_if_set_bool(
        mut self,
        key: &str,
        opt: &Option<bool>,
    ) -> Self {
        self.add_if_set_bool(key, opt);
        self
    }

    pub fn add_arg(&mut self, key: &str, value: impl Display) {
        self.inner.insert(key.to_string(), value.to_string());
    }
    pub fn add_if_set(&mut self, key: &str, opt: &Option<impl Display>) {
        if let Some(v) = opt {
            self.add_arg(key, v);
        }
    }
    pub fn add_if_set_bool(&mut self, key: &str, opt: &Option<bool>) {
        self.add_if_set(
            key,
            &opt.map(|b| match b {
                true => "YES",
                false => "NO",
            }),
        )
    }

    pub async fn from_config(
        config: Arc<Config>,
        kvault: &KeyVault,
    ) -> Result<Self> {
        let mut cs = ConnectionString::new()
            .with_arg("Server", config.hostname.clone())
            .with_args(config.custom_args.clone())
            .with_arg_if_set("Driver", &config.driver)
            .with_arg_if_set("SSL", &config.ssl)
            .with_arg_if_set("DSN", &config.dsn)
            .with_arg_if_set("Database", &config.database)
            .with_arg_if_set(
                "FILEDSN",
                &config.file_dsn.as_ref().map(|p| p.display()),
            )
            .with_arg_if_set(
                "SSLKeyFile",
                &config.ssl_key.as_ref().map(|p| p.display()),
            )
            .with_arg_if_set(
                "SSLCertFile",
                &config.ssl_cert.as_ref().map(|p| p.display()),
            )
            .with_arg_if_set_bool("Encrypt", &config.encrypt)
            .with_arg_if_set_bool(
                "TrustServerCertificate",
                &config.disable_certificate_verification,
            );

        for (key, value) in &config.custom_args {
            std::env::set_var(key, value)
        }

        let (username, password) = match kvault
            .retrieve_creds(config.username.clone().unwrap_or_default())
            .await?
        {
            None => (config.username.clone(), config.password.clone()),
            Some(creds) => (creds.username, creds.password),
        };

        cs.add_if_set("Uid", &username);
        cs.add_if_set("Pwd", &password);

        Ok(cs)
    }
}

impl Display for ConnectionString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.inner.iter().map(|(k, v)| format!("{k}={v}")).join(";")
        )
    }
}
