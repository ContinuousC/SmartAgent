/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::collections::{HashMap, HashSet};
use std::ffi::OsString;
use std::fs::File;
use std::os::unix::fs::MetadataExt;
use std::os::unix::io::RawFd;
use std::os::unix::process::CommandExt;

use std::process::{self, Command};
use std::time::Instant;

use clap::{App, Arg};
use log::{debug, info, warn};
use tokio::fs;

use agent_utils::{quote_filename, vault::KeyVault, TryGetFrom};
use etc::{EtcManager, QueryMode};
use etc_base::{
    Annotated, CheckId, MPId, PackageName, PackageVersion, TableId, Tag,
};
use expression::EvalCell;
use protocol::PluginManager;

use omd_agent::config::PasswordVault;
use omd_agent::context::{Context, Mode, Options};
use omd_agent::error::{Error, Result};
use omd_agent::formula::calculate_table;
use omd_agent::output::TableData;
use omd_agent::{env, omd_root};

#[tokio::main]
async fn main() {
    // console_subscriber::init();

    if let Err(err) = agent().await {
        eprintln!("Error: {}", err);
        process::exit(1)
    }
}

async fn agent() -> Result<()> {
    /* Argument parsing */

    let matches = App::new("SmartM Agent")
        .version(std::env!("CARGO_PKG_VERSION"))
        .author("Maarten Deprez <mdp@si-int.eu>")
        .author("Vincent Stuyck <vst@si-int.eu>")
        .about("Retrieves data from remote systems and outputs to nagios and elasticsearch.")
		.arg(Arg::with_name("host").long("host").short("H").takes_value(true).required(true)
			.help("The name of the target host as defined in WATO. The IP address will be \
				used to connect to the host, if configured; else a lookup is performed."))
		.arg(Arg::with_name("ip").long("ip").short("I").takes_value(true).required(false)
			.help("The ip address of the host."))
		.arg(Arg::with_name("omd_compat").long("omd-compat").short("c")
			.help("Turn on OMD compatibility mode."))
		.arg(Arg::with_name("inventory").long("inventory").short("i")
			.help("Run in inventory mode.").conflicts_with("active"))
		.arg(Arg::with_name("active").long("active").short("a")
			.help("Use active config instead of current config."))
		.arg(Arg::with_name("checks").long("checks").takes_value(true)
			.help("Restrict to certain check types."))
		.arg(Arg::with_name("auth-sock").long("auth-sock").short("C").takes_value(true)
			.help("KeyReader socket fd to use to obtain credentials.").hidden(true))
		.arg(Arg::with_name("verbose").long("verbose").short("v").multiple(true)
			.help("Increase verbosity. This option can be specified multiple times. \
				The maximum verbosity level is 3. Note that this option is NOT \
				compatible with WATO inventory!"))
		.arg(Arg::with_name("show-queries").long("show-queries").short("q")
			.help("Output a list of queries instead of running them."))
			.get_matches();

    let log_level = match matches.occurrences_of("verbose") {
        0 => simplelog::LevelFilter::Off,
        1 => simplelog::LevelFilter::Error,
        2 => simplelog::LevelFilter::Warn,
        3 => simplelog::LevelFilter::Info,
        4 => simplelog::LevelFilter::Debug,
        5.. => simplelog::LevelFilter::Trace,
    };

    // enable logging
    if let Err(e) = simplelog::TermLogger::init(
        log_level,
        simplelog::ConfigBuilder::new()
            .add_filter_ignore_str("serde_xml_rs")
            .add_filter_ignore_str("handlebars")
            .add_filter_ignore_str("want")
            .add_filter_ignore_str("mio")
            .add_filter_ignore_str("odbc_api")
            .add_filter_ignore_str("hyper_util")
            .add_filter_ignore_str("cookie_store")
            .add_filter_ignore_str("reqwest")
            .add_filter_ignore_str("hyper")
            .add_filter_ignore_str("tracing")
            .add_filter_ignore_str("rustls")
            .build(),
        simplelog::TerminalMode::Stderr,
        simplelog::ColorChoice::Auto,
    ) {
        eprintln!("Error: failed to initialize logging: {}", e);
        process::exit(1);
    }
    info!("Starting omd agent");

    let host_name = matches.value_of("host").unwrap().to_string();
    let host_addr = matches.value_of("ip").map(String::from);
    let omd_compat = matches.is_present("omd_compat");
    let mode = match (
        matches.is_present("inventory"),
        matches.is_present("active"),
    ) {
        (true, _) => Mode::Inventory,
        (_, true) => Mode::Active,
        _ => Mode::Current,
    };
    let checks = matches.value_of("checks").map(|checks| {
        checks.split(',').map(|c| CheckId(c.to_string())).collect()
    });

    let options = Options {
        host_name,
        host_addr,
        omd_compat,
        mode,
        checks,
    };
    info!("With options: {:?}", &options);

    /* Load config. */

    let start = Instant::now();

    let config = env::load_config(&options).await?;
    // info!("With config: {:?}", &config);

    let duration = Instant::now().duration_since(start);
    info!(
        "Benchmark: loading config took {:.03}s",
        duration.as_secs_f64()
    );

    /* If using key-reader, connect to key-reader socker. */

    let vault = match config.agent.use_password_vault {
        Some(PasswordVault::KeePass) => match matches.value_of("auth-sock") {
            Some(auth_sock) => {
                let fd: RawFd = auth_sock.parse().map_err(|_| {
                    Error::InvalidArgument(
                        "auth socket fd",
                        auth_sock.to_string(),
                    )
                })?;
                KeyVault::new_key_reader(fd)?
            }
            None => {
                let args: Vec<OsString> = std::env::args_os().skip(1).collect();
                return Err(Error::KeyReader(
                    Command::new("/usr/bin/key-reader")
                        .args(
                            [
                                OsString::from("connect"),
                                OsString::from("--"),
                                std::env::current_exe()?.as_os_str().to_owned(),
                                OsString::from("-C"),
                                OsString::from("SOCK"),
                            ]
                            .iter()
                            .chain(&args),
                        )
                        .exec(),
                ));
            }
        },
        None => KeyVault::new_identity(),
    };

    match vault {
        KeyVault::Identity => info!("no keyvault configured"),
        KeyVault::KeyReader(_) => info!("keyreader configured"),
    }

    /* Load protocol plugins. */
    let cache_path = (env::get_cache_path()?).join(&options.host_name);
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
        omd_root()?.join("local/share/mnow/ssh_parsers"),
        matches.occurrences_of("verbose") as u8,
    ));
    plugin_manager.add_plugin(powershell_protocol::Plugin::new(
        cache_path.clone(),
        vault.clone(),
    ));

    info!(
        "loaded plugins: {:?}",
        plugin_manager
            .get_protocols()
            .into_iter()
            .map(|p| p.0)
            .collect::<Vec<String>>()
            .join(", ")
    );

    /* Load specification files. */

    let start = Instant::now();
    let etc_manager = EtcManager::new();
    let spec_paths = env::get_mp_specs()?;

    info!("loading specs");
    for path in spec_paths {
        info!("loading spec: {:?}", &path);

        if let Some(PasswordVault::KeePass) = config.agent.use_password_vault {
            let st = fs::metadata(&path).await?;
            if st.uid() != 0
                || st.gid() != 0
                || st.mode() & 0o777113 != 0o100000
            {
                warn!("Ignoring specification with invalid owner or permissions: {}", path.display());
                continue;
            }
        }

        match File::open(&path) {
            Err(e) => warn!(
                "Failed to open MP specification {}: {}",
                path.display(),
                e
            ),
            Ok(_) => {
                let pckg_name = PackageName(String::from(
                    path.file_name()
                        .ok_or(Error::InvalidSpecFileName(path.clone()))?
                        .to_str()
                        .ok_or(Error::InvalidSpecFileName(path.clone()))?,
                )); //there is always a filename
                    // TODO: implemented for omd
                    // let pckg_version = env::get_pckg_version(&pckg_name).await?;
                let pckg_version = PackageVersion(String::from("1.00"));
                info!("loading package: {} {}", pckg_name, pckg_version);
                if let Err(e) = etc_manager
                    .load_pkg(
                        pckg_name.clone(),
                        pckg_version,
                        fs::read_to_string(&path).await?,
                        &plugin_manager,
                    )
                    .await
                {
                    warn!("Unable to load spec {}: {}", pckg_name, e);
                }
            }
        }
    }

    let duration = Instant::now().duration_since(start);
    info!(
        "Benchmark: loading MPs took {:.03}s",
        duration.as_secs_f64()
    );

    /* Load host configuration. */

    let ctx = Context {
        options,
        spec: etc_manager.spec().await,
        site_name: env::get_site_name()?,
        parsers_dir: env::get_parsers_path()?,
        cache_dir: env::get_cache_path()?,
        config,
    };
    info!("created context");

    /* Build list of data tables per protocol. Check tags to find enabled MPs
     * and filter by list of enabled checks (if specified on the command line).
     */

    let start = Instant::now();
    info!(
        "loaded mps: {:?}",
        ctx.spec
            .etc
            .mps
            .values()
            .map(|mp| mp.tag.clone())
            .collect::<HashSet<Tag>>()
    );
    info!("calculate mps for tags: {:?}", ctx.config.tags);
    let mps: HashSet<&MPId> = ctx.get_mps();
    info!("calculated mps: {:?}", mps);

    let checks = match ctx.options.mode {
        Mode::Inventory => ctx.options.checks.clone(),
        Mode::Current => ctx.options.checks.clone(),
        Mode::Active => match ctx.config.agent.run_noninventorized_checks {
            true => None,
            false => Some(
                ctx.options
                    .checks
                    .as_ref()
                    .unwrap_or(&ctx.config.checks)
                    .clone(),
            ),
        },
    };

    let mut check_tables: HashMap<CheckId, HashSet<TableId>> = HashMap::new();
    for (check_id, check) in &ctx.spec.etc.checks {
        if mps.contains(&check.mp)
            && checks.as_ref().map_or(true, |cs| cs.contains(check_id))
        {
            let current_check_tables =
                check_tables.entry(check_id.clone()).or_default();

            for table_id in &check.tables {
                let table = table_id.try_get_from(&ctx.spec.etc.tables)?;
                if table.check_mk.unwrap_or(table.monitoring) {
                    current_check_tables.insert(table_id.clone());
                }
            }
        }
    }

    let prot_queries = ctx.spec.queries_for(
        &check_tables
            .values()
            .flatten()
            .cloned()
            .collect::<HashSet<TableId>>(),
        QueryMode::CheckMk,
    )?;

    let duration = Instant::now().duration_since(start);
    info!(
        "Benchmark: calculating query table took {:.03}s",
        duration.as_secs_f64()
    );

    /* Show queries if requested. */

    if matches.is_present("show-queries") {
        plugin_manager.show_queries(&ctx.spec.input, &prot_queries);
    } else {
        /* Run queries. */

        // let mut data = HashMap::new();

        let start = Instant::now();

        let data = plugin_manager
            .run_queries(
                &ctx.spec.input,
                ctx.config.protocols.clone(),
                &prot_queries,
            )
            .await?;

        let duration = Instant::now().duration_since(start);
        info!(
            "Benchmark: running queries took {:.03}s",
            duration.as_secs_f64()
        );

        /* Calculate output tables. */

        let start = Instant::now();
        let mut check_data = HashMap::new();

        for table_id in check_tables
            .values()
            .flatten()
            .cloned()
            .collect::<HashSet<TableId>>()
        {
            let table = table_id.try_get_from(&ctx.spec.etc.tables)?;
            let query = table.query.try_get_from(&ctx.spec.etc.queries)?;

            check_data.insert(
                table_id.clone(),
                match query.run(&data) {
                    Ok(res) => {
                        let table_data =
                            calculate_table(&ctx, table, res.value)?;
                        Ok((table_data, res.warnings))
                    }
                    Err(err) => Err(err),
                },
            );
        }

        let duration = Instant::now().duration_since(start);
        info!(
            "Benchmark: table calculation took {:.03}s",
            duration.as_secs_f64()
        );

        /* Write elastic output. */

        if let Some(smartm_data_config) = &ctx.config.agent.write_smartm_data {
            let start = Instant::now();

            let mut elastic_data = HashMap::new();

            for (table_id, table_data) in check_data.iter() {
                let table_spec = table_id.try_get_from(&ctx.spec.etc.tables)?;
                let mut elastic_table = Vec::new();

                if let Ok((table_data, _)) = table_data {
                    for row in table_data {
                        let row_vars = row
                            .iter()
                            .map(|(field_id, field_data)| {
                                Ok((
                                    field_id
                                        .try_get_from(&ctx.spec.etc.fields)?
                                        .name
                                        .as_str(),
                                    EvalCell::new_evaluated(field_data.clone()),
                                ))
                            })
                            .collect::<Result<_>>()?;

                        let mut elastic_row = HashMap::new();

                        if !table_spec.singleton {
                            if let Some(item_id) = &table_spec.item_id {
                                elastic_row.insert(
                                    elastic::ElasticFieldName(
                                        "item_id".to_string(),
                                    ),
                                    item_id.eval_in_row(Some(&row_vars), None),
                                );
                            }

                            if let Some(item_name) = &table_spec.item_name {
                                elastic_row.insert(
                                    elastic::ElasticFieldName(
                                        "item_name".to_string(),
                                    ),
                                    item_name
                                        .eval_in_row(Some(&row_vars), None),
                                );
                            }
                        }

                        for (field_id, field_data) in row {
                            let field_spec =
                                field_id.try_get_from(&ctx.spec.etc.fields)?;

                            // Quick and dirty solution. TODO: calculate unique field name
                            // for each released mapping from (checked valid and unique)
                            // user-defined name and auto-increasing version number
                            let name =
                                field_id.0.to_lowercase().replace(' ', "_");

                            elastic_row.insert(
                                elastic::ElasticFieldName(name.clone()),
                                field_data.clone(),
                            );

                            /* Reference for relative value. */

                            if let Some(expr) = field_spec.reference.as_ref() {
                                elastic_row.insert(
                                    elastic::ElasticFieldName(format!(
                                        "{}__reference",
                                        name
                                    )),
                                    expr.eval_in_row(Some(&row_vars), None),
                                );
                            }

                            /* Configuration references. */

                            if let Some(refs) = field_spec.references.as_ref() {
                                for (ref_name, expr) in refs {
                                    elastic_row.insert(
                                        elastic::ElasticFieldName(format!(
                                            "{}__references_{}",
                                            name, ref_name
                                        )),
                                        expr.eval_in_row(Some(&row_vars), None),
                                    );
                                }
                            }
                        }

                        elastic_table.push(elastic_row);
                    }
                }

                elastic_data.insert(
                    // Quick and dirty solution. TODO: calculate unique field name
                    // for each released mapping from (checked valid and unique)
                    // user-defined name and auto-increasing version number
                    elastic::ElasticTableName(
                        table_id.0.to_lowercase().replace(' ', "_"),
                    ),
                    elastic_table,
                );
            }

            /* Write data for each instance. */

            for instance in &smartm_data_config.instances {
                if let Err(e) = elastic::write_output(
                    &env::get_data_path()?.join(quote_filename(instance)),
                    &ctx.options.host_name,
                    &ctx.site_name,
                    &elastic_data,
                ) {
                    debug!(
                        "failed to write elastic data for instance {}: {}",
                        instance, e
                    );
                }
            }

            let duration = Instant::now().duration_since(start);
            info!(
                "Benchmark: writing elastic output took {:.03}s",
                duration.as_secs_f64()
            );
        }

        /* Write OMD output. */

        let start = Instant::now();

        let check_data = check_data
            .iter()
            .map(|(tableid, table_result)| {
                (
                    tableid.clone(),
                    table_result.clone().map(|table| Annotated {
                        value: table.0.clone(),
                        warnings: table.1.clone(),
                    }),
                )
            })
            .collect::<HashMap<TableId, TableData>>();

        omd_agent::write_output(&check_tables, &check_data, &ctx)?;

        let duration = Instant::now().duration_since(start);
        info!(
            "Benchmark: writing omd output took {:.03}s",
            duration.as_secs_f64()
        );
    }

    std::mem::drop(plugin_manager);

    std::process::exit(0);
    // Ok(())
}
