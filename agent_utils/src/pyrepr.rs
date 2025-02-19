/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::fmt::{self, Display};

pub struct PyBytes<'a>(pub &'a [u8]);
pub struct PyString<'a>(pub &'a str);
pub struct PyUnicode<'a>(pub &'a str);

impl Display for PyBytes<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "'")?;
        self.0.iter().try_for_each(|b| write!(f, "\\x{:02x}", b))?;
        write!(f, "'")
    }
}

impl Display for PyString<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut buf = [0; 4];
        write!(f, "'")?;
        for c in self.0.chars() {
            match c {
                '\n' => write!(f, "\\n")?,
                '\t' => write!(f, "\\t")?,
                '\'' => write!(f, "\\'")?,
                '\\' => write!(f, "\\\\")?,
                c if c.is_control() => c
                    .encode_utf8(&mut buf)
                    .as_bytes()
                    .iter()
                    .try_for_each(|b| write!(f, "\\x{:02x}", b))?,
                c => write!(f, "{}", c)?,
            }
        }
        write!(f, "'")
    }
}

impl Display for PyUnicode<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut buf = [0; 2];
        write!(f, "u'")?;
        for c in self.0.chars() {
            match c {
                '\n' => write!(f, "\\n")?,
                '\t' => write!(f, "\\t")?,
                '\'' => write!(f, "\\'")?,
                '\\' => write!(f, "\\\\")?,
                c if c.is_control() => {
                    let r = c.encode_utf16(&mut buf);
                    match r.len() {
                        1 => write!(f, "\\u{:04x}", buf[0])?,
                        2 => write!(f, "\\U{:04x}{:04x}", buf[0], buf[1])?,
                        _ => panic!("Unexpected unicode character length!"),
                    }
                }
                c => write!(f, "{}", c)?,
            }
        }
        write!(f, "'")
    }
}
