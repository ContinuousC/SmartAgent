/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::io::Write;

use xml::{writer::XmlEvent, EmitterConfig};

pub(crate) struct ManagedEntityRequest<'a> {
    property_collector: &'a str,
    root_folder: &'a str,
    managed_entity: &'a str,
    properties: &'a [&'a str],
}

impl<'a> ManagedEntityRequest<'a> {
    pub(crate) fn new(
        property_collector: &'a str,
        root_folder: &'a str,
        managed_entity: &'a str,
        properties: &'a [&'a str],
    ) -> Self {
        Self {
            property_collector,
            root_folder,
            managed_entity,
            properties,
        }
    }

    pub(crate) fn to_string(&self) -> xml::writer::Result<String> {
        let mut xml = xml::writer::EventWriter::new_with_config(
            Vec::new(),
            EmitterConfig::new(), //.perform_indent(true),
        );
        self.to_xml(&mut xml)?;
        let res = String::from_utf8_lossy(&xml.into_inner()).to_string();

        // strip doc declaration for co-operation with soap client
        match res.strip_prefix("<?xml version=\"1.0\" encoding=\"utf-8\"?>") {
            Some(s) => Ok(s.to_string()),
            None => Ok(res),
        }
    }

    pub(crate) fn to_xml<W: Write>(
        &self,
        xml: &mut xml::writer::EventWriter<W>,
    ) -> xml::writer::Result<()> {
        // xml.write(XmlEvent::StartDocument {
        //     version: XmlVersion::Version10,
        //     encoding: Some("UTF-8"),
        //     standalone: None,
        // })?;

        xml.write(
            XmlEvent::start_element("SOAP-ENV:Body").ns("ns1", "urn:vim25"),
        )?;
        xml.write(
            XmlEvent::start_element("ns1:RetrievePropertiesEx")
                .attr("xsi:type", "ns1:RetrievePropertiesExRequestType"),
        )?;

        xml.write(
            XmlEvent::start_element("ns1:_this")
                .attr("type", "PropertyCollector"),
        )?;
        xml.write(XmlEvent::characters(self.property_collector))?;
        xml.write(XmlEvent::end_element())?; // ns1:_this

        xml.write(XmlEvent::start_element("ns1:specSet"))?;
        xml.write(XmlEvent::start_element("ns1:propSet"))?;
        simple_elem(xml, "ns1:type", self.managed_entity)?;

        for prop in self.properties {
            simple_elem(xml, "ns1:pathSet", prop)?;
        }

        xml.write(XmlEvent::end_element())?; // ns1:propSet

        default_object_set(xml, self.root_folder)?;

        xml.write(XmlEvent::end_element())?; // ns1:specSet
        xml.write(XmlEvent::start_element("ns1:options"))?;
        xml.write(XmlEvent::end_element())?; // ns1:options
        xml.write(XmlEvent::end_element())?; // ns1:RetrievePropertiesEx
        xml.write(XmlEvent::end_element())?; // SOAP-ENV:Body

        Ok(())
    }
}

fn default_object_set<W: Write>(
    xml: &mut xml::writer::EventWriter<W>,
    root_folder: &str,
) -> xml::writer::Result<()> {
    xml.write(XmlEvent::start_element("ns1:objectSet"))?;
    obj(xml, "Folder", root_folder)?;
    simple_elem(xml, "ns1:skip", "false")?;
    traversal_spec(
        xml,
        "visitFolders",
        "Folder",
        "childEntity",
        &[
            "visitFolders",
            "dcToHf",
            "dcToVmf",
            "crToH",
            "crToRp",
            "dcToDs",
            "hToVm",
            "rpToVm",
        ],
    )?;
    traversal_spec(
        xml,
        "dcToVmf",
        "Datacenter",
        "vmFolder",
        &["visitFolders"],
    )?;
    traversal_spec(
        xml,
        "dcToDs",
        "Datacenter",
        "datastore",
        &["visitFolders"],
    )?;
    traversal_spec(
        xml,
        "dcToHf",
        "Datacenter",
        "hostFolder",
        &["visitFolders"],
    )?;
    traversal_spec(xml, "crToH", "ComputeResource", "host", &[])?;
    traversal_spec(
        xml,
        "crToRp",
        "ComputeResource",
        "resourcePool",
        &["rpToRp", "rpToVm"],
    )?;
    traversal_spec(
        xml,
        "rpToRp",
        "ResourcePool",
        "resourcePool",
        &["rpToRp", "rpToVm"],
    )?;
    traversal_spec(xml, "hToVm", "HostSystem", "vm", &["visitFolders"])?;
    traversal_spec(xml, "rpToVm", "ResourcePool", "vm", &[])?;
    xml.write(XmlEvent::end_element())
}

fn traversal_spec<W: Write>(
    xml: &mut xml::writer::EventWriter<W>,
    name: &str,
    typ: &str,
    path: &str,
    select_sets: &[&str],
) -> xml::writer::Result<()> {
    xml.write(
        XmlEvent::start_element("ns1:selectSet")
            .attr("xsi:type", "ns1:TraversalSpec"),
    )?;
    simple_elem(xml, "ns1:name", name)?;
    simple_elem(xml, "ns1:type", typ)?;
    simple_elem(xml, "ns1:path", path)?;
    simple_elem(xml, "ns1:skip", "false")?;
    for set in select_sets {
        select_set(xml, set)?;
    }
    xml.write(XmlEvent::end_element())
}

fn select_set<W: Write>(
    xml: &mut xml::writer::EventWriter<W>,
    name: &str,
) -> xml::writer::Result<()> {
    xml.write(XmlEvent::start_element("ns1:selectSet"))?;
    xml.write(XmlEvent::start_element("ns1:name"))?;
    xml.write(XmlEvent::characters(name))?;
    xml.write(XmlEvent::end_element())?;
    xml.write(XmlEvent::end_element())
}

fn obj<W: Write>(
    xml: &mut xml::writer::EventWriter<W>,
    typ: &str,
    name: &str,
) -> xml::writer::Result<()> {
    xml.write(XmlEvent::start_element("ns1:obj").attr("type", typ))?;
    xml.write(XmlEvent::characters(name))?;
    xml.write(XmlEvent::end_element())
}

fn simple_elem<W: Write>(
    xml: &mut xml::writer::EventWriter<W>,
    name: &str,
    data: &str,
) -> xml::writer::Result<()> {
    xml.write(XmlEvent::start_element(name))?;
    xml.write(XmlEvent::characters(data))?;
    xml.write(XmlEvent::end_element())
}
