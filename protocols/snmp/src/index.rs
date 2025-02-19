/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::collections::{HashMap, HashSet};

use agent_utils::TryGetFrom;
use etc_base::ProtoDataFieldId;
use netsnmp::Oid;
use value::{Data, DataError};

use super::error::{Result, TypeResult};
use super::input::{Input, ObjectId};
use super::scalar::ScalarSpec;

#[derive(Hash)]
pub struct Index {
    pub vars: Vec<ObjectId>,
    pub implied: Option<ObjectId>,
}

impl Index {
    pub fn empty() -> Self {
        Index {
            vars: Vec::new(),
            implied: None,
        }
    }

    pub fn contains(&self, id: &ObjectId) -> bool {
        self.vars.contains(id) || self.implied.as_ref() == Some(id)
    }

    pub fn get_values(
        &self,
        oid: &Oid,
        input: &Input,
    ) -> Result<HashMap<ObjectId, Data>> {
        let scalars = self
            .vars
            .iter()
            .map(|object_id| {
                Ok((object_id, object_id.try_get_from(&input.scalars)?))
            })
            .collect::<Result<Vec<(&ObjectId, &ScalarSpec)>>>()?;
        let mut vals = HashMap::new();
        let _next = (0..scalars.len()).fold(Some(oid.as_slice()), |idx, i| {
            let (object_id, scalar) = scalars[i];
            let (val, next) = match idx {
                Some(idx) => {
                    let implied = self
                        .implied
                        .as_ref()
                        .map_or(false, |implied| object_id == implied);
                    let res = match implied {
                        true => {
                            let fixed_length: Option<usize> = scalars[(i + 1)..]
                                .iter()
                                .map(|(_, scalar)| scalar.fixed_index_length())
                                .sum();
                            match fixed_length {
                                Some(fixed_length) if fixed_length <= idx.len() => scalar.get_value_from_index(idx, Some(idx.len() - fixed_length)),
                                Some(_) => Err(DataError::TypeError(String::from("calculated implied length is longer than remaining index length"))),
                                None => Err(DataError::TypeError(String::from("unable to calculate implied length for implied index")))
                            }
                        }
                        false => scalar.get_value_from_index(idx, None),
                    };
                    match res {
                        Ok((val, next)) => (val, Some(next)),
                        Err(e) => (Err(e), None),
					}
				}
                None => (
                    Err(DataError::TypeError(String::from(
                        "previous index field failed to parse",
                    ))),
                    None,
                ),
            };
            vals.insert(object_id.clone(), val);
            next
        });

        Ok(vals)
    }

    pub fn to_field_id_set(
        &self,
        input: &Input,
    ) -> TypeResult<HashSet<ProtoDataFieldId>> {
        let mut key_set = HashSet::new();
        for key in self.vars.iter() {
            key_set.insert(key.to_field_id(input)?);
        }
        Ok(key_set)
    }
}
