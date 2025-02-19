/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::collections::{HashMap, HashSet};

use agent_utils::TryGetFrom;
use etc_base::{DataFieldId, DataTableId, Protocol, QueryMap, TableId};
use protocol::Input;
use query::{KeySet, QueryType};

use super::error::Result;
use super::etc::Etc;
use super::query_mode::QueryMode;
use super::source::Source;

/// In-memory representation of EventTypeCatalog definitions.
#[derive(Default, Debug)]
pub struct Spec {
    pub input: HashMap<Protocol, Input>,
    pub etc: Etc,
}

impl Spec {
    pub fn queries_for(
        &self,
        table_ids: &HashSet<TableId>,
        query_mode: QueryMode,
    ) -> Result<QueryMap> {
        let mut prot_queries = HashMap::new();

        for table_id in table_ids {
            let table = table_id.try_get_from(&self.etc.tables)?;
            match query_mode {
                QueryMode::Discovery => {
                    if !table.discovery {
                        continue;
                    }
                }
                QueryMode::Monitoring => {
                    if !table.monitoring {
                        continue;
                    }
                }
                QueryMode::CheckMk => {
                    if !table.check_mk.unwrap_or(table.monitoring) {
                        continue;
                    }
                }
            }
            // TODO: get this info from the query.
            for field_id in &table.fields {
                let field = field_id.try_get_from(&self.etc.fields)?;
                match query_mode {
                    QueryMode::Discovery => {
                        if !field.discovery {
                            continue;
                        }
                    }
                    QueryMode::Monitoring => {
                        if !field.monitoring {
                            continue;
                        }
                    }
                    QueryMode::CheckMk => {
                        if !field.check_mk.unwrap_or(field.monitoring) {
                            continue;
                        }
                    }
                }
                if let Source::Data(data_table_id, data_field_id, _) =
                    &field.source
                {
                    prot_queries
                        .entry(data_table_id.0.clone())
                        .or_insert_with(HashMap::new)
                        .entry(data_table_id.1.clone())
                        .or_insert_with(HashSet::new)
                        .insert(data_field_id.1.clone());
                }
            }
        }

        /* Add all keys (needed for joins, counters). */
        for (protocol, data_tables) in prot_queries.iter_mut() {
            let proto_input = protocol.try_get_from(&self.input)?;
            for (data_table_id, data_fields) in data_tables.iter_mut() {
                let data_table =
                    data_table_id.try_get_from(&proto_input.data_tables)?;
                for data_field_id in &data_table.keys {
                    data_fields.insert(data_field_id.clone());
                }
            }
        }

        Ok(prot_queries)
    }

    /// Find the type of the table.
    pub fn get_data_table_type(
        &self,
        table_id: &DataTableId,
    ) -> Result<QueryType> {
        let DataTableId(proto, table_id) = table_id;
        let input = proto.try_get_from(&self.input)?;
        let table = table_id.try_get_from(&input.data_tables)?;

        Ok(QueryType {
            singleton: table.singleton,
            fields: table
                .fields
                .iter()
                .map(|field_id| {
                    Ok((
                        DataFieldId(proto.clone(), field_id.clone()),
                        field_id
                            .try_get_from(&input.data_fields)?
                            .input_type
                            .clone(),
                    ))
                })
                .collect::<Result<_>>()?,
            keys: KeySet::from_simple(
                table
                    .keys
                    .iter()
                    .map(|field_id| {
                        DataFieldId(proto.clone(), field_id.clone())
                    })
                    .collect(),
            ),
        })
    }
}
