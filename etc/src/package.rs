/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::collections::HashMap;

use serde::{
    de::{Error, IgnoredAny, MapAccess, Visitor},
    Deserialize, Serialize,
};
use serde_json::value::RawValue;

use etc_base::Protocol;

use super::etc::Etc;

/// On-disk representation of EventTypeCatalog definitions.
/// It contains the subset needed to run the agent for one
/// or more Monitoring Packs.
#[derive(Serialize, Clone, Debug)]
pub struct Package {
    /// The input contains protocol-specific parameters,
    /// to be decoded by the protocol plugins.
    #[serde(rename = "Input")]
    pub input: HashMap<Protocol, Box<RawValue>>,

    /* No longer loaded from package, but generated using plugin's
     * self-description API. */
    // pub data_tables: HashMap<DataTableId, DataTableSpec>,
    // pub data_fields: HashMap<DataFieldId, DataFieldSpec>,
    // pub data_table_fields: HashMap<DataTableId, HashSet<DataFieldId>>,
    /// Etc Objects.
    #[serde(flatten)]
    pub etc: Etc,
}

/* Manual Deserialize implementation to get correct linenumbers on
 * errors from flattened Etc deserialization. This can be switched
 * back to auto-derived when serde solves this issue. */

struct PackageVisitor;

const FIELDS: &[&str] = &[
    "Input",
    "MPs",
    "Checks",
    "Queries",
    "Tables",
    "Fields",
    "ConfigRules",
];

impl<'de> Deserialize<'de> for Package {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_struct("Package", FIELDS, PackageVisitor)
    }
}

impl<'de> Visitor<'de> for PackageVisitor {
    type Value = Package;

    fn expecting(
        &self,
        formatter: &mut std::fmt::Formatter,
    ) -> std::fmt::Result {
        write!(formatter, "a Package structure")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut input = None;
        let mut mps = None;
        let mut checks = None;
        let mut queries = None;
        let mut tables = None;
        let mut fields = None;
        let mut config_rules = None;

        while let Some(key) = map.next_key::<&str>()? {
            match key {
                "Input" => match input.is_some() {
                    true => return Err(A::Error::duplicate_field("Input")),
                    false => {
                        input = Some(map.next_value()?);
                    }
                },
                "MPs" => match mps.is_some() {
                    true => return Err(A::Error::duplicate_field("MPs")),
                    false => {
                        mps = Some(map.next_value()?);
                    }
                },
                "Checks" => match checks.is_some() {
                    true => return Err(A::Error::duplicate_field("Checks")),
                    false => {
                        checks = Some(map.next_value()?);
                    }
                },
                "Queries" => match queries.is_some() {
                    true => return Err(A::Error::duplicate_field("Queries")),
                    false => {
                        queries = Some(map.next_value()?);
                    }
                },
                "Tables" => match tables.is_some() {
                    true => return Err(A::Error::duplicate_field("Tables")),
                    false => {
                        tables = Some(map.next_value()?);
                    }
                },
                "Fields" => match fields.is_some() {
                    true => return Err(A::Error::duplicate_field("Fields")),
                    false => {
                        fields = Some(map.next_value()?);
                    }
                },
                "ConfigRules" => match config_rules.is_some() {
                    true => {
                        return Err(A::Error::duplicate_field("ConfigRules"))
                    }
                    false => config_rules = Some(map.next_value()?),
                },
                _ => {
                    /* ignore */
                    let _ = map.next_value::<IgnoredAny>();
                }
            }
        }

        Ok(Package {
            input: input.ok_or_else(|| A::Error::missing_field("Input"))?,
            etc: Etc {
                mps: mps.ok_or_else(|| A::Error::missing_field("MPs"))?,
                checks: checks
                    .ok_or_else(|| A::Error::missing_field("Checks"))?,
                queries: queries
                    .ok_or_else(|| A::Error::missing_field("Queries"))?,
                tables: tables
                    .ok_or_else(|| A::Error::missing_field("Tables"))?,
                fields: fields
                    .ok_or_else(|| A::Error::missing_field("Fields"))?,
                config_rules: config_rules.unwrap_or_default(),
            },
        })
    }
}

/*impl Package {
    /// Load from JSON file
    pub fn from_file<R: Read>(file: R) -> io::Result<Self> {
        Ok(serde_json::from_reader(file)?)
    }
}*/
