/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use nom::{
    branch::alt,
    bytes::complete::{tag, take_while1},
    multi::many0,
    IResult,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, convert::TryFrom};

use crate::TemplateError;

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
#[serde(try_from = "String")]
pub struct Template(pub Vec<Elem>);

impl Template {
    pub fn fill_in(
        &self,
        vars: &HashMap<String, String>,
    ) -> Result<String, TemplateError> {
        Ok(self
            .0
            .iter()
            .map(|e| match e {
                Elem::Fixed(s) => Ok(s.as_ref()),
                Elem::Var(var) => match vars.get(var) {
                    Some(res) => Ok(res.as_ref()),
                    None => {
                        Err(TemplateError::MissingVariable(var.to_string()))
                    }
                },
            })
            .collect::<Result<Vec<&str>, TemplateError>>()?
            .concat())
    }

    pub fn parse(text: &str) -> Result<Self, TemplateError> {
        match template(text) {
            Ok(("", template)) => Ok(template),
            _ => Err(TemplateError::ParseError),
        }
    }
}

impl TryFrom<String> for Template {
    type Error = TemplateError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(&value)
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub enum Elem {
    Fixed(String),
    Var(String),
}

fn template(input: &str) -> IResult<&str, Template> {
    let (input, input_template) = many0(alt((variable, fixed)))(input)?;
    Ok((input, Template(input_template)))
}

fn variable(input: &str) -> IResult<&str, Elem> {
    let (input, _) = tag("{{")(input)?;
    let (input, var_name) = take_while1(|c| match c {
        '0'..='9' => true,
        'a'..='z' => true,
        'A'..='Z' => true,
        '_' => true,
        _ => false,
    })(input)?;
    let (input, _) = tag("}}")(input)?;
    Ok((input, Elem::Var(var_name.to_string())))
}

fn fixed(input: &str) -> IResult<&str, Elem> {
    let (input, text) = take_while1(|c| c != '{')(input)?;
    Ok((input, Elem::Fixed(text.to_string())))
}
