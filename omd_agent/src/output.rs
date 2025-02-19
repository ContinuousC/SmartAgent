/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use chrono::SecondsFormat;
use std::collections::{HashMap, HashSet};
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::{fs, io};

use crate::config::ErrorReporting;
use crate::context::Context;
use crate::env;
use crate::error::Result;
use crate::problems::{get_problems, load_problems};

use agent_utils::TryGetFrom;
use etc_base::{CheckId, FieldId, TableId};
use expression::EvalError;
use query::AnnotatedQueryResult;
use value::{DataError, HashableValue, Value};

type EvalResult = std::result::Result<Value, EvalError>;
type EvaluatedRow = HashMap<FieldId, EvalResult>;
pub type TableData = AnnotatedQueryResult<Vec<EvaluatedRow>>;

pub fn write_output(
    checks: &HashMap<CheckId, HashSet<TableId>>,
    data: &HashMap<TableId, TableData>,
    ctx: &Context,
) -> Result<()> {
    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());

    // Output agent version

    writeln!(out, "<<<check_mk>>>")?;
    writeln!(out, "Version: {}", env!("CARGO_PKG_VERSION"))?;
    writeln!(out, "AgentOS: linux")?;

    // Ouput checks

    for (check_id, tables) in checks {
        writeln!(out, "<<<{}>>>", check_id.0)?;

        let exists = tables.iter().any(|table_id| match data.get(table_id) {
            Some(Ok(_)) => true,
            _ => false,
        });

        if !exists {
            writeln!(out, "ERROR")?;
        } else {
            write_tables(ctx, &mut out, tables, data)?;
        }
    }

    // Output problems to dependency check / file.

    /* Load errors from previous agents in the chain from the "cache file",
     * filter out errors for checks handled by us, and add errors found while
     * running queries for "our" checks.
     */

    let check_names = checks
        .keys()
        .map(|check_id| {
            let check = check_id.try_get_from(&ctx.spec.etc.checks)?;
            let mp = check.mp.try_get_from(&ctx.spec.etc.mps)?;
            Ok(format!("{} {}", &mp.name, &check.name))
        })
        .collect::<Result<HashSet<String>>>()?;

    let error_path = env::omd_root()?.join(env::ERRORS_PATH);
    let errors_file = PathBuf::from(format!(
        "{}.mk",
        error_path.join(&ctx.options.host_name).display()
    ));
    let seen_dir = error_path.join("seen");
    let seen_file = seen_dir.join(&ctx.options.host_name);

    let problems = load_problems(&errors_file)?
        .unwrap_or_default()
        .into_iter()
        .filter(|p| !check_names.contains(&p.check))
        .chain(get_problems(checks, data, ctx)?)
        .collect::<Vec<_>>();

    match &ctx.config.agent.error_reporting {
        ErrorReporting::Handle { move_error_file } => {
            /* Print errors for dependency check. */
            writeln!(out, "<<<Dependency_Check>>>")?;
            for problem in problems {
                writeln!(out, "{}", problem.into_line())?;
                if *move_error_file {
                    fs::create_dir_all(&seen_dir)?;
                    fs::rename(&errors_file, &seen_file)?;
                }
            }
        }
        ErrorReporting::Legacy => {
            /* Print errors to errors file. */
            fs::create_dir_all(error_path)?;
            let mut file = fs::File::create(&errors_file)?;
            for problem in problems {
                writeln!(file, "{}", problem.into_line())?;
            }
        }
    }

    Ok(())
}

fn write_tables<T: Write>(
    ctx: &Context,
    out: &mut T,
    tables: &HashSet<TableId>,
    data: &HashMap<TableId, TableData>,
) -> Result<()> {
    write!(out, "{{")?;

    for table_id in tables {
        if let Some(Ok(res)) = data.get(table_id) {
            write_str(out, &table_id.0)?;
            write!(out, ":")?;
            write_table(ctx, out, &res.value)?;
            write!(out, ",")?;
        }
    }

    writeln!(out, "}}")?;
    Ok(())
}

fn write_table<T: Write>(
    ctx: &Context,
    out: &mut T,
    rows: &Vec<EvaluatedRow>,
) -> Result<()> {
    write!(out, "[")?;

    for row in rows {
        write_row(ctx, out, row)?;
        write!(out, ",")?;
    }

    write!(out, "]")?;
    Ok(())
}

