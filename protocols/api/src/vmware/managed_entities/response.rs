/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::collections::{hash_map::Entry, HashMap};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use xml::{attribute::OwnedAttribute, name::OwnedName, reader::XmlEvent};

use crate::vmware::managed_entities::{
    error::{ParseError, ParseResult},
    from_xml::{
        any_start_tag, characters, end_document, end_tag, ignore_spaces,
        ignore_until_end_tag, many, next, optional, start_document, start_tag,
        FromXml, XmlInput,
    },
};

#[derive(Serialize, Debug)]
pub struct Document<T> {
    pub content: T,
}

#[derive(Serialize, Debug)]
pub struct Envelope<T> {
    pub body: T,
}

#[derive(Serialize, Debug)]
pub struct RetrievePropertiesExResponse {
    pub objects: Vec<Object>,
}

#[derive(Serialize, Debug)]
pub struct Object {
    pub r#type: Option<String>,
    pub id: String,
    pub props: Vec<PropSet>,
}

#[derive(Serialize, Debug)]
pub struct PropSet {
    pub name: String,
    pub val: Option<Value>,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Value {
    String(String),
    Integer(i64),
    Boolean(bool),
    DateTime(DateTime<Utc>),
    ArrayOfString(Vec<String>),
    ArrayOfManagedObjectReference(Vec<ManagedObjectReference>),
    ManagedObjectReference(ManagedObjectReference),
    Generic(GenericValue),
    Unimplemented(String),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum GenericValue {
    Object(GenericObject),
    Array(GenericArray),
    String(String),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GenericObject(HashMap<String, GenericValue>);

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GenericArray(Vec<GenericValue>);

#[derive(Serialize, Deserialize, Debug)]
pub struct ManagedObjectReference {
    pub r#type: Option<String>,
    pub id: String,
}

impl<T: FromXml> FromXml for Document<T> {
    fn from_xml(xml: XmlInput) -> ParseResult<Self> {
        let (_, xml) = ignore_spaces(xml)?;
        let (_, xml) = start_document(xml)?;
        let (content, xml) = T::from_xml(xml)?;
        let (_, xml) = ignore_spaces(xml)?;
        let (_, xml) = end_document(xml)?;
        Ok((Document { content }, xml))
    }
}

impl<T: FromXml> FromXml for Envelope<T> {
    fn from_xml(xml: XmlInput) -> ParseResult<Self> {
        let (_, xml) = ignore_spaces(xml)?;
        let ((envelope_tag, _attrs), xml) = start_tag(
            xml,
            "http://schemas.xmlsoap.org/soap/envelope/",
            "Envelope",
        )?;
        let (_, xml) = ignore_spaces(xml)?;
        let ((body_tag, _attrs), xml) = start_tag(
            xml,
            "http://schemas.xmlsoap.org/soap/envelope/",
            "Body",
        )?;
        let (body, xml) = T::from_xml(xml)?;
        let (_, xml) = ignore_spaces(xml)?;
        let (_, xml) = end_tag(xml, body_tag)?;
        let (_, xml) = ignore_spaces(xml)?;
        let (_, xml) = end_tag(xml, envelope_tag)?;
        Ok((Envelope { body }, xml))
    }
}

impl FromXml for RetrievePropertiesExResponse {
    fn from_xml(xml: XmlInput) -> ParseResult<Self> {
        let (_, xml) = ignore_spaces(xml)?;
        let ((tag, _attrs), xml) =
            start_tag(xml, "urn:vim25", "RetrievePropertiesExResponse")?;
        let (_, xml) = ignore_spaces(xml)?;
        let ((returnval_tag, _), xml) =
            start_tag(xml, "urn:vim25", "returnval")
                .map_err(|_| ParseError::Syntax("missing returnval"))?;
        let (objects, xml) = many(Object::from_xml)(xml)?;
        let (_, xml) = ignore_spaces(xml)?;
        let (_, xml) = end_tag(xml, returnval_tag)
            .map_err(|_| ParseError::Syntax("missing returnval end tag"))?;
        let (_, xml) = ignore_spaces(xml)?;
        let (_, xml) = end_tag(xml, tag).map_err(|_| {
            ParseError::Syntax("missing RetrievePropertiesExResponse end tag")
        })?;
        Ok((RetrievePropertiesExResponse { objects }, xml))
    }
}

impl FromXml for Object {
    fn from_xml(xml: XmlInput) -> ParseResult<Self> {
        let (_, xml) = ignore_spaces(xml)?;
        let ((tag, _attrs), xml) = start_tag(xml, "urn:vim25", "objects")?;
        let (_, xml) = ignore_spaces(xml)?;

        let ((obj_tag, obj_attrs), xml) = start_tag(xml, "urn:vim25", "obj")
            .map_err(|_| ParseError::Syntax("missing obj tag"))?;
        let r#type = obj_attrs
            .iter()
            .find(|attr| {
                attr.name.namespace.as_deref() == Some("urn:vim25")
                    && attr.name.local_name == "type"
            })
            //.ok_or(ParseError::Syntax("missing 'type'"))?
            .map(|attr| attr.value.to_string());
        let (_, xml) = ignore_spaces(xml)?;
        let (id, xml) = characters(xml)
            .map_err(|_| ParseError::Syntax("missing obj id"))?;
        let (_, xml) = ignore_spaces(xml)?;
        let (_, xml) = end_tag(xml, obj_tag)
            .map_err(|_| ParseError::Syntax("missing obj end tag"))?;
        let (props, xml) = many(PropSet::from_xml)(xml)?;
        let (_, xml) = ignore_spaces(xml)?;
        let (_, xml) = end_tag(xml, tag)
            .map_err(|_| ParseError::Syntax("missing objects end tag"))?;
        Ok((
            Object {
                r#type,
                id: id.to_string(),
                props,
            },
            xml,
        ))
    }
}

impl FromXml for PropSet {
    fn from_xml(xml: XmlInput) -> ParseResult<Self> {
        let (_, xml) = ignore_spaces(xml)?;
        let ((tag, _attrs), xml) = start_tag(xml, "urn:vim25", "propSet")?;
        let (_, xml) = ignore_spaces(xml)?;

        let ((name_tag, _attrs), xml) = start_tag(xml, "urn:vim25", "name")
            .map_err(|_| ParseError::Syntax("missing propSet name"))?;
        let (_, xml) = ignore_spaces(xml)?;
        let (name, xml) = characters(xml)
            .map_err(|_| ParseError::Syntax("missing propSet name"))?;
        let (_, xml) = ignore_spaces(xml)?;
        let (_, xml) = end_tag(xml, name_tag)
            .map_err(|_| ParseError::Syntax("missing propSet name end tag"))?;

        let (val, xml) =
            optional(Value::from_xml)(xml).map_err(|e| match e {
                ParseError::Syntax(s) => ParseError::Syntax(s),
                _ => ParseError::Syntax("missing propSet value"),
            })?;

        let (_, xml) = ignore_spaces(xml)?;
        let (_, xml) = end_tag(xml, tag)
            .map_err(|_| ParseError::Syntax("missing propSet end tag"))?;
        Ok((
            PropSet {
                name: name.to_string(),
                val,
            },
            xml,
        ))
    }
}

impl FromXml for Value {
    fn from_xml(xml: XmlInput) -> ParseResult<Self> {
        let (_, xml) = ignore_spaces(xml)?;
        let ((val_tag, val_attrs), xml) = start_tag(xml, "urn:vim25", "val")?;
        let r#type = val_attrs
            .iter()
            .find(|attr| {
                attr.name.namespace.as_deref()
                    == Some("http://www.w3.org/2001/XMLSchema-instance")
                    && attr.name.local_name == "type"
            })
            .map(|attr| attr.value.to_string())
            .ok_or(ParseError::Syntax("missing value type"))?;
        let (res, xml) = match r#type.as_str() {
            "xsd:string" => {
                let (_, xml) = ignore_spaces(xml)?;
                let (s, xml) = optional(characters)(xml)?;
                (Value::String(s.unwrap_or("").to_string()), xml)
            }
            "xsd:int" | "xsd:short" | "xsd:long" => {
                let (_, xml) = ignore_spaces(xml)?;
                let (s, xml) = characters(xml).map_err(|_| {
                    ParseError::Syntax("missing value for int value")
                })?;
                (
                    Value::Integer(s.parse().map_err(|_| {
                        ParseError::Syntax("invalid value for int value")
                    })?),
                    xml,
                )
            }
            "xsd:boolean" => {
                let (_, xml) = ignore_spaces(xml)?;
                let (s, xml) = characters(xml).map_err(|_| {
                    ParseError::Syntax("missing value for boolean value")
                })?;
                (
                    Value::Boolean(match s {
                        "true" => Ok(true),
                        "false" => Ok(false),
                        _ => {
                            Err(ParseError::Syntax("invalid value for boolean"))
                        }
                    }?),
                    xml,
                )
            }
            "xsd:dateTime" => {
                let (_, xml) = ignore_spaces(xml)?;
                let (s, xml) = characters(xml).map_err(|_| {
                    ParseError::Syntax("missing value for dateTime value")
                })?;
                (
                    Value::DateTime(
                        DateTime::parse_from_rfc3339(s)
                            .map_err(|_| {
                                ParseError::Syntax("invalid value for dateTime")
                            })?
                            .into(),
                    ),
                    xml,
                )
            }
            "ArrayOfString" => {
                let (_, xml) = ignore_spaces(xml)?;
                let (vals, xml) = many(|xml| {
                    let (_, xml) = ignore_spaces(xml)?;
                    let ((string_tag, _attrs), xml) =
                        start_tag(xml, "urn:vim25", "string")?;
                    let (_, xml) = ignore_spaces(xml)?;
                    let (val, xml) = optional(characters)(xml)?;
                    let (_, xml) = ignore_spaces(xml)?;
                    let (_, xml) = end_tag(xml, string_tag).map_err(|_| {
                        ParseError::Syntax("missing end tag for string value")
                    })?;
                    Ok((val.unwrap_or("").to_string(), xml))
                })(xml)?;
                (Value::ArrayOfString(vals), xml)
            }
            "ArrayOfManagedObjectReference" => {
                let (_, xml) = ignore_spaces(xml)?;
                let (refs, xml) = many(ManagedObjectReference::from_xml)(xml)
                    .map_err(|_| ParseError::Syntax("many"))?;
                (Value::ArrayOfManagedObjectReference(refs), xml)
            }
            "ManagedObjectReference" => {
                let (_, xml) = ignore_spaces(xml)?;
                let (res, xml) =
                    ManagedObjectReference::from_xml_inner(xml, val_attrs)?;
                (Value::ManagedObjectReference(res), xml)
            }
            _ => match GenericValue::from_xml(xml) {
                Ok((val, xml)) => (Value::Generic(val), xml),
                Err(_) => {
                    let (_, xml) =
                        ignore_until_end_tag(xml, val_tag).map_err(|_| {
                            ParseError::Syntax("missing value end tag")
                        })?;
                    eprintln!("unimplemented type: {}", r#type);
                    return Ok((Value::Unimplemented(r#type.to_string()), xml));
                }
            },
        };

        let (_, xml) = ignore_spaces(xml)?;
        let (_, xml) = end_tag(xml, val_tag)
            .map_err(|_| ParseError::Syntax("missing value end tag"))?;
        Ok((res, xml))
    }
}

impl FromXml for ManagedObjectReference {
    fn from_xml(xml: XmlInput) -> ParseResult<Self> {
        let (_, xml) = ignore_spaces(xml)?;
        let ((tag, attrs), xml) =
            start_tag(xml, "urn:vim25", "ManagedObjectReference")?;
        let (res, xml) = Self::from_xml_inner(xml, attrs)?;
        let (_, xml) = ignore_spaces(xml)?;
        let (_, xml) = end_tag(xml, tag)?;
        Ok((res, xml))
    }
}

impl ManagedObjectReference {
    fn from_xml_inner<'a>(
        xml: XmlInput<'a>,
        attrs: &'a [OwnedAttribute],
    ) -> ParseResult<'a, Self> {
        let r#type = attrs
            .iter()
            .find(|attr| {
                attr.name.namespace.as_deref() == Some("urn:vim25")
                    && attr.name.local_name == "type"
            })
            .map(|attr| attr.value.to_string());
        //.ok_or(ParseError::Syntax("missing ManagedObjectReference type"))?;

        let (_, xml) = ignore_spaces(xml)?;
        let (id, xml) = characters(xml)?;

        Ok((
            ManagedObjectReference {
                r#type,
                id: id.to_string(),
            },
            xml,
        ))
    }
}

impl FromXml for GenericValue {
    fn from_xml(xml: XmlInput) -> ParseResult<Self> {
        let (_, xml) = ignore_spaces(xml)?;

        match next(xml)? {
            // Next event is a tag: object or array.
            (xml::reader::XmlEvent::StartElement { name, .. }, _) => {
                match GenericObject::from_xml(xml) {
                    Ok((val, xml)) => Ok((GenericValue::Object(val), xml)),
                    Err(_) => {
                        let (val, xml) = GenericArray::from_xml_tag(xml, name)?;
                        Ok((GenericValue::Array(val), xml))
                    }
                }
            }
            // Empty tag contents: can be anything; we'll return an empty string
            (xml::reader::XmlEvent::EndElement { .. }, _) => {
                Ok((GenericValue::String(String::from("")), xml))
            }
            // Characters: this must be a (type parseable from) string
            (xml::reader::XmlEvent::Characters(s), xml) => {
                Ok((GenericValue::String(s.to_string()), xml))
            }
            (ev, _) => Err(ParseError::Unexpected(ev.clone())),
        }
    }
}

impl FromXml for GenericArray {
    fn from_xml(xml: XmlInput) -> ParseResult<Self> {
        let (_, xml) = ignore_spaces(xml)?;
        match next(xml)? {
            (XmlEvent::StartElement { name, .. }, _) => {
                Self::from_xml_tag(xml, name)
            }
            (ev, _) => Err(ParseError::Unexpected(ev.clone())),
        }
    }
}

impl GenericArray {
    fn from_xml_tag<'a>(
        xml: XmlInput<'a>,
        name: &'a OwnedName,
    ) -> ParseResult<'a, Self> {
        let (vals, xml) = many(|xml| {
            let (_, xml) = ignore_spaces(xml)?;
            let ((tag, _), xml) = start_tag(
                xml,
                name.namespace.as_deref().unwrap_or(""),
                name.local_name.as_str(),
            )?;
            let (val, xml) = GenericValue::from_xml(xml)?;
            let (_, xml) = ignore_spaces(xml)?;
            let (_, xml) = end_tag(xml, tag)?;
            Ok((val, xml))
        })(xml)?;

        let (_, xml) = ignore_spaces(xml)?;
        match next(xml)? {
            (XmlEvent::EndElement { .. }, _) => Ok((GenericArray(vals), xml)),
            (ev, _) => Err(ParseError::Unexpected(ev.clone())),
        }
    }
}

impl FromXml for GenericObject {
    fn from_xml(mut xml: XmlInput) -> ParseResult<Self> {
        let mut vals = HashMap::new();
        loop {
            let (_, ixml) = ignore_spaces(xml)?;
            match any_start_tag(ixml) {
                Ok(((tag, _), ixml)) => {
                    let (val, ixml) = GenericValue::from_xml(ixml)?;
                    let (_, ixml) = end_tag(ixml, tag)?;
                    match vals.entry(tag.local_name.to_string()) {
                        Entry::Occupied(mut ent) => {
                            if let GenericValue::Array(vals) = ent.get_mut() {
                                vals.0.push(val);
                            } else {
                                ent.insert(GenericValue::Array(GenericArray(
                                    vec![ent.get().clone(), val],
                                )));
                            }
                        }
                        Entry::Vacant(ent) => {
                            ent.insert(val);
                        }
                    }
                    xml = ixml;
                }
                Err(_) => {
                    let (_, ixml) = ignore_spaces(ixml)?;
                    return match next(xml)? {
                        (XmlEvent::EndElement { .. }, _) => {
                            Ok((GenericObject(vals), ixml))
                        }
                        (ev, _) => Err(ParseError::Unexpected(ev.clone())),
                    };
                }
            }
        }
    }
}
