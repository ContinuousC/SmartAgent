/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

mod error;

use std::collections::HashMap;
use std::path::PathBuf;
use std::process;

use clap::{App, Arg};
use tokio::fs;

use agent_utils::{KeyVault, TryGetFrom};
use etc::{EtcManager, QueryMode, Source};
use etc_base::{DataTableId, PackageName, PackageVersion};
use expression::{row::ExprRow, EvalError, EvalOpts, Expr};
use protocol::PluginManager;
use value::{DataError, TypeOpts};

use error::Result;

#[tokio::main]
async fn main() {
    let matches = App::new("Smart Type Checker")
        .arg(
            Arg::with_name("strict-strings")
                .long("strict-strings")
                .help(
                    "Do not perform implicit conversion between \
                                        binary and unicode strings.",
                ),
        )
        .arg(
            Arg::with_name("pkgs")
                .help("One or more ETC packages.")
                .takes_value(true)
                .multiple(true),
        )
        .get_matches();

    match run(
        &EvalOpts {
            types: TypeOpts {
                strict_strings: matches.is_present("strict-strings"),
            },
        },
        &matches
            .values_of("pkgs")
            .map_or_else(Vec::new, |vs| vs.collect::<Vec<_>>()),
    )
    .await
    {
        Ok(i) => process::exit(i),
        Err(e) => {
            eprintln!("{}", e);
            process::exit(2);
        }
    }
}

