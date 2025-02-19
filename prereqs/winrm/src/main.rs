/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::collections::{HashMap, HashSet};

use clap::Parser;
use colored::Colorize;
use futures::{stream, StreamExt};
use log::{debug, error, info};
use tokio::fs;

use winrm_rs::session::{Session, SessionBuilder};

use winrm_prereqs::{
    args::{Args, WmiMethod},
    Error, Result, TestResult,
};

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    args.init_logger();

    // calculate hosts
    let hosts: HashSet<String> = args.get_hosts().await?;
    info!("Checking prereqs for hosts: {:?}", &hosts);

    info!("scheduling {} hosts", hosts.len());
    let results =
        stream::iter(hosts.iter().map(|h| test_host(h.clone(), &args)))
            .buffer_unordered(10)
            .collect::<Vec<TestResult>>()
            .await;
    info!("All hosts have been tested");

    for result in &results {
        result.log_outcome();
    }

    info!(
        "Summary: {}, {}",
        format!(
            "{} hosts ok",
            results.iter().filter(|r| r.is_success()).count()
        )
        .green(),
        format!(
            "{} hosts failed",
            results.iter().filter(|r| !r.is_success()).count()
        )
        .red(),
    );
    Ok(())
}

async fn test_host(hostname: String, args: &Args) -> TestResult {
    let mut session = match create_session(&hostname, args).await {
        Ok(s) => s,
        Err(e) => {
            error!(
                "Could not create a session to test host {}: {:?}",
                &hostname, e
            );
            return TestResult::new(
                hostname,
                Some(String::from("Session creation")),
                Err(e),
            );
        }
    };
    debug!("Session created for: {}", &hostname);

    let (out, res) = if let Some(script) = args.ps_script.as_ref() {
        script.test(&mut session).await
    } else if let Some(script) = args.cmd_script.as_ref() {
        script.test(&mut session).await
    } else {
        test_host_wmi(session, args).await
    };

    if res.is_ok() && args.print_stdout {
        println!("Result from {hostname}:");
        println!("{out}");
    }

    if res.is_err() {
        TestResult::new(hostname, Some(out), res)
    } else {
        if args.log_object {
            debug!("Result of {}: {}", &hostname, out);
        }
        TestResult::new(hostname, None, Ok(()))
    }
}

async fn test_host_wmi(
    mut session: Session,
    args: &Args,
) -> (String, Result<()>) {
    let mut out = Vec::new();

    if args.wmi_method == WmiMethod::EnumerateCimInstance {
        for obj in args.wmi_object.iter() {
            match session.enumerate_ciminstance(obj, "root\\cimv2").await {
                Err(e) => return (obj.to_string(), Err(Error::WinRM(e))),
                Ok(r) => out.push(format!("{}: {:#?}", obj, r)),
            }
        }
    } else {
        let shell = match session.shell().await {
            Ok(s) => s,
            Err(e) => {
                return (String::from("Shell creation"), Err(Error::WinRM(e)))
            }
        };

        for obj in args.wmi_object.iter() {
            let res: Result<Vec<HashMap<String, String>>> =
                match args.wmi_method {
                    WmiMethod::GetWmiObject => {
                        session
                            .get_wmiobject(
                                &shell,
                                obj,
                                &[String::from("*")],
                                &String::from("root\\cimv2"),
                            )
                            .await
                    }
                    WmiMethod::GetCimInstance => {
                        session
                            .get_ciminstance(
                                &shell,
                                obj,
                                &[String::from("*")],
                                &String::from("root\\cimv2"),
                            )
                            .await
                    }
                    _ => unreachable!("Handled by args"),
                }
                .map_err(Error::WinRM);

            match res {
                Err(e) => return (obj.to_string(), Err(e)),
                Ok(r) => out.push(format!("{}: {:#?}", obj, r)),
            }
        }
        if let Err(e) = session.close_shell(shell).await {
            return (String::from("Closing shell"), Err(Error::WinRM(e)));
        }
    }

    (out.join("\n").to_string(), Ok(()))
}

async fn create_session(hostname: &String, args: &Args) -> Result<Session> {
    let hostname = if args.append_domain && args.domain.is_some() {
        format!("{}.{}", hostname, args.domain.as_ref().unwrap())
    } else {
        hostname.clone()
    };
    let mut session_builder = SessionBuilder::with_credentials(
        args.get_winrm_credentials(&hostname).await?,
    )
    .hostname(hostname)
    .https(!args.disable_ssl)
    .timeout(args.timeout)
    .ignore_hostnames(args.danger_disable_hostname_verification)
    .ignore_cert(args.danger_disable_certificate_verification);

    if let Some(ip) = args.ipaddr {
        session_builder = session_builder.resolve(std::net::IpAddr::V4(ip))
    }
    if let Some(port) = args.port {
        session_builder = session_builder.port(port);
    }
    if let Some(cert_path) = &args.cacert {
        let cert = fs::read(cert_path)
            .await
            .map_err(|e| Error::Cert(e.to_string()))?;
        session_builder = session_builder
            .root_ca(winrm_rs::session::CertificateFormat::PEM, &cert)
            .map_err(|e| {
                Error::Cert(format!("Not a valid PEM format: {}", e))
            })?;
    }
    let mut session = session_builder.build()?;
    session.login().await?;
    Ok(session)
}
