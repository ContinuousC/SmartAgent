/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

#[cfg(feature = "schemars")]
#[test]
fn string_schema() {
    use schemars::schema_for;
    use serde_json::json;
    use unit::Unit;

    let schema = jsonschema::validator_for(
        &serde_json::to_value(schema_for!(Unit)).unwrap(),
    )
    .unwrap();

    #[cfg(feature = "serialize_as_string")]
    let examples = [json!("kB/s")];
    #[cfg(not(feature = "serialize_as_string"))]
    let examples = [json!({ "Information": { "Byte": "Kilo" } })];

    examples.iter().for_each(|example| {
        schema.validate(example).expect("schema validation failed");
    });
}
