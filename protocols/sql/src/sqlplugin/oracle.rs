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

use itertools::Itertools;
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

    fn pivot_table(table: Table) -> DTEResult<Table> {
        let keyfields: Vec<_> = table
            .first()
            .map(|row| {
                row.iter()
                    .filter(|(k, _)| !["NAME", "VALUE"].contains(&k.as_str()))
                    .map(|(k, v)| Ok((k.clone(), v.clone())))
                    .collect()
            })
            .unwrap_or_default();

        let row = table
            .into_iter()
            .map(|mut row| {
                Ok((
                    row.remove("NAME").ok_or(DTError::FieldNotFound("NAME"))?,
                    row.remove("VALUE")
                        .ok_or(DTError::FieldNotFound("VALUE"))?,
                ))
            })
            .chain(keyfields)
            .collect::<DTEResult<HashMap<String, String>>>()?;

        Ok(vec![row])
    }
}

#[async_trait::async_trait]
impl SqlPlugin for Plugin {
    fn name(&self) -> &'static str {
        "Oracle"
    }

    async fn connection_string_per_instance(
        &self,
        base: ConnectionString,
        config: Arc<Config>,
    ) -> Result<HashMap<InstanceType, ConnectionString>> {
        config
            .instances
            .iter()
            .map(|inst| match inst {
                InstanceType::String(i) => {
                    Ok((inst.clone(), base.clone().with_arg("DSN", i)))
                }
                _ => Err(Error::InvalidInstance(inst.clone(), self.name())),
            })
            .collect()
    }

    fn construct_query(
        &self,
        datatable: &TableSpec,
        datafields: HashSet<&FieldSpec>,
    ) -> DTEResult<String> {
        let name_id = match datatable.sql_table_name.as_str() {
            "V$SYSMETRIC" => "METRIC_NAME",
            "V$SYSSTAT" => "NAME",
            _ => "",
        };

        if name_id.is_empty() {
            return datatable
                .to_query(&datafields)
                .map_err(|e| DTError::ConstructQuery(Box::new(e)));
        }

        let keyfields = datafields
            .iter()
            .filter(|f| f.is_key)
            .map(|f| f.column_request.as_str())
            .collect_vec()
            .join(", ");

        let query = format!(
            "SELECT {name_id} AS NAME, VALUE, {keyfields} FROM {} WHERE {}",
            &datatable.sql_table_name,
            datafields
                .iter()
                .filter(|f| !f.is_key)
                .map(|f| format!("{name_id} = '{}'", &f.column_name))
                .join(" OR ")
        );

        Ok(query)
    }

    fn transform_table<'a>(
        &self,
        spec: &TableSpec,
        table: &'a Table,
    ) -> DTEResult<Cow<'a, Table>> {
        if ["V$SYSMETRIC", "V$SYSSTAT"].contains(&spec.sql_table_name.as_str())
        {
            return Self::pivot_table(table.clone()).map(Cow::Owned);
        }

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