fn write_row<T: Write>(
    ctx: &Context,
    out: &mut T,
    row: &EvaluatedRow,
) -> Result<()> {
    write!(out, "{{")?;

    for (field_id, val) in row {
        write_str(out, &field_id.0)?;
        write!(out, ":")?;
        write_field(ctx, out, val)?;
        write!(out, ",")?;
    }

    write!(out, "}}")?;
    Ok(())
}

fn write_field<T: Write>(
    ctx: &Context,
    out: &mut T,
    val: &EvalResult,
) -> Result<()> {
    match val {
        Ok(v) => write_value(out, v)?,
        Err(EvalError::DataError(DataError::CounterOverflow)) => {
            write!(out, "\"(...)\"")?
        }
        Err(EvalError::DataError(DataError::CounterPending)) => {
            write!(out, "\"(...)\"")?
        }
        Err(EvalError::ErrorValue(s)) => write_str(out, s)?,
        Err(e) => match ctx.config.agent.show_field_errors {
            true => write_str(out, &format!("{}", e))?,
            false => write!(out, "None")?,
        },
    }

    Ok(())
}

fn write_value<T: Write>(out: &mut T, val: &Value) -> Result<()> {
    match val {
        Value::BinaryString(v) => write_bytes(out, v)?,
        Value::UnicodeString(v) => write_str(out, v)?,
        Value::Integer(v) => write!(out, "{v}")?,
        Value::Float(v) => match v.is_finite() {
            true => write!(out, "{v}")?,
            false => write!(out, "None")?,
        },
        Value::Quantity(v) => match v.normalize() {
            Ok(v) => match v.0.is_finite() {
                true => write!(out, "{}", v.0)?,
                false => write!(out, "None")?,
            },
            Err(_) => write!(out, "None")?,
        },
        Value::Enum(v) => write_str(out, v.get_value())?,
        Value::IntEnum(v) => write_str(out, v.get_value_str())?,
        Value::Boolean(v) => match v {
            true => write!(out, "True")?,
            false => write!(out, "False")?,
        },
        Value::Time(v) => write_str(
            out,
            &v.to_rfc3339_opts(SecondsFormat::Micros, true).to_string(),
        )?,
        Value::Age(v) => {
            let seconds = match v.num_nanoseconds() {
                Some(v) => v as f64 / 1000000000.0,
                None => match v.num_microseconds() {
                    Some(v) => v as f64 / 1000000.0,
                    None => v.num_milliseconds() as f64 / 1000.0,
                },
            };
            write!(out, "{}", seconds)?
        }
        Value::MacAddress(v) => write!(
            out,
            "'{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}'",
            v[0], v[1], v[2], v[3], v[4], v[5],
        )?,
        Value::Ipv4Address(v) => {
            write!(out, "'{}.{}.{}.{}'", v[0], v[1], v[2], v[3])?
        }
        Value::Ipv6Address(v) => write!(
            out,
            "'{:x}:{:x}:{:x}:{:x}:{:x}:{:x}:{:x}:{:x}'",
            v[0], v[1], v[2], v[3], v[4], v[5], v[6], v[7]
        )?,
        Value::Option(v) => match v.get_value() {
            Some(v) => write_value(out, v)?,
            None => write!(out, "None")?,
        },
        Value::Result(v) => match v.get_value() {
            Ok(v) => write_value(out, v)?,
            Err(v) => write_value(out, v)?,
        },
        Value::Tuple(vs) => {
            write!(out, "(")?;
            for (i, v) in vs.iter().enumerate() {
                if i > 0 {
                    write!(out, ",")?;
                }
                write_value(out, v)?;
            }
            write!(out, ")")?;
        }
        Value::List(v) => {
            write!(out, "[")?;
            for (i, v) in v.get_values().iter().enumerate() {
                if i > 0 {
                    write!(out, ",")?;
                }
                write_value(out, v)?;
            }
            write!(out, "]")?;
        }
        Value::Set(v) => {
            write!(out, "[")?;
            for (i, v) in v.get_values().iter().enumerate() {
                if i > 0 {
                    write!(out, ",")?;
                }
                write_hashable_value(out, v)?;
            }
            write!(out, "]")?;
        }
        Value::Map(v) => {
            write!(out, "{{")?;
            for (i, (k, v)) in v.get_values().iter().enumerate() {
                if i > 0 {
                    write!(out, ",")?;
                }
                write_hashable_value(out, k)?;
                write!(out, ": ")?;
                write_value(out, v)?;
            }
            write!(out, "}}")?;
        }
        Value::Json(v) => {
            write!(out, "json.loads(")?;
            write_str(out, &serde_json::to_string(v).unwrap())?;
            write!(out, ")")?;
        }
    };
    Ok(())
}

