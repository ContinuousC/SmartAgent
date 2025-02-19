/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::collections::HashMap;

use netsnmp::Oid;
use serde::{Deserialize, Serialize};

use agent_utils::{DBObj, Key, TryAppend, TryGetFrom};
use etc_base::{ProtoDataFieldId, ProtoDataTableId};

use super::entry::EntrySpec;
use super::error::{TypeError, TypeResult};
use super::scalar::ScalarSpec;

/* SNMP-specific IDs. */

/// An object id identifies a specific version of an object
/// as defined in a module. This is differentiated from the Oid
/// since subsequent module versions can provide distinct, possibly
/// conflicting, definitions for the same Oid.
#[derive(
    Serialize,
    Deserialize,
    Debug,
    Clone,
    Key,
    Hash,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
)]
#[serde(transparent)]
pub struct ObjectId(String);

/*impl KeyFor<ModuleSpec> for ObjectId {}
impl KeyFor<EntrySpec> for ObjectId {}
impl KeyFor<ScalarSpec> for ObjectId {}
impl KeyFor<EventSpec> for ObjectId {}*/

/* Input specification. */

#[derive(Serialize, Deserialize, DBObj, PartialEq, Eq, Clone, Debug)]
#[serde(rename_all = "PascalCase")]
#[derive(Default)]
pub struct Input {
    /* SNMP objects. */
    pub objects: HashMap<ObjectId, ObjectSpec>,
    pub modules: HashMap<ObjectId, ModuleSpec>,
    pub tables: HashMap<ObjectId, EntrySpec>,
    pub scalars: HashMap<ObjectId, ScalarSpec>,
    pub events: HashMap<ObjectId, EventSpec>,
}

#[derive(DBObj, Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct ObjectSpec {
    #[serde(rename = "ModuleId")]
    pub module: Option<ObjectId>,
    #[serde(rename = "Oid")]
    #[serde(alias = "OID")]
    pub oid: Oid,
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Type")]
    pub typ: ObjectType,
    #[serde(rename = "ContextGroup")]
    pub context_group: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub enum ObjectType {
    Module, // MODULE-IDENTITY
    Table,  // OBJECT-TYPE with entry syntax (net-snmp says OTHER)
    Scalar, // OBJECT-TYPE with scalar syntax
    Event,  // NOTIFICATION-TYPE or TRAP-TYPE
}

#[derive(DBObj, Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct ModuleSpec {
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Organization")]
    pub organization: String,
    #[serde(rename = "LastUpdated")]
    pub last_updated: String,
}

#[derive(DBObj, Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct EventSpec {
    pub variables: Vec<ObjectId>,
}

impl TryAppend for Input {
    fn try_append(&mut self, other: Self) -> agent_utils::Result<()> {
        self.objects.try_append(other.objects)?;
        self.modules.try_append(other.modules)?;
        self.tables.try_append(other.tables)?;
        self.scalars.try_append(other.scalars)?;
        self.events.try_append(other.events)?;
        Ok(())
    }
}

impl ObjectId {
    pub fn from_table_id(
        id: &ProtoDataTableId,
        input: &Input,
    ) -> TypeResult<Option<Self>> {
        match id.0 == "noIndex" {
            true => Ok(None),
            false => {
                let obj_id = Self(id.0.to_string());
                match obj_id.try_get_from(&input.objects)?.typ {
                    ObjectType::Table => Ok(Some(obj_id)),
                    _ => Err(TypeError::InvalidTableId(id.clone())),
                }
            }
        }
    }
    pub fn from_field_id(
        id: &ProtoDataFieldId,
        input: &Input,
    ) -> TypeResult<Self> {
        let obj_id = Self(id.0.to_string());
        match obj_id.try_get_from(&input.objects)?.typ {
            ObjectType::Scalar => Ok(obj_id),
            _ => Err(TypeError::InvalidFieldId(id.clone())),
        }
    }
    pub fn to_table_id(&self, input: &Input) -> TypeResult<ProtoDataTableId> {
        match self.try_get_from(&input.objects)?.typ {
            ObjectType::Table => Ok(ProtoDataTableId(self.0.to_string())),
            _ => Err(TypeError::InvalidTable(self.clone())),
        }
    }
    pub fn to_field_id(&self, input: &Input) -> TypeResult<ProtoDataFieldId> {
        match self.try_get_from(&input.objects)?.typ {
            ObjectType::Scalar => Ok(ProtoDataFieldId(self.0.to_string())),
            _ => Err(TypeError::InvalidField(self.clone())),
        }
    }

    /// Handle erroneous "noIndex" ObjectIds; this is a valid ProtoDataTableId,
    /// but not an actual ObjectId.
    pub fn handle_noindex(self) -> Option<Self> {
        match self.0 == "noIndex" {
            false => Some(self),
            true => None,
        }
    }
}