async fn run(eval_opts: &EvalOpts, pkgs: &[&str]) -> Result<i32> {
    /* Load specification(s). */
    let vault = KeyVault::Identity;
    let cache_path = PathBuf::from("/tmp/smart-agent");
    let mut plugin_manager = PluginManager::new();
    plugin_manager.add_plugin(snmp_protocol::Plugin::new(
        cache_path.clone(),
        vault.clone(),
    ));
    plugin_manager.add_plugin(azure_protocol::Plugin::new(
        cache_path.clone(),
        vault.clone(),
    ));
    plugin_manager.add_plugin(wmi_protocol::Plugin::new(
        cache_path.clone(),
        vault.clone(),
    ));
    plugin_manager.add_plugin(api_protocol::Plugin::new(
        cache_path.clone(),
        vault.clone(),
    ));
    plugin_manager.add_plugin(sql_protocol::Plugin::new(
        cache_path.clone(),
        vault.clone(),
    ));
    plugin_manager.add_plugin(ssh_protocol::Plugin::new(
        cache_path.clone(),
        vault.clone(),
        PathBuf::new(),
        0,
    ));
    plugin_manager.add_plugin(powershell_protocol::Plugin::new(
        cache_path.clone(),
        vault.clone(),
    ));

    let etc_manager = EtcManager::new();

    for file in pkgs {
        etc_manager
            .load_pkg(
                PackageName(file.to_string()),
                PackageVersion(String::from("1.0")), // TODO
                fs::read_to_string(file).await?,
                &plugin_manager,
            )
            .await?;
    }

    let spec = etc_manager.spec().await;
    let etc = &spec.etc;

    /* Find data table types. */

    let mut type_map = HashMap::new();

    for (prot, prot_input) in &spec.input {
        for (data_table_id, _data_table) in &prot_input.data_tables {
            let table_id = DataTableId(prot.clone(), data_table_id.clone());
            let table_type = spec.get_data_table_type(&table_id)?;
            type_map.insert(table_id, table_type);
        }
    }

    /* Generate data. */

    let mut errors = HashMap::new();
    let mut data_errors = HashMap::new();
    let mut query_errors = HashMap::new();
    let mut table_errors = HashMap::new();

    for query_mode in &[QueryMode::Monitoring, QueryMode::Discovery] {
        for (table_id, table_spec) in &etc.tables {
            /* Skip tables not enabled for mode. */
            if !table_spec.query_for(*query_mode) {
                continue;
            }

            /* Run type-check. */
            let query_type = match table_spec
                .query
                .try_get_from(&etc.queries)?
                .check(&type_map)
            {
                Ok(query_type) => query_type,
                Err(err) => {
                    query_errors.insert(
                        format!(
                            "{} ({:?} mode)",
                            table_spec
                                .name
                                .as_ref()
                                .map_or("unknown", |name| name.as_str()),
                            query_mode
                        ),
                        err,
                    );
                    continue;
                }
            };

            let field_specs = table_spec.fields_for_mode(*query_mode, etc)?;
            let mut data = HashMap::new();

            for (_field_id, field_spec) in &field_specs {
                match &field_spec.source {
                    Source::Data(_, data_field_id, _) => {
                        match query_type.fields.get(data_field_id) {
                            Some(typ) => {
                                data.insert(
                                    field_spec.name.as_str(),
                                    typ.clone(),
                                );
                            }
                            None => {
                                data_errors.insert(
                                    field_spec.name.as_str(),
                                    DataError::Missing,
                                );
                            }
                        }
                    }
                    Source::Config => {
                        data.insert(
                            field_spec.name.as_str(),
                            field_spec.input_type.clone(),
                        );
                    }
                    Source::Formula(_) => {}
                }
            }

            if data.is_empty() {
                table_errors
                    .insert(table_id.0.as_str(), "table contains no fields!");
            }

            if data_errors.is_empty() && !data.is_empty() {
                let expr_row = ExprRow(
                    field_specs
                        .iter()
                        .map(|(_field_id, field_spec)| {
                            (
                                field_spec.name.as_str(),
                                match &field_spec.source {
                                    Source::Data(_, _, e) => {
                                        e.clone().unwrap_or(Expr::Data)
                                    }
                                    Source::Formula(e) => e.clone(),
                                    Source::Config => Expr::Data,
                                },
                            )
                        })
                        .collect(),
                );

                let row = expr_row.check_opts(data, eval_opts);

                /* Save errors. */

                for ((_field_id, field_spec), (field_name, field_type)) in
                    field_specs.iter().zip(row.0)
                {
                    match field_type {
                        Ok(field_type) => {
                            if !field_type.castable_to_opts(
                                &field_spec.input_type,
                                &eval_opts.types,
                            ) {
                                if field_name.contains("Resource") {
                                    println!(
                                        "{} has inputtype {} and fieldtype: {}",
                                        &field_name,
                                        &field_spec.input_type,
                                        &field_type
                                    );
                                }
                                errors
                                    .entry(format!(
                                        "{} ({:?} mode)",
                                        table_id.0, query_mode
                                    ))
                                    .or_insert_with(HashMap::new)
                                    .insert(
                                        field_name.to_string(),
                                        EvalError::TypeError(
                                            "InputType does not match \
											 calculated field type",
                                        ),
                                    );
                            }
                        }
                        Err(err) => {
                            errors
                                .entry(table_id.0.to_string())
                                .or_insert_with(HashMap::new)
                                .insert(field_name.to_string(), err);
                        }
                    }
                }
            }
        }
    }

    /* Print output. */

    match data_errors.is_empty()
        && errors.is_empty()
        && query_errors.is_empty()
        && table_errors.is_empty()
    {
        true => Ok(0),

        false => {
            /* Failure: output errors. */

            if !data_errors.is_empty() {
                let title = "Data fields";
                eprintln!("{}\n{}", title, "-".repeat(title.len()));
                for (field_name, error) in data_errors {
                    eprintln!("- {}: {}", field_name, error);
                }
                eprintln!();
            }

            if !query_errors.is_empty() {
                let title = "Queries";
                eprintln!("{}\n{}", title, "-".repeat(title.len()));
                for (query_name, error) in query_errors {
                    eprintln!("- {}: {}", query_name, error);
                }
                eprintln!();
            }

            if !table_errors.is_empty() {
                let title = "Tables";
                eprintln!("{}\n{}", title, "-".repeat(title.len()));
                for (table_name, error) in table_errors {
                    eprintln!("- {}: {}", table_name, error);
                }
                eprintln!();
            }

            for (table_name, field_errors) in errors {
                eprintln!(
                    "Table: {}\n{}",
                    table_name,
                    "-".repeat(table_name.len())
                );
                for (field_name, error) in field_errors {
                    eprintln!("- {}: {}", field_name, error);
                }
                eprintln!();
            }

            Ok(1)
        }
    }
}