fn write_hashable_value<T: Write>(
    out: &mut T,
    val: &HashableValue,
) -> Result<()> {
    match val {
        HashableValue::BinaryString(v) => write_bytes(out, v)?,
        HashableValue::UnicodeString(v) => write_str(out, v)?,
        HashableValue::Integer(v) => write!(out, "{v}")?,
        HashableValue::Enum(v) => write_str(out, v.get_value())?,
        HashableValue::IntEnum(v) => write_str(out, v.get_value_str())?,
        HashableValue::Boolean(v) => match v {
            true => write!(out, "True")?,
            false => write!(out, "False")?,
        },
        HashableValue::MacAddress(v) => write!(
            out,
            "'{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}'",
            v[0], v[1], v[2], v[3], v[4], v[5],
        )?,
        HashableValue::Ipv4Address(v) => {
            write!(out, "'{}.{}.{}.{}'", v[0], v[1], v[2], v[3])?
        }
        HashableValue::Ipv6Address(v) => write!(
            out,
            "'{:x}:{:x}:{:x}:{:x}:{:x}:{:x}:{:x}:{:x}'",
            v[0], v[1], v[2], v[3], v[4], v[5], v[6], v[7]
        )?,
        HashableValue::Option(v) => match v.get_value() {
            Some(v) => write_hashable_value(out, v)?,
            None => write!(out, "None")?,
        },
        HashableValue::Result(v) => match v.get_value() {
            Ok(v) => write_hashable_value(out, v)?,
            Err(v) => write_hashable_value(out, v)?,
        },
        HashableValue::List(v) => {
            write!(out, "[")?;
            for (i, v) in v.get_values().iter().enumerate() {
                if i > 0 {
                    write!(out, ",")?;
                }
                write_hashable_value(out, v)?;
            }
            write!(out, "]")?;
        }
        HashableValue::Tuple(vs) => {
            write!(out, "(")?;
            for (i, v) in vs.iter().enumerate() {
                if i > 0 {
                    write!(out, ",")?;
                }
                write_hashable_value(out, v)?;
            }
            write!(out, ")")?;
        }
    };
    Ok(())
}

fn write_str<T: Write>(out: &mut T, val: &str) -> Result<()> {
    let mut buf = [0; 4];

    write!(out, "u'")?;

    for c in val.chars() {
        match c {
            '\n' => write!(out, "\\\\n")?,
            '\'' => write!(out, "\\\\'")?,
            '\\' => write!(out, "\\\\\\\\")?,
            c => {
                // we don't check for a direct space since output can contain unicode whitespaces: https://www.unicode.org/Public/UCD/latest/ucd/PropList.txt
                if c.is_whitespace() {
                    write!(out, "\\s")?
                } else if !c.is_control() {
                    write!(out, "{}", c)?
                } else {
                    let r = c.encode_utf16(&mut buf);
                    match r.len() {
                        1 => write!(out, "\\u{:04x}", buf[0])?,
                        2 => write!(out, "\\U{:04x}{:04x}", buf[0], buf[1])?,
                        _ => panic!("Unexpected unicode character length!"),
                    }
                }
            }
        }
    }

    write!(out, "'")?;
    Ok(())
}

fn write_bytes<T: Write>(out: &mut T, val: &Vec<u8>) -> Result<()> {
    write!(out, "'")?;

    for c in val {
        match c {
            b' ' => write!(out, "\\s")?,
            b'\n' => write!(out, "\\\\n")?,
            b'\'' => write!(out, "\\\\'")?,
            b'\\' => write!(out, "\\\\\\\\")?,
            c => {
                if *c >= 32 && *c <= 127 {
                    write!(out, "{}", *c as char)?
                } else {
                    write!(out, "\\\\x{:02x}", c)?
                }
            }
        }
    }

    write!(out, "'")?;
    Ok(())
}
