/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use xml::{attribute::OwnedAttribute, name::OwnedName, reader::XmlEvent};

use super::error::{ParseError, ParseResult};

pub type XmlInput<'a> = &'a [XmlEvent];

pub trait FromXml: Sized {
    fn from_xml(xml: XmlInput) -> ParseResult<Self>;
}

/* Parsers. */

pub fn start_document(xml: XmlInput) -> ParseResult<()> {
    let (event, xml) = next(xml)?;
    match event {
        XmlEvent::StartDocument { .. } => Ok(((), xml)),
        _ => Err(ParseError::Unexpected(event.clone())),
    }
}

pub fn end_document(xml: XmlInput) -> ParseResult<()> {
    let (event, xml) = next(xml)?;
    match event {
        XmlEvent::EndDocument => Ok(((), xml)),
        _ => Err(ParseError::Unexpected(event.clone())),
    }
}

pub fn start_tag<'a>(
    xml: XmlInput<'a>,
    ns: &str,
    local: &str,
) -> ParseResult<'a, (&'a OwnedName, &'a [OwnedAttribute])> {
    let (event, xml) = next(xml)?;
    match event {
        XmlEvent::StartElement {
            name, attributes, ..
        } => match name.namespace.as_deref() == Some(ns)
            && name.local_name == local
        {
            true => Ok(((name, attributes), xml)),
            false => Err(ParseError::UnexpectedTag(name.to_string())),
        },
        _ => Err(ParseError::Unexpected(event.clone())),
    }
}

pub fn any_start_tag(
    xml: XmlInput<'_>,
) -> ParseResult<'_, (&OwnedName, &[OwnedAttribute])> {
    let (event, xml) = next(xml)?;
    match event {
        XmlEvent::StartElement {
            name, attributes, ..
        } => Ok(((name, attributes), xml)),
        _ => Err(ParseError::Unexpected(event.clone())),
    }
}

pub fn end_tag<'a>(
    xml: XmlInput<'a>,
    start_tag: &OwnedName,
) -> ParseResult<'a, ()> {
    let (event, xml) = next(xml)?;
    match event {
        XmlEvent::EndElement { name } => match name == start_tag {
            true => Ok(((), xml)),
            false => Err(ParseError::UnexpectedEndTag(name.to_string())),
        },
        _ => Err(ParseError::Unexpected(event.clone())),
    }
}

pub fn characters(xml: XmlInput<'_>) -> ParseResult<'_, &str> {
    let (event, xml) = next(xml)?;
    match event {
        XmlEvent::Characters(s) => Ok((s.as_str(), xml)),
        _ => Err(ParseError::Unexpected(event.clone())),
    }
}

pub fn ignore_until_end_tag<'a>(
    mut xml: XmlInput<'a>,
    tag: &OwnedName,
) -> ParseResult<'a, ()> {
    loop {
        let (event, next) = next(xml)?;
        xml = next;
        match event {
            XmlEvent::EndElement { name } => match name == tag {
                true => return Ok(((), xml)),
                false => {
                    return Err(ParseError::UnexpectedEndTag(name.to_string()))
                }
            },
            XmlEvent::EndDocument => return Err(ParseError::Eof),
            XmlEvent::StartElement { name, .. } => {
                let (_, next) = ignore_until_end_tag(next, name)?;
                xml = next;
            }
            _ => {}
        }
    }
}

pub fn ignore_spaces(mut xml: XmlInput) -> ParseResult<()> {
    while let Ok((event, next)) = next(xml) {
        match event {
            XmlEvent::ProcessingInstruction { .. }
            | XmlEvent::Comment(_)
            | XmlEvent::Whitespace(_) => {}
            _ => break,
        }
        xml = next;
    }
    Ok(((), xml))
}

pub fn next(xml: XmlInput) -> ParseResult<&XmlEvent> {
    match xml.first() {
        Some(event) => Ok((event, &xml[1..])),
        None => Err(ParseError::Eof),
    }
}

pub fn many<F: FnMut(XmlInput) -> ParseResult<R>, R>(
    mut elem: F,
) -> impl FnMut(XmlInput) -> ParseResult<Vec<R>> {
    move |mut xml| {
        let mut es = Vec::new();
        loop {
            match elem(xml) {
                Ok((e, next)) => {
                    es.push(e);
                    xml = next;
                }
                Err(ParseError::Syntax(s)) => {
                    return Err(ParseError::Syntax(s))
                }
                Err(_) => break,
            }
        }
        Ok((es, xml))
    }
}

pub fn optional<'a, F: FnMut(XmlInput<'a>) -> ParseResult<'a, R>, R: 'a>(
    mut parser: F,
) -> impl FnMut(XmlInput<'a>) -> ParseResult<'a, Option<R>> {
    move |xml| match parser(xml) {
        Ok((value, xml)) => Ok((Some(value), xml)),
        Err(ParseError::Syntax(s)) => Err(ParseError::Syntax(s)),
        Err(_) => Ok((None, xml)),
    }
}
