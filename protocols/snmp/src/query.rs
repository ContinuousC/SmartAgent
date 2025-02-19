/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};

use futures::{StreamExt, TryStreamExt};
use netsnmp::{Oid, SingleSession};

use agent_utils::{ip_lookup_one, TryGetFrom};
use etc_base::{
    Annotated, AnnotatedResult, ProtoDataTableId, ProtoQueryMap, ProtoRow,
    Warning,
};
use log::debug;
use logger::Verbosity;
use parking_lot::Mutex;
use value::DataError;

use crate::config::ContextSelector;
use crate::Config;

use super::counters::Counters;
use super::error::{DTError, DTWarning, Error, Result, WalkError, WalkWarning};
use super::get::Gets;
use super::index::Index;
use super::input::{Input, ObjectId};
use super::stats::Stats;
use super::walk::{WalkTable, WalkVar, Walks};

pub(super) type Data = std::result::Result<netsnmp::Value, netsnmp::ErrType>;
pub type WalkData = AnnotatedResult<HashMap<Oid, Data>, WalkWarning, WalkError>;
pub(super) type TableData = AnnotatedResult<Vec<ProtoRow>, DTWarning, DTError>;
pub type WalkMap = HashMap<Oid, WalkData>;
pub type DataMap = HashMap<ProtoDataTableId, TableData>;

async fn init_snmp_session(
    snmp: &netsnmp::NetSNMP,
    auth: &Option<netsnmp::Auth>,
    config: &Config,
) -> Result<SingleSession> {
    let ip_addr = match config.ip_addr {
        Some(ip) => ip,
        None => ip_lookup_one(&config.host_name).await?,
    };
    let mut session_builder = snmp.session().set_async_probe(true);

    let peer = match &config.host_config.port {
        Some(port) => format!("{}:{}", ip_addr, port),
        None => ip_addr.to_string(),
    };

    debug!("SNMP: connecting to {}", peer);
    session_builder = session_builder
        .set_peer(peer.as_bytes())
        .map_err(Error::Connection)?;

    if let Some(auth) = auth.as_ref() {
        /* Make sure to leave this disabled for final versions! */
        //debug!("SNMP authentication config: {:?}", &auth);
        session_builder = session_builder
            .set_auth(auth)
            .map_err(Error::Authentication)?
    }

    if let Some(timing) = &config.host_config.timing {
        debug!("SNMP timing config: {:?}", &timing);
        session_builder = session_builder
            .set_retries(timing.retries)
            .set_timeout(timing.timeout);
    }

    session_builder.open_single().map_err(Error::Connection)
}

/// Retrieve SNMP data from a stored walk.
pub(super) async fn retrieve_data_from_walk(
    _queries: HashMap<String, (Walks, Gets)>,
    _stats: &Mutex<Stats>,
) -> Result<WalkMap> {
    Err(Error::StoredWalkNotImplemented)
}

/// Retrieve SNMP data using simple get requests.
pub(super) async fn retrieve_data_nobulk(
    snmp: &netsnmp::NetSNMP,
    auth: &Option<netsnmp::Auth>,
    config: &Config,
    queries: HashMap<String, (Walks, Gets)>,
    stats: &Mutex<Stats>,
) -> Result<WalkMap> {
    debug!("SNMP: using get/getnext requests");
    let mut session = init_snmp_session(snmp, auth, config).await?;
    let data = Mutex::new(HashMap::new());
    for (context, queries) in queries {
        retrieve_data_nobulk_inner(
            config,
            &mut session,
            &context,
            queries,
            stats,
            &data,
        )
        .await?;
    }
    Ok(data.into_inner())
}

