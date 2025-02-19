/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    fmt::Display,
    sync::Arc,
    time::SystemTime,
};

use protocol::CounterDb;
use value::{Data, DataError};

use crate::{
    Config, ConnectionString, DTEResult, DTError, Error, FieldSpec,
    InstanceType, Result, Table, TableSpec,
};

use super::SqlPlugin;

#[derive(Debug)]
pub struct Plugin(CounterDb);

impl Plugin {
    pub async fn from_protocol_plugin(
        prot_plugin: &crate::Plugin,
    ) -> Result<Self> {
        CounterDb::load(prot_plugin.cache_dir.join("sql_counters.json"))
            .await
            .map(Self)
            .map_err(Error::CounterDbCreation)
    }
}

#[async_trait::async_trait]
impl SqlPlugin for Plugin {
    fn name(&self) -> &'static str {
        "ODBC"
    }

    async fn connection_string_per_instance(
        &self,
        base: ConnectionString,
        config: Arc<Config>,
    ) -> Result<HashMap<InstanceType, ConnectionString>> {
        config
            .instances
            .iter()
            .map(|inst| {
                if let InstanceType::Port(p) = inst {
                    Ok((
                        inst.clone(),
                        base.clone().with_arg("Port", p.to_string()),
                    ))
                } else {
                    Err(Error::InvalidInstance(inst.clone(), self.name()))
                }
            })
            .collect()
    }
    fn construct_query(
        &self,
        datatable: &TableSpec,
        datafields: HashSet<&FieldSpec>,
    ) -> DTEResult<String> {
        datatable
            .to_query(&datafields)
            .map_err(|e| DTError::ConstructQuery(Box::new(e)))
    }
    fn transform_table<'a>(
        &self,
        _spec: &TableSpec,
        table: &'a Table,
    ) -> DTEResult<Cow<'a, Table>> {
        Ok(Cow::Borrowed(table))
    }

    async fn save_counters(&self) -> Result<()> {
        self.0.save().await.map_err(Error::CounterDbSave)
    }
    fn parse_counter(
        &self,
        row: &mut HashMap<String, String>,
        field: &FieldSpec,
        base_key: &str,
    ) -> Data {
        let val = row.remove(&field.column_name).ok_or(DataError::Missing)?;
        let val = val.parse().map_err(|_| {
            DataError::Parse(val, field.parameter_type.to_string())
        })?;
        self.0.counter(
            format!("{}.{}", base_key, field.column_name),
            val,
            SystemTime::now(),
        )
    }
    fn parse_difference(
        &self,
        row: &mut HashMap<String, String>,
        field: &FieldSpec,
        base_key: &str,
    ) -> Data {
        let val = row.remove(&field.column_name).ok_or(DataError::Missing)?;
        let val = val.parse().map_err(|_| {
            DataError::Parse(val, field.parameter_type.to_string())
        })?;
        self.0.difference(
            format!("{}.{}", base_key, field.column_name),
            val,
            SystemTime::now(),
        )
    }
}

impl Display for Plugin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}
