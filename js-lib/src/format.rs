/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

//use std::fmt::Write;

//use chrono::{DateTime, Duration, Local, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
//use wasm_bindgen::prelude::*;

use etc::FieldSpec;
use metrics_types::{Metric, Status, Thresholded};
use rule_engine::selector::ValueSelector;
use unit::{DecPrefix, Dimension, DimensionlessUnit, Unit};
use value::{FormatOpts, Type};

// abs (rel) [ warn / crit ]    { thresholds are configurable }
// abs (rel)                    { no thresholds are configurable }
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FormattedField {
    pub absolute: Option<Result<FormattedFieldValue, String>>,
    pub relative: Option<Result<FormattedFieldValue, String>>,
    pub thresholds: Option<Result<FormattedThresholds, String>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FormattedThresholds {
    pub warning: Option<FormattedThresholdValue>,
    pub critical: Option<FormattedThresholdValue>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FormattedFieldValue {
    pub formatted: String,
    pub sortable: Value,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FormattedThresholdValue {
    pub formatted: String,
    pub triggered: bool,
}

impl FormattedField {
    pub fn from_metric(
        metric: &Metric<Thresholded<Value, Value>>,
        spec: &FieldSpec,
    ) -> Result<Self, String> {
        Ok(FormattedField {
            absolute: metric.value.as_ref().map(
                |thresholded| match &thresholded.value {
                    Ok(value) => {
                        FormattedFieldValue::from_metric_abs(value, spec)
                    }
                    Err(e) => Err(e.to_string()),
                },
            ),
            relative: match &metric.relative {
                Some(thresholded) => match &thresholded.value {
                    Ok(value) => {
                        FormattedFieldValue::from_metric_rel(value, spec)
                            .transpose()
                    }
                    Err(e) => Some(Err(e.to_string())),
                },
                None => None,
            },
            thresholds: spec
                .threshold
                .as_ref()
                .map(|_| FormattedThresholds::from_metric(metric, spec)),
        })
    }
}

impl FormattedThresholds {
    pub fn from_metric(
        metric: &Metric<Thresholded<Value, Value>>,
        spec: &FieldSpec,
    ) -> Result<Self, String> {
        let abs_warning = metric
            .value
            .as_ref()
            .map(|v| {
                v.thresholds
                    .as_ref()
                    .map(|t| (t, v.status == Some(Ok(Status::Warning))))
            })
            .transpose()
            .map(|m| m.and_then(|(m, s)| m.warning.as_ref().map(|t| (t, s))));
        let rel_warning = metric
            .value
            .as_ref()
            .map(|v| {
                v.thresholds
                    .as_ref()
                    .map(|t| (t, v.status == Some(Ok(Status::Warning))))
            })
            .transpose()
            .map(|m| m.and_then(|(m, s)| m.warning.as_ref().map(|t| (t, s))));
        let abs_critical = metric
            .value
            .as_ref()
            .map(|v| {
                v.thresholds
                    .as_ref()
                    .map(|t| (t, v.status == Some(Ok(Status::Critical))))
            })
            .transpose()
            .map(|m| m.and_then(|(m, s)| m.critical.as_ref().map(|t| (t, s))));
        let rel_critical = metric
            .value
            .as_ref()
            .map(|v| {
                v.thresholds
                    .as_ref()
                    .map(|t| (t, v.status == Some(Ok(Status::Critical))))
            })
            .transpose()
            .map(|m| m.and_then(|(m, s)| m.critical.as_ref().map(|t| (t, s))));
        Ok(Self {
            warning: match abs_warning? {
                Some((threshold, triggered)) => {
                    Some(FormattedThresholdValue::from_metric_abs(
                        threshold, spec, triggered,
                    )?)
                }
                None => match rel_warning? {
                    Some((threshold, triggered)) => {
                        FormattedThresholdValue::from_metric_rel(
                            threshold, spec, triggered,
                        )?
                    }
                    None => None,
                },
            },
            critical: match abs_critical? {
                Some((threshold, triggered)) => {
                    Some(FormattedThresholdValue::from_metric_abs(
                        threshold, spec, triggered,
                    )?)
                }
                None => match rel_critical? {
                    Some((threshold, triggered)) => {
                        FormattedThresholdValue::from_metric_rel(
                            threshold, spec, triggered,
                        )?
                    }
                    None => None,
                },
            },
        })
    }
}

impl FormattedFieldValue {
    pub fn from_metric_abs(
        value: &Value,
        spec: &FieldSpec,
    ) -> Result<Self, String> {
        let value = spec
            .input_type
            .value_from_json_unit(value.clone(), spec.display_unit)
            .map_err(|e| e.to_string())?;
        Self::from_value(value, &format_opts_abs(spec)?)
    }
    pub fn from_metric_rel(
        value: &Value,
        spec: &FieldSpec,
    ) -> Result<Option<Self>, String> {
        if matches!(
            &spec.input_type,
            Type::Integer | Type::Float | Type::Quantity(_)
        ) && spec.reference.is_some()
        {
            match format_opts_rel(spec)? {
                Some(format_opts) => {
                    let value = Type::Quantity(Dimension::Dimensionless)
                        .value_from_json_unit(
                            value.clone(),
                            Some(
                                spec.relative_display_type
                                    .unwrap_or(
                                        etc::RelativeDisplayType::Percentage,
                                    )
                                    .display_unit(),
                            ),
                        )
                        .map_err(|e| e.to_string())?;
                    Self::from_value(value, &format_opts).map(Some)
                }
                None => Ok(None),
            }
        } else {
            Err("relative value is not defined for this field".to_string())
        }
    }
    pub fn from_value(
        value: value::Value,
        opts: &FormatOpts,
    ) -> Result<Self, String> {
        Ok(Self {
            formatted: value.format(opts).map_err(|e| e.to_string())?,
            sortable: value.to_sortable_json_value(),
        })
    }
}

impl FormattedThresholdValue {
    pub fn from_metric_abs(
        value: &Value,
        spec: &FieldSpec,
        triggered: bool,
    ) -> Result<Self, String> {
        Ok(Self {
            formatted: ValueSelector::from_metric_abs(
                &spec.input_type,
                &spec.display_unit,
                value,
            )
            .map_err(|e| e.to_string())?
            .format(&format_opts_abs(spec)?)
            .map_err(|e| e.to_string())?,
            triggered,
        })
    }
    pub fn from_metric_rel(
        value: &Value,
        spec: &FieldSpec,
        triggered: bool,
    ) -> Result<Option<Self>, String> {
        match format_opts_rel(spec)? {
            Some(format_opts) => Ok(Some(Self {
                formatted: ValueSelector::from_metric_rel(
                    &spec.input_type,
                    value,
                )
                .map_err(|e| e.to_string())?
                .format(&format_opts)
                .map_err(|e| e.to_string())?,
                triggered,
            })),
            None => Ok(None),
        }
    }
}

fn parse_format(input: &str) -> Result<u8, String> {
    match format_string(input) {
        Ok(("", precision)) => Ok(precision),
        _ => Err(String::from("invalid format string")),
    }
}

fn format_string(input: &str) -> nom::IResult<&str, u8> {
    let (input, (_, precision, _)) = nom::sequence::tuple((
        nom::bytes::complete::tag("%."),
        nom::combinator::map_res(
            nom::character::complete::digit1,
            |s: &str| s.parse(),
        ),
        nom::branch::alt((
            nom::character::complete::char('d'),
            nom::character::complete::char('f'),
        )),
    ))(input)?;
    Ok((input, precision))
}

pub(crate) fn format(
    value: Value,
    rel_value: Option<Value>,
    spec: FieldSpec,
) -> Result<String, String> {
    let abs_formatted = FormattedFieldValue::from_metric_abs(&value, &spec)?;
    Ok(match rel_value {
        None => abs_formatted.formatted,
        Some(rel_value) => {
            match FormattedFieldValue::from_metric_rel(&rel_value, &spec)? {
                None => abs_formatted.formatted,
                Some(rel_formatted) => format!(
                    "{} ({})",
                    abs_formatted.formatted, rel_formatted.formatted
                ),
            }
        }
    })
}

fn format_opts_abs(spec: &FieldSpec) -> Result<FormatOpts, String> {
    Ok(FormatOpts {
        autoscale: true, // spec.autoscale
        precision: match &spec.numeric_format {
            Some(fmt) => Some(parse_format(fmt)?),
            None => None,
        },
        unit: spec.display_unit,
    })
}

fn format_opts_rel(spec: &FieldSpec) -> Result<Option<FormatOpts>, String> {
    let unit = match spec
        .relative_display_type
        .as_ref()
        .unwrap_or(&etc::RelativeDisplayType::Percentage)
    {
        etc::RelativeDisplayType::Hidden => return Ok(None),
        etc::RelativeDisplayType::Percentage => {
            Unit::Dimensionless(DimensionlessUnit::Percent)
        }
        etc::RelativeDisplayType::Ratio => {
            Unit::Dimensionless(DimensionlessUnit::Count(DecPrefix::Unit))
        }
    };
    Ok(Some(FormatOpts {
        autoscale: false,
        precision: match &spec.relative_format {
            Some(fmt) => Some(parse_format(fmt)?),
            None => None,
        },
        unit: Some(unit),
    }))
}

// pub(crate) fn format(
//     value: &JsValue,
//     rel_value: &JsValue,
//     field: FieldSpec,
// ) -> Result<String, String> {
//     match field.input_type {
//         Type::Quantity(dimension) => {
//             let value: Option<f64> =
//                 JsValue::into_serde(value).map_err(|e| e.to_string())?;
//             let rel_value: Option<f64> = match field.reference.is_some() {
//                 true => {
//                     JsValue::into_serde(rel_value).map_err(|e| e.to_string())?
//                 }
//                 false => None,
//             };

//             let formatted_value = match value {
//                 None => None,
//                 Some(value) => {
//                     let q = Quantity(value, dimension.reference_unit());
//                     let u = field
//                         .display_unit
//                         .unwrap_or_else(|| dimension.reference_unit());
//                     let v = q.convert(&u).map_err(|e| e.to_string())?.0;
//                     let fmt = field.numeric_format.as_deref().unwrap_or("%.2f");
//                     Some(format!("{}{}", FloatFmt::new(fmt, v)?, &u))
//                 }
//             };

//             let formatted_rel_value = match rel_value {
//                 None => None,
//                 Some(rel_value) => {
//                     match field
//                         .relative_display_type
//                         .unwrap_or(RelativeDisplayType::Percentage)
//                     {
//                         RelativeDisplayType::Hidden => None,
//                         RelativeDisplayType::Percentage => {
//                             let fmt = field
//                                 .relative_format
//                                 .as_deref()
//                                 .unwrap_or("%.0f");
//                             Some(format!(
//                                 "{}%",
//                                 FloatFmt::new(fmt, rel_value * 100.0)?
//                             ))
//                         }
//                         RelativeDisplayType::Ratio => {
//                             let fmt = field
//                                 .relative_format
//                                 .as_deref()
//                                 .unwrap_or("%.2f");
//                             Some(format!("{}x", FloatFmt::new(fmt, rel_value)?))
//                         }
//                     }
//                 }
//             };

//             Ok(match (formatted_value, formatted_rel_value) {
//                 (None, _) => String::from("-"),
//                 (Some(value), None) => value,
//                 (Some(value), Some(rel_value)) => {
//                     format!("{} ({})", value, rel_value)
//                 }
//             })
//         }

//         Type::Float => {
//             let value: Option<f64> =
//                 JsValue::into_serde(value).map_err(|e| e.to_string())?;
//             let rel_value: Option<f64> = match field.reference.is_some() {
//                 true => {
//                     JsValue::into_serde(rel_value).map_err(|e| e.to_string())?
//                 }
//                 false => None,
//             };

//             let formatted_value = match value {
//                 None => None,
//                 Some(value) => {
//                     let fmt = field.numeric_format.as_deref().unwrap_or("%.2f");
//                     Some(format!("{}", FloatFmt::new(fmt, value)?,))
//                 }
//             };

//             let formatted_rel_value = match rel_value {
//                 None => None,
//                 Some(rel_value) => {
//                     match field
//                         .relative_display_type
//                         .unwrap_or(RelativeDisplayType::Percentage)
//                     {
//                         RelativeDisplayType::Hidden => None,
//                         RelativeDisplayType::Percentage => {
//                             let fmt = field
//                                 .relative_format
//                                 .as_deref()
//                                 .unwrap_or("%.0f");
//                             Some(format!(
//                                 "{}%",
//                                 FloatFmt::new(fmt, rel_value * 100.0)?
//                             ))
//                         }
//                         RelativeDisplayType::Ratio => {
//                             let fmt = field
//                                 .relative_format
//                                 .as_deref()
//                                 .unwrap_or("%.2f");
//                             Some(format!("{}x", FloatFmt::new(fmt, rel_value)?))
//                         }
//                     }
//                 }
//             };

//             Ok(match (formatted_value, formatted_rel_value) {
//                 (None, _) => String::from("-"),
//                 (Some(value), None) => value,
//                 (Some(value), Some(rel_value)) => {
//                     format!("{} ({})", value, rel_value)
//                 }
//             })
//         }

//         Type::Integer => {
//             let value: Option<i64> =
//                 JsValue::into_serde(value).map_err(|e| e.to_string())?;
//             let rel_value: Option<f64> = match field.reference.is_some() {
//                 true => {
//                     JsValue::into_serde(rel_value).map_err(|e| e.to_string())?
//                 }
//                 false => None,
//             };

//             let formatted_value = match value {
//                 None => None,
//                 Some(value) => Some(format!("{}", value)),
//             };

//             let formatted_rel_value = match rel_value {
//                 None => None,
//                 Some(rel_value) => {
//                     match field
//                         .relative_display_type
//                         .unwrap_or(RelativeDisplayType::Percentage)
//                     {
//                         RelativeDisplayType::Hidden => None,
//                         RelativeDisplayType::Percentage => {
//                             let fmt = field
//                                 .relative_format
//                                 .as_deref()
//                                 .unwrap_or("%.0f");
//                             Some(format!(
//                                 "{}%",
//                                 FloatFmt::new(fmt, rel_value * 100.0)?
//                             ))
//                         }
//                         RelativeDisplayType::Ratio => {
//                             let fmt = field
//                                 .relative_format
//                                 .as_deref()
//                                 .unwrap_or("%.2f");
//                             Some(format!("{}x", FloatFmt::new(fmt, rel_value)?))
//                         }
//                     }
//                 }
//             };

//             Ok(match (formatted_value, formatted_rel_value) {
//                 (None, _) => String::from("-"),
//                 (Some(value), None) => value,
//                 (Some(value), Some(rel_value)) => {
//                     format!("{} ({})", value, rel_value)
//                 }
//             })
//         }

//         Type::UnicodeString
//         | Type::MacAddress
//         | Type::Ipv4Address
//         | Type::Ipv6Address => {
//             // verify Type::MacAddress | Type::Ipv4Address | Type::Ipv6Address ?

//             let value: Option<String> =
//                 JsValue::into_serde(value).map_err(|e| e.to_string())?;

//             Ok(match value {
//                 Some(value) => value,
//                 None => String::from("-"),
//             })
//         }

//         Type::BinaryString => {
//             let value: Option<Vec<u8>> =
//                 JsValue::into_serde(value).map_err(|e| e.to_string())?;
//             Ok(match value {
//                 Some(value) => {
//                     let mut s = String::with_capacity(value.len() * 2);
//                     value.iter().for_each(|c| write!(s, "{:02x}", c).unwrap());
//                     s
//                 }
//                 None => String::from("-"),
//             })
//         }

//         Type::Enum(cs) => {
//             let value: Option<String> =
//                 JsValue::into_serde(value).map_err(|e| e.to_string())?;

//             match value {
//                 Some(value) => match cs.contains(&value) {
//                     true => Ok(value),
//                     false => Err(String::from("invalid value for enum")),
//                 },
//                 None => Ok(String::from("-")),
//             }
//         }

//         Type::IntEnum(cs) => {
//             let value: Option<String> =
//                 JsValue::into_serde(value).map_err(|e| e.to_string())?;

//             match value {
//                 Some(value) => {
//                     match cs.values().find(|val| *val == &value).is_some() {
//                         true => Ok(value),
//                         false => Err(String::from("invalid value for enum")),
//                     }
//                 }
//                 None => Ok(String::from("-")),
//             }
//         }

//         Type::Boolean => {
//             let value: Option<bool> =
//                 JsValue::into_serde(value).map_err(|e| e.to_string())?;

//             // match field.boolean_format
//             Ok(match value {
//                 Some(true) => String::from("true"),
//                 Some(false) => String::from("false"),
//                 None => String::from("-"),
//             })
//         }

//         Type::Time => {
//             let value: Option<String> =
//                 JsValue::into_serde(value).map_err(|e| e.to_string())?;

//             match value {
//                 Some(value) => {
//                     let value = DateTime::parse_from_rfc3339(value.as_str())
//                         .map_err(|e| e.to_string())?
//                         .with_timezone(&Utc);
//                     match &field.time_display_type {
//                         None | Some(TimeDisplayType::Time) => {
//                             // field.time_format ...
//                             Ok(value.with_timezone(&Local).to_rfc2822())
//                         }
//                         Some(TimeDisplayType::Age) => {
//                             let value = value.signed_duration_since(Utc::now());
//                             // todo: improve output
//                             Ok(format!("{}", value))
//                         }
//                     }
//                 }
//                 None => Ok(String::from("-")),
//             }
//         }

//         Type::Age => {
//             let value: Option<f64> =
//                 JsValue::into_serde(value).map_err(|e| e.to_string())?;

//             match value {
//                 Some(value) => {
//                     let value = Duration::milliseconds((value * 1000.0) as i64);
//                     match &field.time_display_type {
//                         None | Some(TimeDisplayType::Time) => {
//                             let value = Utc::now() + value;
//                             Ok(value.with_timezone(&Local).to_rfc2822())
//                         }
//                         Some(TimeDisplayType::Age) => Ok(format!("{}", value)),
//                     }
//                 }
//                 None => Ok(String::from("-")),
//             }
//         }

//         Type::Option(_)
//         | Type::Result(_, _)
//         | Type::Set(_)
//         | Type::Tuple(_) => Err(format!("Unimplemented InputType for format")),
//     }
// }

// pub fn convert(value: f64, unit: Unit) -> Result<f64, UnitError> {
//     Ok(Quantity(value, unit.normalize()).convert(&unit)?.0)
// }

// struct FloatFmt {
//     precision: usize,
//     val: f64,
// }

// impl FloatFmt {
//     fn new(fmt: &str, val: f64) -> Result<Self, String> {
//         let mut chars = fmt.chars().peekable();

//         match chars.next() {
//             Some('%') => Ok(()),
//             _ => Err(String::from("Invalid format string: expected '%'")),
//         }?;

//         match chars.next() {
//             Some('.') => Ok(()),
//             _ => Err(String::from("Invalid format string: expected '.'")),
//         }?;

//         let mut precision: usize = 0;
//         while let Some(c) = chars.peek() {
//             if let Some(v) = c.to_digit(10) {
//                 precision = precision * 10 + v as usize;
//                 chars.next();
//             } else {
//                 break;
//             }
//         }

//         match chars.next() {
//             Some('f') => Ok(()),
//             Some('d') => Ok(()),
//             _ => {
//                 Err(String::from("Invalid format string: expected 'f' or 'd'"))
//             }
//         }?;

//         match chars.next() {
//             None => Ok(()),
//             _ => Err(String::from(
//                 "Invalid format string: unexpected end of string",
//             )),
//         }?;

//         Ok(Self { precision, val })
//     }
// }

// impl fmt::Display for FloatFmt {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         write!(f, "{:.*}", &self.precision, &self.val)?;
//         Ok(())
//     }
// }
