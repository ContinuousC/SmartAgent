/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::collections::HashMap;
use std::fmt::{Display, Write};
use std::hash::Hash;
#[cfg(feature = "trust-dns-resolver")]
use std::net::IpAddr;

use serde::de::DeserializeOwned;
use serde::Serialize;

#[cfg(feature = "trust-dns-resolver")]
use trust_dns_resolver::{AsyncResolver, Resolver};

use super::error::{Error, Result};
pub use agent_derive::{Key, NamedObj};

/* Named objects and ids. */

/// An object with a type name.
pub trait NamedObj {
    const NAME: &'static str;
}

/// A unique id in one sense or another.
pub trait Key:
    Serialize + DeserializeOwned + Display + Clone + Eq + Hash
{
}

/// A unique id for a specific type of object.
pub trait KeyFor<T>: Key {}

/* "Try get" convenience methods. */

/// Add a convenience function HashMaps indexes by an id
/// field to add error handling to a lookup that should.
/// always be successfull. It will return a suitable Err
/// variant (which can be handled using the question mark
/// notation) when this is not the case.
pub trait TryGet<K, V> {
    fn try_get(&self, key: &K) -> Result<&V>;
    fn try_get_mut(&mut self, key: &K) -> Result<&mut V>;
}

pub trait TryGetFrom<C, V> {
    fn try_get_from<'a>(&self, map: &'a C) -> Result<&'a V>;
}

impl<K, V> TryGet<K, V> for HashMap<K, V>
where
    K: Key + Display,
{
    fn try_get(&self, key: &K) -> Result<&V> {
        self.get(key)
            .ok_or_else(|| Error::MissingObject(format!("{}", key)))
    }
    fn try_get_mut(&mut self, key: &K) -> Result<&mut V> {
        self.get_mut(key)
            .ok_or_else(|| Error::MissingObject(format!("{}", key)))
    }
}

impl<K, V> TryGetFrom<HashMap<K, V>, V> for K
where
    K: Key + Display,
{
    fn try_get_from<'a>(&self, map: &'a HashMap<K, V>) -> Result<&'a V> {
        map.try_get(self)
    }
}

/* Fallible in-place semigroup traits. */

/// In-place semigroup trait with error handling.
pub trait TryAppend {
    fn try_append(&mut self, other: Self) -> Result<()>;
}

/// In-place semigroup trait with error handling.
pub trait TryAppendState {
    type State;
    fn try_append_state(
        &mut self,
        other: Self,
        state: &Self::State,
    ) -> Result<()>;
}

pub fn quote_filename(name: &str) -> String {
    let mut r = String::new();
    name.bytes()
        .try_for_each(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b' ' | b'_' | b'-' => {
                write!(r, "{}", b as char)
            }
            _ => write!(r, ":{b:2x}"),
        })
        .unwrap();
    r
}

pub fn unquote_filename(name: &str) -> Option<String> {
    let mut r = Vec::new();
    let mut s = name.bytes();
    while let Some(b) = s.next() {
        match b {
            b':' => {
                if let Some(b0) = s.next() {
                    if let (Some(v0), Some(b1)) =
                        (hex_digit_value(b0), s.next())
                    {
                        if let Some(v1) = hex_digit_value(b1) {
                            r.push(v0 << 4 | v1);
                        } else {
                            r.push(b':');
                            r.push(b0);
                            r.push(b1);
                        }
                    } else {
                        r.push(b':');
                        r.push(b0);
                    }
                } else {
                    r.push(b':');
                }
            }
            _ => r.push(b),
        }
    }
    String::from_utf8(r).ok()
}

fn hex_digit_value(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'A'..=b'F' => Some(b - b'A' + 10),
        b'a'..=b'f' => Some(b - b'a' + 10),
        _ => None,
    }
}

#[cfg(feature = "trust-dns-resolver")]
pub async fn ip_lookup(hostname: &str) -> Result<Vec<IpAddr>> {
    Ok(AsyncResolver::tokio_from_system_conf()
        .map_err(Error::Resolve)?
        .lookup_ip(hostname)
        .await
        .map_err(Error::Resolve)?
        .iter()
        .collect())
}

#[cfg(feature = "trust-dns-resolver")]
pub async fn ip_lookup_one(hostname: &str) -> Result<IpAddr> {
    AsyncResolver::tokio_from_system_conf()
        .map_err(Error::Resolve)?
        .lookup_ip(hostname)
        .await
        .map_err(Error::Resolve)?
        .iter()
        .next()
        .ok_or(Error::ResolveMissing)
}

#[cfg(feature = "trust-dns-resolver")]
pub fn ip_lookup_sync(hostname: &str) -> Result<Vec<IpAddr>> {
    Ok(Resolver::from_system_conf()
        .map_err(Error::ResolveIo)?
        .lookup_ip(hostname)
        .map_err(Error::Resolve)?
        .iter()
        .collect())
}

#[cfg(feature = "trust-dns-resolver")]
pub fn ip_lookup_one_sync(hostname: &str) -> Result<IpAddr> {
    Resolver::from_system_conf()
        .map_err(Error::ResolveIo)?
        .lookup_ip(hostname)
        .map_err(Error::Resolve)?
        .iter()
        .next()
        .ok_or(Error::ResolveMissing)
}
