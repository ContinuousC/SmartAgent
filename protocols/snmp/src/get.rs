/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::collections::HashMap;
use std::mem;

use netsnmp::Oid;

use etc_base::Annotated;
use log::debug;

use super::error::WalkError;
use super::query::WalkMap;

pub struct Gets(Vec<GetVar>);
pub struct GetVar {
    pub oid: Oid,
    pub done: bool,
}

impl Gets {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    /// Add a table to the list of walks.
    pub fn push(&mut self, oid: Oid) {
        self.0.push(GetVar::new(oid))
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn width(&self) -> usize {
        self.0.len()
    }

    pub fn oids(&self) -> Vec<Oid> {
        self.0.iter().map(|v| v.oid.clone()).collect()
    }

    /// Fill available_width with get requests.
    pub fn take(&mut self, available_width: usize) -> Gets {
        let width = available_width.min(self.0.len());
        let rest = self.0.split_off(width);
        Gets(mem::replace(&mut self.0, rest))
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut GetVar> {
        self.0.iter_mut()
    }

    /*pub fn into_iter(self) -> impl Iterator<Item = GetVar> {
    self.0.into_iter()
    }*/

    pub fn reinject(&mut self, gets: Gets) {
        self.0.extend(gets.0.into_iter().filter(|v| !v.done))
    }

    pub fn extend(&mut self, gets: Gets) {
        self.0.extend(gets.0)
    }
}

impl GetVar {
    pub fn new(oid: Oid) -> Self {
        GetVar { oid, done: false }
    }

    /// Save get data.
    pub fn save(&mut self, var: &netsnmp::VariablePtr, data: &mut WalkMap) {
        self.done = true;
        if self.oid.contains(&var.get_name()) {
            debug!("SNMP: get {}: saving {}", &self.oid, var.get_name());
            if let Ok(Annotated { value: table, .. }) =
                data.entry(self.oid.clone()).or_insert_with(|| {
                    Ok(Annotated {
                        value: HashMap::new(),
                        warnings: Vec::new(),
                    })
                })
            {
                table.insert(Oid::empty(), var.get_value());
            }
        } else {
            debug!("SNMP: get {}: no such object", &self.oid);
            data.insert(self.oid.clone(), Err(WalkError::NoSuchObject));
        }
    }
}