async fn retrieve_data_nobulk_inner(
    config: &Config,
    session: &mut SingleSession,
    context: &str,
    queries: (Walks, Gets),
    stats: &Mutex<Stats>,
    data: &Mutex<WalkMap>,
) -> Result<()> {
    let (mut walks, mut gets) = queries;
    let quirks = &config.host_config.quirks;

    for get in gets.iter_mut() {
        let res = session
            .get_next_with_context_async(&get.oid, Some(context))
            .await;
        let mut data = data.lock();
        match res {
            Ok(Some(var)) => {
                get.save(&var, &mut data);
            }
            Ok(None) => {
                data.insert(get.oid.clone(), Err(WalkError::NoSuchObject));
            }
            Err(netsnmp::Error::Response(_))
                if quirks.invalid_packets_at_end =>
            {
                data.insert(get.oid.clone(), Err(WalkError::NoSuchObject));
            }
            Err(err) => return Err(Error::Query(err)),
        }
    }

    for walk in walks.iter_vars_mut() {
        while !walk.done {
            let res = session
                .get_next_with_context_async(&walk.last, Some(context))
                .await;
            let mut data = data.lock();
            match res {
                Ok(Some(var)) => walk.save(&var, &mut data, quirks),
                Ok(None) => break,
                Err(netsnmp::Error::Response(_))
                    if quirks.invalid_packets_at_end =>
                {
                    break
                }
                Err(err) => return Err(Error::Query(err)),
            }
        }

        if !walk.invalid && walk.retrieved == 0 {
            log::debug!("SNMP: walk {}: checking existence of table", walk.oid);
            let res = session
                .get_with_context_async(&walk.oid, Some(context))
                .await;
            let mut data = data.lock();
            match res {
                Ok(Some(var)) => {
                    walk.save_get(&var, &mut data, quirks);
                }
                Ok(None) => {
                    log::debug!(
                        "SNMP: walk {}: received empty response -> OID does not exist",
                        walk.oid
                    );
                    data.insert(walk.oid.clone(), Err(WalkError::NoSuchObject));
                }
                Err(netsnmp::Error::Response(s))
                    if quirks.invalid_packets_at_end =>
                {
                    log::debug!(
                        "SNMP: walk {}: ignoring error response: {s} -> pretending OID does not exist",
                        walk.oid,
                    );
                    data.insert(walk.oid.clone(), Err(WalkError::NoSuchObject));
                }
                Err(err) => {
                    if let netsnmp::Error::Response(s) = &err {
                        log::debug!(
                                "SNMP: walk {}: received error response: {s} (enable invalid_packets_at_end quirk to ignore)",
                                walk.oid
                            );
                    } else {
                        log::debug!("SNMP: walk {}: {err}", walk.oid);
                    }
                    return Err(Error::Query(err));
                }
            }
        }

        walk.save_stats(&mut stats.lock());
    }

    Ok(())
}

/// Retrieve SNMP data using optimized bulk requests.
pub(super) async fn retrieve_data_bulk(
    snmp: &netsnmp::NetSNMP,
    auth: &Option<netsnmp::Auth>,
    config: &Config,
    queries: HashMap<String, (Walks, Gets)>,
    stats: &Mutex<Stats>,
) -> Result<WalkMap> {
    let opts = &config.host_config.bulk_opts;

    debug!("SNMP: using bulk requests with options: {:?}", opts);

    let data = Mutex::new(HashMap::new());

    for (context, (mut walks, gets)) in queries {
        debug!("SNMP: retrieving data for context '{}'", context);
        walks.split(opts.max_width.min(opts.max_size));

        let walks = Mutex::new(walks);
        let gets = Mutex::new(gets);

        let failed_gets = Mutex::new(Gets::new());
        let failed_walks = Mutex::new(Walks::new());

        futures::stream::iter(0..config.host_config.workers)
            .map(Ok)
            .try_for_each_concurrent(None, {
                // For some odd reason, "move" is needed to avoid
                // borrowing "i" (which is Copy!?). Consequently we
                // need to borrow all the other variables before
                // entering the async block, to avoid moving them...
                |i| {
                    let (context, walks, gets, data, failed_gets, failed_walks) = (
                    	&context,
                    	&walks,
                    	&gets,
                    	&data,
                    	&failed_gets,
                    	&failed_walks,
                    );
                    async move {
                        retrieve_data_bulk_worker(
                            i,
                            snmp,
                            auth,
                            config,
                            stats,
                            context,
                            walks,
                            gets,
                            data,
                            failed_gets,
                            failed_walks,
                        )
                        .await
                    }
                }
            })
            .await?;

        let failed_gets = failed_gets.into_inner();
        let failed_walks = failed_walks.into_inner();

        if !(failed_gets.is_empty() && failed_walks.is_empty()) {
            debug!("SNMP: using get/getnext requests for failed gets / walks");
            retrieve_data_nobulk_inner(
                config,
                &mut init_snmp_session(snmp, auth, config).await?,
                &context,
                (failed_walks, failed_gets),
                stats,
                &data,
            )
            .await?;
        }
    }

    Ok(data.into_inner())
}

