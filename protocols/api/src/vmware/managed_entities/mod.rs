/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

pub(crate) mod error;
pub(crate) mod from_xml;
pub(crate) mod request;
pub(crate) mod response;

use std::collections::{HashMap, HashSet};

use crate::soap::SoapClient;

use self::{
    from_xml::FromXml,
    request::ManagedEntityRequest,
    response::{Document, Envelope, RetrievePropertiesExResponse},
};

use super::{
    cc_config::EssentialConfig,
    error::{Error, Result},
    requests::{LoginRequest, SysteminfoRequest},
};
use reqwest::header::{HeaderMap, HeaderValue};
use response::Value;

pub async fn get_managed_entities(
    host: String,
    config: EssentialConfig,
    entity_type: String,
    properties: HashSet<String>,
) -> Result<HashMap<String, HashMap<String, Option<Value>>>> {
    let config = config.into_omd_config(host);

    let endpoint = format!(
        "https://{}:{}/sdk",
        config.get_hostname().await?,
        config.port.unwrap_or(443),
    );

    let mut headers = HeaderMap::new();
    headers.insert("SOAPAction", HeaderValue::from_static("urn:vim25/5.0"));

    let soapclient = SoapClient::create(
        endpoint,
        headers,
        config.certificate.as_ref(),
        config.disable_certificate_verification.unwrap_or(false),
        config.disable_hostname_verification.unwrap_or(false),
    )
    .await?;

    let sysinfo = SysteminfoRequest::new(&soapclient, &HashMap::new())
        .await
        .map_err(Error::SoapError)?;

    if let Some(creds) = &config.credentials {
        let mut args = sysinfo.to_hashmap();
        args.insert(String::from("username"), creds.username.to_string());
        args.insert(
            String::from("password"),
            creds
                .password
                .as_ref()
                .ok_or(Error::MissingKRObject(String::from("Password")))?
                .to_string(),
        );

        LoginRequest::new(&soapclient, &args)
            .await
            .map_err(Error::Login)?;
    }

    let req = ManagedEntityRequest::new(
        &sysinfo.body.response.returnval.property_collector.data,
        &sysinfo.body.response.returnval.root_folder.data,
        &entity_type,
        properties
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .as_slice(),
    )
    .to_string()
    .map_err(Error::GenerateRequest)?;

    log::debug!("sending request: {}", req);

    let data = soapclient.request(req).await?;
    let xml = read_events(xml::reader::EventReader::from_str(&data))
        .map_err(Error::ParseResponseXml)?;
    let (res, _rest) =
        match Document::<Envelope<RetrievePropertiesExResponse>>::from_xml(&xml)
        {
            Ok(r) => r,
            Err(e) => {
                log::debug!("failed to parse xml: {}", data);
                return Err(Error::ParseResponse(e));
            }
        };

    Ok(res
        .content
        .body
        .objects
        .into_iter()
        .map(|obj| {
            (
                obj.id,
                obj.props
                    .into_iter()
                    .map(|p| (p.name, p.val))
                    .collect::<HashMap<_, _>>(),
            )
        })
        .collect::<HashMap<_, _>>())
}

fn read_events<R: std::io::Read>(
    mut xml: xml::reader::EventReader<R>,
) -> xml::reader::Result<Vec<xml::reader::XmlEvent>> {
    let mut elems = Vec::new();
    loop {
        let event = xml.next()?;
        match event {
            xml::reader::XmlEvent::EndDocument => {
                elems.push(event);
                break;
            }
            _ => {
                elems.push(event);
            }
        }
    }
    Ok(elems)
}
