/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::Arc;

use tokio::io::{AsyncRead, AsyncWrite};
use tokio_rustls::rustls::{
    self, AllowAnyAuthenticatedClient, ServerConfig, Session,
};
use tokio_rustls::server::TlsStream;
use x509_parser::parse_x509_certificate;

use super::error::Result;

pub fn make_tls_config(
    ca_path: &Path,
    cert_path: &Path,
    key_path: &Path,
) -> Result<Arc<ServerConfig>> {
    let mut root_certs = rustls::RootCertStore::empty();
    let mut ca_file = BufReader::new(File::open(ca_path)?);
    root_certs.add_pem_file(&mut ca_file).unwrap();

    let verifier = AllowAnyAuthenticatedClient::new(root_certs);
    let mut tls_config = ServerConfig::new(verifier);

    let mut key_file = BufReader::new(File::open(key_path)?);
    let key = rustls::internal::pemfile::pkcs8_private_keys(&mut key_file)
        .unwrap()
        .into_iter()
        .next()
        .unwrap();

    let certs = [cert_path, ca_path]
        .iter()
        .map(|path| {
            let mut cert_file = BufReader::new(File::open(path)?);
            Ok(rustls::internal::pemfile::certs(&mut cert_file).unwrap())
        })
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

    tls_config.set_single_cert(certs, key)?;
    Ok(Arc::new(tls_config))
}

pub fn get_certificate_ids<S>(stream: &TlsStream<S>) -> Result<(String, String)>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let peer_cert_data = stream
        .get_ref()
        .1
        .get_peer_certificates()
        .ok_or(api::Error::AuthenticationFailed)?
        .into_iter()
        .next()
        .ok_or(api::Error::AuthenticationFailed)?;
    let peer_cert = parse_x509_certificate(&peer_cert_data.0)
        .map_err(|_| api::Error::AuthenticationFailed)?
        .1;

    let mut orgs = peer_cert.subject().iter_organization();
    let org = orgs
        .next()
        .ok_or(api::Error::AuthenticationFailed)?
        .attr_value
        .as_str()
        .map_err(|_| api::Error::AuthenticationFailed)?
        .to_string();

    let mut names = peer_cert.subject().iter_common_name();
    let name = names
        .next()
        .ok_or(api::Error::AuthenticationFailed)?
        .attr_value
        .as_str()
        .map_err(|_| api::Error::AuthenticationFailed)?
        .to_string();

    Ok((org, name))
}
