/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use super::error::{Error, Result};
use super::utils::{Key, KeyFor, NamedObj, TryAppend};
pub use agent_derive::DBObj;

/// An object loaded from the database.
pub trait DBObj: NamedObj + Serialize + DeserializeOwned {}

/// An id for a database object.
#[derive(Serialize, Deserialize, Debug)]
#[serde(transparent)]
pub struct DBId<T: DBObj>(pub String, PhantomData<T>);

impl<T: DBObj> Key for DBId<T> {}
impl<T: DBObj> KeyFor<T> for DBId<T> {}

impl<T: DBObj> DBId<T> {
    pub fn from_raw(id: String) -> Self {
        Self(id, PhantomData)
    }
}

/* We need to implement these traits ourselves because
the auto-derived ones add dependencies on T. */

impl<T: DBObj> Hash for DBId<T> {
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        self.0.hash(hasher)
    }
}

impl<T: DBObj> Clone for DBId<T> {
    fn clone(&self) -> Self {
        DBId(self.0.clone(), PhantomData)
    }
}

impl<T: DBObj> PartialEq for DBId<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<T: DBObj> Eq for DBId<T> {}

impl<T: DBObj> PartialOrd for DBId<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<T: DBObj> Ord for DBId<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}

/* Display implementations. */

impl<T: DBObj> fmt::Display for DBId<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}Id {}", T::NAME, self.0)
    }
}

/* TryAppend implementation checking that double objects are the same. */

impl<K: Key, V: PartialEq> TryAppend for HashMap<K, V> {
    fn try_append(&mut self, other: Self) -> Result<()> {
        for (key, val) in other.into_iter() {
            match self.entry(key.clone()) {
                Entry::Vacant(ent) => {
                    ent.insert(val);
                    Ok(())
                }
                Entry::Occupied(ent) => {
                    if *ent.get() == val {
                        Ok(())
                    } else {
                        Err(Error::IncompatibleDefinitions(format!("{}", key)))
                    }
                }
            }?
        }

        Ok(())
    }
}
