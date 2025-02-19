/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use serde::ser::Serializer;

pub fn serialize<S: Serializer>(serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_unit()
}
