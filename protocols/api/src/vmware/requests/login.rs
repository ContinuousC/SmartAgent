/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use handlebars::Handlebars;
use serde::{Deserialize, Serialize};

use crate::soap::{SoapClient, SoapError, Value};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LoginRequest {
    #[serde(rename = "Body")]
    body: Body,
}

impl LoginRequest {
    pub async fn new(
        client: &SoapClient,
        args: &HashMap<String, String>,
    ) -> Result<LoginRequest, SoapError> {
        let template = r#"<SOAP-ENV:Body xmlns:ns1="urn:vim25">
							<ns1:Login xsi:type="ns1:LoginRequestType">
								<ns1:_this type="SessionManager">{{session_manager}}</ns1:_this>
								<ns1:userName>{{username}}</ns1:userName>
								<ns1:password>{{password}}</ns1:password>
							</ns1:Login>
						</SOAP-ENV:Body>"#
            .to_string();
        let body = Handlebars::new().render_template(&template, &args)?;
        let response = &client.request(body).await?;
        // println!("login response: {}", &response);
        let request: LoginRequest = serde_xml_rs::from_str(response)?;
        Ok(request)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Body {
    #[serde(rename = "LoginResponse")]
    response: Response,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Response {
    returnval: ReturnValue,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct ReturnValue {
    key: Value<String>,
    #[serde(rename = "userName")]
    username: Value<String>,
    #[serde(rename = "fullName")]
    fullname: Value<Option<String>>,
    #[serde(rename = "loginTime")]
    login_time: Value<DateTime<Utc>>,
    #[serde(rename = "lastActiveTime")]
    last_active_time: Value<DateTime<Utc>>,
    locale: Value<String>,
    #[serde(rename = "messageLocale")]
    message_locale: Value<String>,
    #[serde(rename = "extensionSession")]
    extension_session: Value<bool>,
}
