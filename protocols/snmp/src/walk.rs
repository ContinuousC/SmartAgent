/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::collections::hash_map::{DefaultHasher, Entry};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::iter::FromIterator;
use std::mem;

use netsnmp::Oid;
use serde::{Deserialize, Serialize};
use tdigest::TDigest;

use etc_base::{Annotated, Warning};
use log::debug;
use logger::Verbosity;

use crate::config::Quirks;

use super::config::BulkConfig;
use super::error::{WalkError, WalkWarning};
use super::query::WalkMap;
use super::stats::Stats;

pub struct Walks(Vec<WalkTable>);

pub struct WalkTable {
    vars: Vec<WalkVar>,
    expected: Option<usize>,
}

pub struct WalkVar {
    pub oid: Oid,
    pub done: bool,
    pub invalid: bool,
    pub last: Oid,
    pub index: DefaultHasher,
    pub retrieved: usize,
}

#[derive(Serialize, Deserialize)]
pub struct WalkStats {
    pub length: TDigest,
    pub index: u64,
}

impl Walks {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn from(tables: Vec<WalkTable>) -> Self {
        Self(tables)
    }

    /// Add a table to the list of walks.
    pub fn push(&mut self, table: WalkTable) {
        self.0.push(table)
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn width(&self) -> usize {
        self.0.iter().map(|w| w.vars.len()).sum()
    }

    pub fn oids(&self) -> Vec<Oid> {
        self.0
            .iter()
            .flat_map(|w| w.vars.iter())
            .map(|v| v.last.clone())
            .collect()
    }

    pub fn max_expected(&self) -> Option<usize> {
        self.0.iter().fold(None, |e, w| match (e, w.expected) {
            (Some(a), Some(b)) => Some(
                a.max((b as i64 - w.max_retrieved() as i64).max(0) as usize),
            ),
            (None, Some(b)) => {
                Some((b as i64 - w.max_retrieved() as i64).max(0) as usize)
            }
            (a, _) => a,
        })
    }

    pub fn iter_vars_mut(&mut self) -> impl Iterator<Item = &mut WalkVar> {
        self.0.iter_mut().flat_map(|w| w.vars.iter_mut())
    }

    /// Split walk tables longer than max_width into smaller walk tables.
    pub fn split(&mut self, max_width: usize) {
        for mut walk in std::mem::take(&mut self.0).into_iter() {
            while walk.vars.len() > max_width {
                let new_vars = walk.vars.split_off(max_width);
                let chunk = mem::replace(&mut walk.vars, new_vars);
                self.push(WalkTable {
                    vars: chunk,
                    expected: walk.expected,
                });
            }
            self.push(walk)
        }
    }

    /// Fill at most max_width vars with walk tables of expected length max(expected_length) - max_len_diff.
    pub fn take(
        &mut self,
        available_width: usize,
        available_size: usize,
        opts: &BulkConfig,
        quirks: &Quirks,
    ) -> Walks {
        //let chunk = |e : Option<usize>| e.unwrap_or(opts.def_length).min(opts.max_length);
        //let min_expected = (chunk(self.max_expected()) as i64 - opts.max_len_diff as i64).max(0) as usize;

        let mut current_walks = Self::new();
        let mut max_expected = None;
        let mut min_expected = None;
        let mut width = 0;

        //let (long_walks,short_walks) : (Vec<_>,Vec<_>) = mem::replace(&mut self.0, Vec::new()).into_iter()
        //.partition(|w| chunk(w.expected) >= min_expected);

        for walk in std::mem::take(&mut self.0).into_iter() {
            let new_width = width + walk.vars.len();
            let new_max_expected = max_expected.max(walk.expected);
            let new_min_expected = min_expected.min(walk.expected);
            let new_length = opts.max_repetitions(
                new_max_expected,
                available_size,
                new_width,
            );

            match new_width <= available_width
                && new_min_expected
                    .map_or(true, |e| new_length <= e + opts.max_len_diff)
                && (!quirks.invalid_packets_at_end
                    || current_walks.0.is_empty())
            {
                true => {
                    current_walks.push(walk);
                    width = new_width;
                    min_expected = new_min_expected;
                    max_expected = new_max_expected;
                }
                false => self.push(walk),
            }
        }

        current_walks
    }

    /*/// Take the longest walk from the list.
    pub fn take_longest(&mut self) -> Walks {

    let mut longest = None;
    let mut maximum = None;

    for (i,walk) in self.0.iter().enumerate() {
        if let Some(expected) = walk.expected {
        if maximum.map_or(true, |max| expected > max) {
            maximum = Some(expected);
            longest = Some(i);
        }
        }
    }

    let walk = match longest {
        None => self.0.pop(),
        Some(i) => Some(self.0.remove(i))
    };

    match walk {
        Some(w) => Walks(vec![w]),
        None => Walks(vec![])
    }

    }*/

    /// Add walked vars that did not reach the end
    pub fn reinject(&mut self, walks: Walks, stats: &mut Stats) {
        for mut table in walks.0.into_iter() {
            for walk in std::mem::take(&mut table.vars).into_iter() {
                match walk.done {
                    true => walk.save_stats(stats),
                    false => table.vars.push(walk),
                }
            }

            if !table.vars.is_empty() {
                self.push(table);
            }
        }
    }

    pub fn extend(&mut self, walks: Walks) {
        self.0.extend(walks.0);
    }
}

impl WalkTable {
    pub fn new() -> Self {
        Self {
            vars: Vec::new(),
            expected: None,
        }
    }

