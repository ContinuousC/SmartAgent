/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::collections::{HashMap, HashSet};
use std::iter::once;

use serde::{Deserialize, Serialize};

use etc_base::{Annotated, DataFieldId, DataTableId, Row, RowType, Warning};
use logger::Verbosity;
use protocol::{DataMap, ErrorOrigin};

use super::compat::{self, TableQuery};
use super::error::{
    AnnotatedQueryResult, QueryCheckResult, QueryError, QueryResult,
    QueryTypeError, QueryWarning,
};
use super::join::{self, JoinOperand};
use super::key_set::KeySet;
use super::prefilter::PreFilter;
use super::reindex::{self, Select};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum Query {
    Data(DataTableId, ErrorAction, bool),
    Filter(PreFilter, Box<Query>),
    Join(JoinOperand, JoinOperand),
    Reindex(Vec<DataFieldId>, Select, Box<Query>),
    // Compat with excel-ETCs
    TableQueries(Vec<TableQuery>),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct QueryType {
    pub keys: KeySet,
    pub fields: RowType,
    pub singleton: bool,
}

pub type TypeMap = HashMap<DataTableId, QueryType>;

/// The error action controls what happens to a table query when a data table query fails:
/// - Fail: the query fails with the error message of the data table query
/// - Warn: an empty table is used and the error is transformed into a warning message
/// - Info: an empty table is used and the error is transformed into an informational message
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ErrorAction {
    Fail,
    Warn,
    Info,
}

impl Default for ErrorAction {
    fn default() -> Self {
        Self::Warn
    }
}

impl Query {
    /// Run a table query given the retrieved protocol data
    pub fn run(&self, data: &DataMap) -> AnnotatedQueryResult<Vec<Row>> {
        match self.eval(data) {
            Ok(Annotated {
                value: (table, true),
                warnings,
            }) => Ok(Annotated {
                value: table,
                warnings,
            }),
            Ok(Annotated {
                value: (_, false),
                warnings,
            }) => Err(QueryError::DoesntExist(warnings)),
            Err(e) => Err(e),
        }
    }

    /// Evaluate a table query given the retrieved protocol data,
    /// returning Ok(((result,exists),warnings)) or Err(error).
    fn eval(&self, data: &DataMap) -> AnnotatedQueryResult<(Vec<Row>, bool)> {
        match self {
            Query::Data(data_table_id, error_action, ignore_existence) => {
                let severity = match error_action {
                    ErrorAction::Fail | ErrorAction::Warn => Verbosity::Warning,
                    ErrorAction::Info => Verbosity::Info,
                };
                match data.get(data_table_id) {
                    Some(Ok(Annotated { value: table, warnings})) => Ok(Annotated {
                        value: (table.to_vec(), !ignore_existence),
                        warnings: warnings
                            .iter()
                            .map(|Warning{message, ..}| {
                                Warning {
                                    verbosity: Verbosity::Info, /* Could be configurable in ETC. */
                                    message: QueryWarning::DTWarning(message.clone()),
                                }
                            })
                            .collect(),
                    }),
                    Some(Err(err)) => {
                        if let ErrorOrigin::Protocol(_) = &err.origin {
                            Err(QueryError::Protocol(err.clone()))
                        } else if let ErrorAction::Fail = error_action {
                            Err(QueryError::Protocol(err.clone()))
                        } else {
                            Ok(Annotated{
                                value: (Vec::new(), false),
                                warnings: once(Warning { verbosity: severity, message: QueryWarning::DTError(err.clone()) }).collect(),
                            })
                        }
                    }
                    None => Err(QueryTypeError::MissingDataTable(data_table_id.clone()))?,
                }
            }
            Query::Filter(filter, query) => match query.eval(data)? {
                Annotated {
                    value: (table, exists),
                    warnings,
                } => Ok(Annotated {
                    value: (
                        table
                            .into_iter()
                            .filter_map(|r| match filter.run(&r) {
                                Ok(true) => Some(Ok(r)),
                                Ok(false) => None,
                                Err(e) => Some(Err(e)),
                            })
                            .collect::<QueryResult<_>>()?,
                        exists,
                    ),
                    warnings,
                }),
            },
            Query::Join(left, right) => {
                match (left.query.eval(data)?, right.query.eval(data)?) {
                    (
                        Annotated {
                            value: (ltable, lexists),
                            warnings: lwarnings,
                        },
                        Annotated {
                            value: (rtable, rexists),
                            warnings: rwarnings,
                        },
                    ) => Ok(Annotated {
                        value: (
                            join::run(left, right, ltable, rtable)?,
                            lexists || rexists,
                        ),
                        warnings: lwarnings
                            .into_iter()
                            .chain(rwarnings)
                            .collect(),
                    }),
                }
            }
            Query::Reindex(fields, select, query) => match query.eval(data)? {
                Annotated {
                    value: (table, exists),
                    warnings,
                } => Ok(Annotated {
                    value: (reindex::run(fields, select, table)?, exists),
                    warnings,
                }),
            },
            // Support TableQueries for backward compatibility:
            Query::TableQueries(queries) => {
                compat::transform_table_queries(queries.to_vec())?.eval(data)
            }
        }
    }

    /// Type-check the query.
    pub fn check(&self, data: &TypeMap) -> QueryCheckResult<QueryType> {
        match self {
            Query::Data(data_table_id, _error_action, _ignore_existence) => {
                match data.get(data_table_id) {
                    Some(data_table) => Ok(data_table.clone()),
                    None => Err(QueryTypeError::MissingDataTable(
                        data_table_id.clone(),
                    )),
                }
            }
            Query::Filter(filter, query) => {
                let table = query.check(data)?;
                filter.check(table)
            }
            Query::Join(left, right) => {
                let ltable = left.query.check(data)?;
                let rtable = right.query.check(data)?;
                join::check(left, right, ltable, rtable)
            }
            Query::Reindex(fields, _, query) => {
                let table = query.check(data)?;
                reindex::check(fields, table)
            }
            // Support TableQueries for backward compatibility:
            Query::TableQueries(queries) => {
                let table = compat::transform_table_queries(queries.to_vec())?;
                table.check(data)
            }
        }
    }

    /// Generate a list of data tables required for the query.
    pub fn required_data_tables(&self) -> HashSet<DataTableId> {
        match self {
            Query::Data(data_table, _, _) => once(data_table.clone()).collect(),
            Query::Filter(_, query) => query.required_data_tables(),
            Query::Join(left, right) => left
                .query
                .required_data_tables()
                .into_iter()
                .chain(right.query.required_data_tables())
                .collect(),
            Query::Reindex(_, _, query) => query.required_data_tables(),
            Query::TableQueries(qs) => {
                qs.iter().map(|q| q.data_table.clone()).collect()
            }
        }
    }

    /*pub fn required_data(&self) -> HashMap<DataTableId,HashSet<DataFieldId>> {
    match self {
        Query::Data(data_table,_) => once((data_table.clone(),HashSet::new())).collect(),
        Query::Filter(_,query) => query.required_data(),
        Query::Join(left,right) => combine_query_maps(left.query.required_data_tables(),
                              right.query.required_data_tables()),
        Query::TableQueries(qs) => qs.into_iter()
        .map(|q| q.data_table.clone())
        .collect()
    }
    }*/
}

/*
fn combine_query_maps(a: HashMap<DataTableId,HashSet<DataFieldId>>,
              b: HashMap<DataTableId,HashSet<DataFieldId>>)
              -> HashMap<DataTableId,HashSet<DataFieldId>> {

}
*/
