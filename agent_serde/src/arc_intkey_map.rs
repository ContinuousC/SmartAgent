/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::sync::Arc;

use serde::{de, ser};
use std::collections::BTreeMap;

use super::intkey_map;

pub fn serialize<S>(
    map: &Arc<BTreeMap<i64, String>>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: ser::Serializer,
{
    intkey_map::serialize(map.as_ref(), serializer)
}

pub fn deserialize<'de, D>(
    deserializer: D,
) -> Result<Arc<BTreeMap<i64, String>>, D::Error>
where
    D: de::Deserializer<'de>,
{
    Ok(Arc::new(deserializer.deserialize_map(intkey_map::Visitor)?))
}