    pub fn push(&mut self, var: WalkVar, expected: Option<usize>) {
        self.vars.push(var);
        self.expected = match (self.expected, expected) {
            (Some(a), Some(b)) => Some(a.max(b)),
            (Some(a), None) => Some(a),
            (_, b) => b,
        };
    }

    pub fn max_retrieved(&self) -> usize {
        self.vars.iter().fold(0, |m, v| v.retrieved.max(m))
    }
}

impl WalkVar {
    pub fn new(oid: Oid) -> Self {
        Self {
            oid: oid.clone(),
            last: oid,
            done: false,
            invalid: false,
            retrieved: 0,
            index: DefaultHasher::new(),
        }
    }

    /// Save walk data.
    pub fn save(
        &mut self,
        var: &netsnmp::VariablePtr,
        data: &mut WalkMap,
        quirks: &Quirks,
    ) {
        if !self.done {
            let oid = var.get_name();
            let val = var.get_value();

            if !self.oid.contains(&oid) {
                debug!(
                    "SNMP: walk {}: done (got oid past table: {})",
                    &self.oid, &oid
                );
                self.done = true;
            } else if let Err(netsnmp::ErrType::EndOfMibView) = val {
                debug!("SNMP: walk {}: done (end of mib view)", &self.oid);
                self.done = true;
            } else if oid <= self.last && !quirks.ignore_oids_not_increasing {
                debug!("SNMP: walk {}: done (oids not increasing)", &self.oid);
                self.done = true;
                match data.entry(self.oid.clone()) {
                    Entry::Vacant(ent) => {
                        ent.insert(Err(WalkError::OIDsNotIncreasing));
                    }
                    Entry::Occupied(mut ent) => match ent.get_mut() {
                        Ok(Annotated { warnings, .. }) => {
                            warnings.push(Warning {
                                verbosity: Verbosity::Warning,
                                message: WalkWarning::OIDsNotIncreasing,
                            });
                        }
                        Err(_) => {}
                    },
                }
            } else {
                debug!("SNMP: walk {}: saving {}", &self.oid, &oid);
                let index = oid.in_table(&self.oid);
                if let Ok(Annotated { value: table, .. }) =
                    data.entry(self.oid.clone()).or_insert_with(|| {
                        Ok(Annotated {
                            value: HashMap::new(),
                            warnings: Vec::new(),
                        })
                    })
                {
                    table.insert(index.clone(), val);
                }
                index.hash(&mut self.index);
                self.last = oid;
                self.retrieved += 1;
            }
        }
    }

    /// Save walk data for get query if no data was received from
    /// getnext queries.
    pub fn save_get(
        &mut self,
        var: &netsnmp::VariablePtr,
        data: &mut WalkMap,
        _quirks: &Quirks,
    ) {
        let oid = var.get_name();
        let val = var.get_value();

        let res = match val {
            Ok(val) if self.oid == oid => {
                debug!("SNMP: walk {}: saving single value from get", &self.oid);
				let index = oid.in_table(&self.oid);
                index.hash(&mut self.index);
				self.last = oid;
				self.retrieved += 1;
				Ok(Annotated {
                    value: HashMap::from_iter([(index, Ok(val))]),
                    warnings: Vec::new(),
                })
            }
            Err(netsnmp::ErrType::NoSuchInstance) => {
                debug!(
                    "SNMP: walk {}: got \"no such instance\" -> OID exists",
                    &self.oid
                );
                Ok(Annotated {
                    value: HashMap::new(),
                    warnings: Vec::new(),
                })
            }
			Ok(_) /* if self.oid != oid */ => {
                debug!(
                    "SNMP: walk {}: got value for another oid {oid} -> OID does not exist",
                    &self.oid
                );
                Err(WalkError::NoSuchObject)
			}
            Err(e) => {
                debug!(
                    "SNMP: walk {}: got error \"{e:?} -> OID does not exist",
                    &self.oid
                );
                Err(WalkError::NoSuchObject)
            }
		};

        data.insert(self.oid.clone(), res);
    }

    /// Save walk statistics.
    pub(crate) fn save_stats(&self, stats: &mut Stats) {
        stats
            .walk_entry(self.oid.clone())
            .and_modify(|s| s.update(self.retrieved, self.index.finish()))
            .or_insert_with(|| {
                WalkStats::from(self.retrieved, self.index.finish())
            });
    }
}

impl WalkStats {
    fn from(length: usize, index: u64) -> Self {
        Self {
            index,
            length: TDigest::new_with_size(10)
                .merge_sorted(vec![length as f64]),
        }
    }

    fn update(&mut self, length: usize, index: u64) {
        self.length = self.length.merge_sorted(vec![length as f64]);
        self.index = index;
    }
}