async fn retrieve_data_bulk_worker(
    workern: u16,
    snmp: &netsnmp::NetSNMP,
    auth: &Option<netsnmp::Auth>,
    config: &Config,
    stats: &Mutex<Stats>,
    context: &str,
    walks: &Mutex<Walks>,
    gets: &Mutex<Gets>,
    data: &Mutex<WalkMap>,
    failed_gets: &Mutex<Gets>,
    failed_walks: &Mutex<Walks>,
) -> Result<()> {
    debug!("SNMP: starting worker {}", workern);

    let quirks = &config.host_config.quirks;
    let opts = &config.host_config.bulk_opts;
    let mut session = init_snmp_session(snmp, auth, config).await?;

    loop {
        let (mut current_gets, mut current_walks, max_repetitions) = {
            let mut walks = walks.lock();
            let mut gets = gets.lock();

            if walks.is_empty() && gets.is_empty() {
                break;
            }

            debug!(
                "SNMP: worker {}: choosing from {} gets, {} walks",
                workern,
                gets.width(),
                walks.width()
            );

            let current_walks =
                walks.take(opts.max_width, opts.max_size, opts, quirks);
            let max_repetitions = opts.max_repetitions(
                current_walks.max_expected(),
                opts.max_size,
                current_walks.width(),
            );
            let current_gets = match quirks.invalid_packets_at_end
                && !current_walks.is_empty()
            {
                false => {
                    gets.take((opts.max_width - current_walks.width()).min(
                        opts.max_size - max_repetitions * current_walks.width(),
                    ))
                }
                true => Gets::new(),
            };

            (current_gets, current_walks, max_repetitions)
        };

        let get_oids = current_gets.oids();
        let walk_oids = current_walks.oids();
        let expected_size = get_oids.len() + max_repetitions * walk_oids.len();

        if get_oids.is_empty() && (walk_oids.is_empty() || max_repetitions == 0)
        {
            return Err(Error::EmptyQuery);
        }

        if get_oids.len() + walk_oids.len() > opts.max_width
            || max_repetitions > opts.max_length
            || expected_size > opts.max_size
        {
            return Err(Error::InvalidQuery);
        }

        debug!(
            "SNMP: worker {}, get_bulk: {} gets, {}x{} walks ( {} / {} )",
            workern,
            current_gets.width(),
            max_repetitions,
            current_walks.width(),
            get_oids
                .iter()
                .map(Oid::to_string)
                .collect::<Vec<_>>()
                .join(" "),
            walk_oids
                .iter()
                .map(Oid::to_string)
                .collect::<Vec<_>>()
                .join(" ")
        );

        match session
            .get_bulk_with_context_async(
                &get_oids,
                &walk_oids,
                max_repetitions,
                Some(context),
            )
            .await
        {
            Ok(pdu) => {
                let mut data = data.lock();
                let mut vars = pdu.variables().peekable();
                let mut i = 0;
                for (get, var) in current_gets.iter_mut().zip(&mut vars) {
                    get.save(var, &mut data);
                    i += 1;
                }
                while vars.peek().is_some() {
                    for (walk, var) in
                        current_walks.iter_vars_mut().zip(&mut vars)
                    {
                        walk.save(var, &mut data, quirks);
                        i += 1;
                    }
                }

                if i == 0 {
                    debug!(
                        "SNMP: worker {}: Received an empty response",
                        workern
                    );
                    return Err(Error::EmptyResponse);
                } else if i == expected_size {
                    debug!(
                        "SNMP: worker {}: Retrieved {} vars \
						 (as expected)",
                        workern, i
                    );
                } else if i < expected_size {
                    debug!(
                        "SNMP: worker {}: Retrieved {} vars ({} less \
						 than expected; decrease max_size?)",
                        workern,
                        i,
                        expected_size - i
                    );
                } else {
                    debug!(
                        "SNMP: worker {}: Retrieved {} vars ({} more \
						 than expected!?)",
                        workern,
                        i,
                        i - expected_size
                    );
                }
            }

            Err(err @ netsnmp::Error::Response(_))
                if quirks.invalid_packets_at_end =>
            {
                debug!("SNMP: worker {}: Received error: {}; saving current get/walks to be retrieved using non-bulk requests", workern, err);
                failed_gets.lock().extend(current_gets);
                failed_walks.lock().extend(current_walks);
                current_gets = Gets::new();
                current_walks = Walks::new();
            }

            Err(err) => {
                debug!("SNMP: worker {}: Received error: {}", workern, err);
                return Err(Error::Query(err)); // Fatal!

                /* Translate to walk-specific warning / error or bail out.
                 * Warning is optional. If a warning is returned, it is used
                 * in case the variable already has some data. */
                /*let (warn,err) = match err {
                    /*netsnmp::Error::Response(e) if e == "Timeout" => (
                    Some(WalkWarning::Timeout),
                    WalkError::Timeout
                    ),*/
                    err => return Err(Error::Query(err).into()) // Fatal!
                };*/

                /*for get in current_gets.iter_mut() {
                    get.done = true;
                    match data.entry(get.oid.clone()) {
                    Entry::Vacant(ent) => {
                        ent.insert(Err(err.clone()));
                    },
                    Entry::Occupied(mut ent) => match (&warn, ent.get_mut()) {
                        (Some(warn), Ok((_,warnings))) => {
                        warnings.push((Verbosity::Warning, warn.clone()));
                        },
                        (None, Ok(_)) => {
                        /* Yeah, this returns a Result<_> :p */
                        let _ = ent.insert(Err(err.clone()));
                        },
                        (_, Err(_)) => {},
                    }
                    }
                }

                for walk in current_walks.iter_vars_mut() {
                    walk.done = true;
                    walk.invalid = true;
                    match data.entry(walk.oid.clone()) {
                    Entry::Vacant(ent) => {
                        ent.insert(Err(err.clone()));
                    },
                    Entry::Occupied(mut ent) => match (&warn, ent.get_mut()) {
                        (Some(warn), Ok((_,warnings))) => {
                        warnings.push((Verbosity::Warning, warn.clone()));
                        },
                        (None, Ok(_)) => {
                        let _ = ent.insert(Err(err.clone()));
                        },
                        (_, Err(_)) => {},
                    }
                    }
                }*/
            }
        }

        for walk in current_walks.iter_vars_mut() {
            if config.host_config.quirks.refresh_session {
                session = init_snmp_session(snmp, auth, config).await?;
            }

            if walk.done && !walk.invalid && walk.retrieved == 0 {
                log::debug!(
                    "SNMP: walk {}: checking existence of table",
                    walk.oid
                );
                let res = session
                    .get_with_context_async(&walk.oid, Some(context))
                    .await;
                let mut data = data.lock();
                match res {
                    Ok(Some(var)) => {
                        walk.save_get(&var, &mut data, quirks);
                    }
                    Ok(None) => {
                        log::debug!(
                            "SNMP: walk {}: received empty response -> OID does not exist",
                            walk.oid
                        );
                        data.insert(
                            walk.oid.clone(),
                            Err(WalkError::NoSuchObject),
                        );
                    }
                    Err(netsnmp::Error::Response(s))
                        if quirks.invalid_packets_at_end =>
                    {
                        log::debug!(
                            "SNMP: walk {}: ignoring error response: {s} -> pretending OID does not exist",
                            walk.oid
                        );
                        data.insert(
                            walk.oid.clone(),
                            Err(WalkError::NoSuchObject),
                        );
                    }
                    Err(err) => {
                        if let netsnmp::Error::Response(s) = &err {
                            log::debug!(
                                "SNMP: walk {}: received error response: {s} (enable invalid_packets_at_end quirk to ignore)",
                                walk.oid
                            );
                        } else {
                            log::debug!("SNMP: walk {}: {err}", walk.oid);
                        }
                        return Err(Error::Query(err));
                    }
                }
            }
        }

        gets.lock().reinject(current_gets);
        walks.lock().reinject(current_walks, &mut stats.lock());
    }

    Ok(())
}

