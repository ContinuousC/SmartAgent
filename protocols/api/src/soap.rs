/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::path::PathBuf;

use log::info;
use minidom::Element;
use reqwest::{
    header::{HeaderMap, HeaderValue},
    Certificate, Client,
};
use serde::{Deserialize, Serialize};
use tokio::fs;

#[derive(Debug)]
pub struct SoapClient {
    endpoint: String,
    client: Client,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CertType {
    PEM,
    DER,
}

impl SoapClient {
    fn envelope(body: String) -> String {
        format!(
            r#"<SOAP-ENV:Envelope
						xmlns:SOAP-ENC="http://schemas.xmlsoap.org/soap/encoding/"
						xmlns:SOAP-ENV="http://schemas.xmlsoap.org/soap/envelope/"
						xmlns:ZSI="http://www.zolera.com/schemas/ZSI/"
						xmlns:soapenc="http://schemas.xmlsoap.org/soap/encoding/"
						xmlns:soapenv="http://schemas.xmlsoap.org/soap/envelope/"
						xmlns:xsd="http://www.w3.org/2001/XMLSchema"
						xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
					<SOAP-ENV:Header></SOAP-ENV:Header>
					{}
				</SOAP-ENV:Envelope>"#,
            body
        )
    }

    pub async fn create(
        endpoint: String,
        mut headers: HeaderMap,
        certificate: Option<&(CertType, PathBuf)>,
        disable_certificate_verification: bool,
        disable_hostname_verification: bool,
    ) -> Result<SoapClient, SoapError> {
        headers.insert(
            "Content-Type",
            HeaderValue::from_static("text/xml; charset=\"utf-8\""),
        );
        let mut client = Client::builder()
            .user_agent("SmartAgent")
            .default_headers(headers)
            .cookie_store(true)
            .danger_accept_invalid_certs(disable_certificate_verification)
            .danger_accept_invalid_hostnames(disable_hostname_verification);

        if let Some((cert_type, cert_path)) = certificate {
            info!("loading certificate ({:?}): {:?}", cert_type, cert_path);
            let cert = fs::read(cert_path).await?;
            client = client.add_root_certificate(match cert_type {
                CertType::PEM => Certificate::from_pem(&cert)?,
                CertType::DER => Certificate::from_der(&cert)?,
            });
        }
        let client = client.build()?;

        Ok(SoapClient { client, endpoint })
    }

    pub async fn request(&self, body: String) -> Result<String, SoapError> {
        let body = SoapClient::envelope(body);
        let response =
            self.client.post(&self.endpoint).body(body).send().await?;
        Ok(response.text().await?)
    }
}

pub fn get_child_as_string(
    elem: &Element,
    ns: String,
    child: String,
) -> Result<value::Value, SoapError> {
    Ok(value::Value::UnicodeString(
        elem.get_child(&child, ns.as_str())
            .ok_or(SoapError::XMLChildNotFound(child))?
            .text(),
    ))
}

pub fn get_child_as_int(
    elem: &Element,
    ns: String,
    child: String,
) -> Result<value::Value, SoapError> {
    Ok(value::Value::Integer(
        elem.get_child(&child, ns.as_str())
            .ok_or(SoapError::XMLChildNotFound(child.clone()))?
            .text()
            .parse::<i64>()
            .map_or_else(
                |_e| {
                    Err(SoapError::XMLParseValue(
                        child,
                        elem.text(),
                        String::from("i64"),
                    ))
                },
                Ok,
            )?,
    ))
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Value<T> {
    #[serde(rename = "$value")]
    pub data: T,
}

#[derive(thiserror::Error, Debug)]
pub enum SoapError {
    #[error("Template could not be filled: {0}")]
    TemplateRenderError(Box<handlebars::RenderError>),
    #[error("XML could not be deserialized: {0}")]
    XMLDeserializeError(#[from] serde_xml_rs::Error),
    #[error("XML could not be parsed: {0}")]
    XMLParseError(#[from] minidom::Error),
    #[error("Child {0} not present in xml")]
    XMLChildNotFound(String),
    #[error("Attribute {0} not present in node")]
    XMLAttrributeNotFound(String),
    #[error("Cannot parse {0} with value {1} to {2}")]
    XMLParseValue(String, String, String),
    #[error("Request to host failed: {0}")]
    ReqwestError(#[from] reqwest::Error),
    #[error("{0:?}")]
    IO(#[from] std::io::Error),
}

impl From<handlebars::RenderError> for SoapError {
    fn from(value: handlebars::RenderError) -> Self {
        Self::TemplateRenderError(Box::new(value))
    }
}
