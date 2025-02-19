/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::collections::{HashMap, HashSet};
use std::io::BufRead;
use std::{fs, io, path::Path};

use agent_utils::TryGetFrom;
use etc_base::{Annotated, CheckId, TableId, Warning};
use logger::Verbosity;
use protocol::ErrorCategory;
use query::QueryError;

use super::output::TableData;
use crate::context::Context;
use crate::error::{Error, Result};

pub(crate) struct Problem {
    pub(crate) mp: String,
    pub(crate) check: String,
    pub(crate) data_table: String,
    pub(crate) protocol: String,
    pub(crate) success: bool,
    pub(crate) message: String,
}

pub(crate) fn load_problems(path: &Path) -> Result<Option<Vec<Problem>>> {
    log::debug!("Error file: {}", path.display());
    path.exists()
        .then(|| {
            io::BufReader::new(fs::File::open(path)?)
                .lines()
                .map(|line| Problem::from_line(&line?))
                .collect::<Result<Vec<_>>>()
        })
        .transpose()
}

/* Find problems for our checks. Note: this is extremely quirky.
 *
 * Among others, it is unclear what is the unit described by each
 * line of the dependency cache file and what to do with non-datatable
 * errors.
 *
 * Our input is a list of severities and messages from the query. Messages
 * need to come through this channel (and not directly from the data problem
 * map) because it is the query that determines if missing or problematic
 * data tables are problematic for the check.
 *
 * The error file contains columns to indicate the data table and protocol
 * and requires status on each line. The dependency check concatenates
 * all messages for every check and protocol combination, adding severity
 * and data table prefixes to each line. Therefore, we output one message
 * per line, with its severity and the "protocol" and "data table" it
 * refers to.
 *
 * However, the check also expects at least one line for every combination
 * of check and protocol to build the rows in its output table. If no error
 * messages are returned for a check and protocol combination, we therefore
 * add a line with "ok" status and empty error message.
 *
 */