// /// Check if a table column exists at Oid.
// async fn check_existence(
//     session: &mut SingleSession,
//     context: &str,
//     oid: &Oid,
// ) -> Result<bool> {
//     debug!("SNMP: get: {}", oid);
//     match session.get_with_context_async(oid, Some(context)).await {
//         Ok(Some(var)) => match var.get_value() {
//             Err(netsnmp::ErrType::NoSuchInstance) => {
//                 debug!(
//                     "SNMP: Received \"no such instance\" \
// 				       --> OID exists"
//                 );
//                 Ok(true)
//             }
//             val => {
//                 debug!(
//                     "SNMP: Received value \"{:?}\" \
// 				      --> OID does not exist",
//                     val
//                 );
//                 Ok(false)
//             }
//         },
//         Ok(None) => {
//             debug!(
//                 "SNMP: Received an empty response\
// 				  --> OID does not exist"
//             );
//             Ok(false)
//         }
//         Err(err) => {
//             debug!("SNMP: Query error: {}", err);
//             Err(Error::Query(err))
//         }
//     }
// }

/// Calculate the list of queries to be made.
pub(super) fn get_queries(
    input: &Input,
    config: &Config,
    query_map: &ProtoQueryMap,
    stats: &mut Stats,
) -> Result<HashMap<String, (Walks, Gets)>> {
    let (session_context, context_rules) = match &config.host_config.auth {
        Some(netsnmp::Auth::V3(netsnmp::V3Auth { context, .. })) => (
            context.as_deref().unwrap_or(DEFAULT_CONTEXT).to_string(),
            config.host_config.snmpv3_contexts.as_slice(),
        ),
        _ => (String::from(DEFAULT_CONTEXT), &[][..]),
    };

    let mut walks = HashMap::new();
    let mut gets = HashMap::new();

    for (table_id, field_ids) in query_map {
        match ObjectId::from_table_id(table_id, input)? {
            None => {
                for field_id in field_ids {
                    let obj = ObjectId::from_field_id(field_id, input)?
                        .try_get_from(&input.objects)?;
                    for context in get_contexts(
                        &obj.oid,
                        &obj.context_group,
                        &session_context,
                        context_rules,
                    ) {
                        gets.entry(context)
                            .or_insert_with(Gets::new)
                            .push(obj.oid.clone());
                    }
                }
            }

            Some(obj_id) => {
                let obj = obj_id.try_get_from(&input.objects)?;
                let entry = obj_id.try_get_from(&input.tables)?;
                let contexts = get_contexts(
                    &obj.oid,
                    &obj.context_group,
                    &session_context,
                    context_rules,
                );
                let index = entry.get_index(input)?;
                match entry.fold.is_some() {
                    true => {
                        let table_oid =
                            &obj_id.try_get_from(&input.objects)?.oid;
                        for context in &contexts {
                            add_walk(
                                walks
                                    .entry(context.to_string())
                                    .or_insert_with(HashMap::new),
                                table_oid,
                                &index,
                                stats,
                            );
                        }
                    }
                    false => {
                        for field_id in field_ids {
                            let obj_id =
                                ObjectId::from_field_id(field_id, input)?;
                            if !index.contains(&obj_id) {
                                let field_oid =
                                    &obj_id.try_get_from(&input.objects)?.oid;
                                for context in &contexts {
                                    add_walk(
                                        walks
                                            .entry(context.to_string())
                                            .or_insert_with(HashMap::new),
                                        field_oid,
                                        &index,
                                        stats,
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    let contexts: HashSet<_> =
        gets.keys().chain(walks.keys()).cloned().collect();

    Ok(contexts
        .into_iter()
        .map(|context| {
            (
                context.clone(),
                (
                    walks.remove(&context).map_or_else(Walks::new, |walks| {
                        Walks::from(walks.into_values().collect())
                    }),
                    gets.remove(&context).unwrap_or_else(Gets::new),
                ),
            )
        })
        .collect())
}

const DEFAULT_CONTEXT: &str = "";

/// Find a list of contexts to use for the OID, based on the configured rules.
fn get_contexts(
    oid: &Oid,
    group: &Option<String>,
    session_context: &String,
    rules: &[(ContextSelector, HashSet<Option<String>>)],
) -> HashSet<String> {
    rules
        .iter()
        .find_map(|(sel, val)| match sel {
            ContextSelector::All => Some(val),
            ContextSelector::Group(sel_group) => group
                .as_ref()
                .map_or(false, |group| group == sel_group)
                .then_some(val),
            ContextSelector::Oid(sel_oid) => (sel_oid == oid).then_some(val),
        })
        .map_or_else(
            || std::iter::once(session_context.to_string()).collect(),
            |contexts| {
                contexts
                    .iter()
                    .map(|context| {
                        context
                            .as_deref()
                            .unwrap_or(session_context.as_str())
                            .to_string()
                    })
                    .collect()
            },
        )
}

fn add_walk(
    walks: &mut HashMap<u64, WalkTable>,
    oid: &Oid,
    index: &Index,
    stats: &Stats,
) {
    let (index_hash, expected) = match stats.get_walk(oid) {
        Some(s) => (
            s.index,
            Some(s.length.estimate_quantile(0.99).ceil() as usize + 1),
        ),
        None => {
            let mut hasher = DefaultHasher::new();
            index.hash(&mut hasher);
            (hasher.finish(), None)
        }
    };

    walks
        .entry(index_hash)
        .or_insert_with(WalkTable::new)
        .push(WalkVar::new(oid.clone()), expected);
}

/// Build a DataMap from an SNMPDataMap, converting field data, warnings and errors.
pub(super) fn build_tables(
    input: &Input,
    query: &ProtoQueryMap,
    data: WalkMap,
    counters: &mut Counters,
) -> Result<DataMap> {
    let mut result: DataMap = HashMap::new();

    for (data_table_id, data_field_ids) in query {
        /* Find table index. */

        let data_table = ObjectId::from_table_id(data_table_id, input)?;
        let entry = data_table
            .as_ref()
            .map_or::<Result<_>, _>(Ok(None), |obj_id| {
                Ok(Some(obj_id.try_get_from(&input.tables)?))
            })?;
        let index = entry.map_or_else(
            || Ok(Index::empty()),
            |entry| entry.get_index(input),
        )?;
        let fold = entry.and_then(|entry| entry.fold);

        let mut table_data: Vec<ProtoRow> = Vec::new();
        let mut errors = HashMap::new();
        let mut warnings = HashMap::new();

        match (&data_table, fold) {
            (Some(obj_id), Some(fold)) => {
                let table_obj = obj_id.try_get_from(&input.objects)?;

                match data.get(&table_obj.oid) {
                    Some(Ok(Annotated {
                        value: rows,
                        warnings: warn,
                    })) => {
                        let mut data = HashMap::new();

                        for (oid, val) in rows {
                            let i = match oid.as_slice().first() {
                                Some(i) => *i - 1,
                                None => continue,
                            };
                            data.entry(i / fold + 1)
                                .or_insert_with(HashMap::new)
                                .insert(i % fold + 1, val.clone());
                        }

                        for (i, row) in data {
                            table_data.push(
                                data_field_ids
                                    .iter()
                                    .map(|data_field_id| {
                                        let obj_id = ObjectId::from_field_id(
                                            data_field_id,
                                            input,
                                        )?;
                                        let field_obj = obj_id
                                            .try_get_from(&input.objects)?;

                                        Ok((
                                            data_field_id.clone(),
                                            match field_obj
                                                .oid
                                                .in_table(&table_obj.oid)
                                                .as_slice()
                                                .first()
                                            {
                                                None => {
                                                    Ok(value::Value::Integer(
                                                        i as i64,
                                                    ))
                                                }
                                                Some(c) => {
                                                    match row.get(c) {
                                                        Some(val) => {
                                                            let scalar = obj_id
																.try_get_from(&input.scalars)?;
                                                            scalar.get_value(
                                                                val.as_ref(),
                                                                &obj_id,
                                                                &Oid::from_vec(vec![i]),
                                                                counters,
                                                            ).unwrap_or(Err(
																DataError::Missing,
															))
                                                        }
                                                        None => Err(
                                                            DataError::Missing,
                                                        ),
                                                    }
                                                }
                                            },
                                        ))
                                    })
                                    .collect::<Result<_>>()?,
                            )
                        }

                        for Warning { message, .. } in warn {
                            warnings
                                .entry(message.clone())
                                .or_insert_with(HashSet::new)
                                .insert(table_obj.oid.clone());
                        }
                    }
                    Some(Err(err)) => {
                        errors
                            .entry(err.clone())
                            .or_insert_with(HashSet::new)
                            .insert(table_obj.oid.clone());
                    }
                    None => {
                        return Err(Error::NotRequested(table_obj.oid.clone()));
                    }
                }
            }
            _ => {
                /* Get all populated indices for the requested fields. */

                let mut index_cols = HashMap::new();
                let mut found_cols = HashMap::new();
                let mut err_cols = HashSet::new();

                for data_field_id in data_field_ids {
                    let obj_id = ObjectId::from_field_id(data_field_id, input)?;

                    if index.contains(&obj_id) {
                        index_cols
                            .insert(data_field_id.clone(), obj_id.clone());
                        continue;
                    }

                    let object = &obj_id.try_get_from(&input.objects)?;

                    match data.get(&object.oid) {
                        Some(Ok(Annotated {
                            value: rows,
                            warnings: warn,
                        })) => {
                            found_cols.insert(data_field_id.clone(), rows);
                            for Warning { message, .. } in warn {
                                warnings
                                    .entry(message.clone())
                                    .or_insert_with(HashSet::new)
                                    .insert(object.oid.clone());
                            }
                        }
                        Some(Err(err)) => {
                            err_cols.insert(data_field_id.clone());
                            errors
                                .entry(err.clone())
                                .or_insert_with(HashSet::new)
                                .insert(object.oid.clone());
                        }
                        None => {
                            return Err(Error::NotRequested(
                                object.oid.clone(),
                            ));
                        }
                    }
                }

                /* Verify if at least one non-index column was requested
                 * and all requested columns failed. */
                if found_cols.is_empty() && !errors.is_empty() {
                    result.insert(
                        data_table_id.clone(),
                        Err(DTError::WalkErrs(errors)),
                    );
                    continue;
                }

                /* Find all index vals. */

                let mut row_ids = HashSet::new();

                for rows in found_cols.values() {
                    row_ids.extend(rows.keys())
                }

                let mut sorted_row_ids: Vec<_> = row_ids.into_iter().collect();
                sorted_row_ids.sort();

                /* Retrieve values. */

                for row_id in &sorted_row_ids {
                    let mut row = HashMap::new();
                    let mut idx_vals = index.get_values(row_id, input)?;

                    for (data_field_id, object_id) in &index_cols {
                        row.insert(
                            data_field_id.clone(),
                            match idx_vals.remove(object_id) {
                                Some(val) => val,
                                None => Err(DataError::Missing),
                            },
                        );
                    }

                    for (data_field_id, rows) in &found_cols {
                        let obj_id =
                            ObjectId::from_field_id(data_field_id, input)?;
                        let scalar = obj_id.try_get_from(&input.scalars)?;

                        row.insert(
                            data_field_id.clone(),
                            match rows.get(row_id).and_then(|val| {
                                scalar.get_value(
                                    val.as_ref(),
                                    &obj_id,
                                    row_id,
                                    counters,
                                )
                            }) {
                                Some(val) => val,
                                None => Err(DataError::Missing),
                            },
                        );
                    }

                    for data_field_id in &err_cols {
                        row.insert(
                            data_field_id.clone(),
                            Err(DataError::Missing),
                        );
                    }

                    table_data.push(row);
                }
            }
        }

        result.insert(
            data_table_id.clone(),
            Ok(Annotated {
                value: table_data,
                warnings: match errors.is_empty() && warnings.is_empty() {
                    false => vec![Warning {
                        verbosity: Verbosity::Warning,
                        message: DTWarning::WalkErrs(errors, warnings),
                    }],
                    true => Vec::new(),
                },
            }),
        );
    }

    Ok(result)
}
