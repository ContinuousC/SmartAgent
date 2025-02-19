/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::path::PathBuf;

use tokio::fs;

use super::error::{Error, Result};

pub async fn get_envvar_from_file(path: &str, var: &str) -> Result<String> {
    Ok(shell_descape(&fs::read_to_string(path).await?)
        .into_iter()
        .find(|comp| comp.starts_with(&format!("{}=", var)))
        .ok_or(Error::MissingEnvVarInFile(
            PathBuf::from(path),
            var.to_string(),
        ))?[var.len() + 1..]
        .to_string())
}

fn shell_descape(val: &str) -> Vec<String> {
    let mut chars = val.chars();
    let mut out = Vec::new();
    let mut cur = String::new();

    while let Some(c) = chars.next() {
        match c {
            ' ' | '\t' | '\n' => {
                if !cur.is_empty() {
                    out.push(cur);
                    cur = String::new();
                }
            }
            '\\' => {
                if let Some(c) = chars.next() {
                    match c {
                        'n' => cur.push('\n'),
                        'r' => cur.push('\r'),
                        't' => cur.push('\t'),
                        // 'x' => ...
                        // '0' => ...
                        _ => cur.push(c),
                    }
                }
            }
            '"' => {
                while let Some(c) = chars.next() {
                    match c {
                        '"' => break,
                        '\\' => {
                            if let Some(c) = chars.next() {
                                match c {
                                    'n' => cur.push('\n'),
                                    'r' => cur.push('\r'),
                                    't' => cur.push('\t'),
                                    // 'x' => ...
                                    // '0' => ...
                                    _ => cur.push(c),
                                }
                            }
                        }
                        _ => cur.push(c),
                    }
                }
            }
            '\'' => {
                for c in chars.by_ref() {
                    match c {
                        '\'' => break,
                        _ => cur.push(c),
                    }
                }
            }
            _ => cur.push(c),
        }
    }

    if !cur.is_empty() {
        out.push(cur);
    }

    out
}