pub(crate) fn get_problems(
    checks: &HashMap<CheckId, HashSet<TableId>>,
    data: &HashMap<TableId, TableData>,
    ctx: &Context,
) -> Result<Vec<Problem>> {
    // (check, protocol) -> ( (data table, msg) -> warn )
    let mut problems: HashMap<
        (CheckId, String),
        HashMap<(String, String), bool>,
    > = HashMap::new();

    checks
        .iter()
        .try_for_each::<_, Result<()>>(|(check_id, table_ids)| {
            table_ids.iter().try_for_each::<_, Result<()>>(|table_id| {
                let table = table_id.try_get_from(&ctx.spec.etc.tables)?;
                let query = table.query.try_get_from(&ctx.spec.etc.queries)?;

                /* Add errors grouped by (check,protocol). */
                match data.get(table_id) {
                    Some(Ok(Annotated { warnings, value: _ }))
                    | Some(Err(QueryError::DoesntExist(warnings))) => {
                        for Warning {
                            verbosity,
                            message: warning,
                        } in warnings
                        {
                            let (data_table, protocol) = warning
                                .omd_category()
                                .to_data_table_and_protocol()?;
                            let warn = problems
                                .entry((check_id.clone(), protocol.to_string()))
                                .or_insert_with(HashMap::new)
                                .entry((
                                    data_table.to_string(),
                                    warning.omd_message(),
                                ))
                                .or_insert(false);
                            *warn = *warn || *verbosity <= Verbosity::Warning;
                        }
                    }
                    Some(Err(error)) => {
                        let (data_table, protocol) = error
                            .omd_category()
                            .to_data_table_and_protocol()?;
                        let warn = problems
                            .entry((check_id.clone(), protocol.to_string()))
                            .or_insert_with(HashMap::new)
                            .entry((
                                data_table.to_string(),
                                error.omd_message(),
                            ))
                            .or_insert(false);
                        *warn = true;
                    }
                    None => {
                        let (data_table, protocol) = ErrorCategory::Agent
                            .to_data_table_and_protocol()?;
                        let warn = problems
                            .entry((check_id.clone(), protocol.to_string()))
                            .or_insert_with(HashMap::new)
                            .entry((
                                data_table.to_string(),
                                format!("Missing {}", table_id),
                            ))
                            .or_insert(false);
                        *warn = true;
                    }
                }

                /* Make sure "unfailed" protocols are present as well. */
                query
                    .required_data_tables()
                    .iter()
                    .try_for_each::<_, Result<()>>(|data_table_id| {
                        problems
                            .entry((
                                check_id.clone(),
                                data_table_id.0 .0.to_string(),
                            ))
                            .or_insert_with(HashMap::new);
                        Ok(())
                    })?;

                Ok(())
            })?;

            Ok(())
        })?;

    /* Output problems. */
    let mut result = Vec::new();
    problems.into_iter().try_for_each::<_, Result<()>>(
        |((check_id, protocol), msgs)| {
            let check = check_id.try_get_from(&ctx.spec.etc.checks)?;
            let mp = check.mp.try_get_from(&ctx.spec.etc.mps)?;
            let check_name = format!("{} {}", &mp.name, &check.name);

            let mut sorted_warn_msgs: Vec<_> =
                msgs.iter().filter(|(_, &v)| v).map(|(k, _)| k).collect();
            let mut sorted_info_msgs: Vec<_> =
                msgs.iter().filter(|(_, &v)| !v).map(|(k, _)| k).collect();
            sorted_warn_msgs.sort();
            sorted_info_msgs.sort();

            match sorted_warn_msgs.is_empty()
                && (!ctx.config.agent.show_table_info
                    || sorted_info_msgs.is_empty())
            {
                true => result.push(Problem {
                    mp: mp.name.to_string(),
                    check: check_name,
                    data_table: "".to_string(),
                    protocol: protocol.to_string(),
                    success: true,
                    message: "".to_string(),
                }),
                false => {
                    for (data_table, msg) in sorted_warn_msgs {
                        result.push(Problem {
                            mp: mp.name.to_string(),
                            check: check_name.to_string(),
                            data_table: data_table.to_string(),
                            protocol: protocol.to_string(),
                            success: false,
                            message: msg.to_string(),
                        });
                    }
                    if ctx.config.agent.show_table_info {
                        for (data_table, msg) in sorted_info_msgs {
                            result.push(Problem {
                                mp: mp.name.to_string(),
                                check: check_name.to_string(),
                                data_table: data_table.to_string(),
                                protocol: protocol.to_string(),
                                success: true,
                                message: msg.to_string(),
                            });
                        }
                    }
                }
            }
            Ok(())
        },
    )?;
    Ok(result)
}

impl Problem {
    pub(crate) fn from_line(line: &str) -> Result<Problem> {
        let fields = line.splitn(6, ';').collect::<Vec<_>>();
        let n = fields.len();
        let mut fields = fields.into_iter();

        Ok(Problem {
            mp: if n == 6 {
                fields
                    .next()
                    .ok_or(Error::MissingDependencyField("mp"))?
                    .to_string()
            } else {
                "".to_string()
            },
            check: fields
                .next()
                .ok_or(Error::MissingDependencyField("check"))?
                .to_string(),
            data_table: fields
                .next()
                .ok_or(Error::MissingDependencyField("data_table"))?
                .to_string(),
            protocol: fields
                .next()
                .ok_or(Error::MissingDependencyField("protocol"))?
                .to_string(),
            success: match fields
                .next()
                .ok_or(Error::MissingDependencyField("success"))?
            {
                "True" => Ok(true),
                "False" => Ok(false),
                val => Err(Error::InvalidDependencyField(
                    "success",
                    val.to_string(),
                )),
            }?,
            message: fields
                .next()
                .ok_or(Error::MissingDependencyField("message"))?
                .to_string(),
        })
    }
    pub(crate) fn into_line(&self) -> String {
        format!(
            "{};{};{};{};{};{}",
            self.mp,
            self.check,
            self.data_table,
            self.protocol,
            match self.success {
                true => "True",
                false => "False",
            },
            self.message
        )
    }
}
