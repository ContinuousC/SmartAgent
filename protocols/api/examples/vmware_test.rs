/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::{collections::HashMap, path::PathBuf, sync::Arc};

use api_protocol::{
    soap::{CertType, SoapClient},
    vmware::requests::{
        AvailableCountersRequest, LoginRequest, PerfCounterDataRequest,
        PerfCounterSyntaxRequest, SysteminfoRequest,
    },
};
use reqwest::header::{HeaderMap, HeaderValue};

#[tokio::main]
async fn main() {
    let endpoint =
        format!("https://{}:{}/sdk", "SRVVM003.azstlucas.local", 443);
    //format!("https://{}:{}/sdk", "olvesxp29.olvz.intra", 443);

    let mut headers: HeaderMap = HeaderMap::new();
    headers.insert("SOAPAction", HeaderValue::from_static("urn:vim25/5.0"));

    let soapclient = Arc::new(
        SoapClient::create(
            endpoint,
            headers,
            Some(&(
                CertType::PEM,
                PathBuf::from("/omd/sites/stlucas_test/srvvm003.cert"),
                //PathBuf::from("/omd/sites/olvz_test/OLVESXP29.pem"),
            )),
            true,
            false,
        )
        .await
        .expect("SOAP error"),
    );

    eprintln!("sysinfo...");
    let sysinfo = SysteminfoRequest::new(&soapclient, &HashMap::new())
        .await
        .expect("sysinfo");
    println!("sysinfo: {:?}", sysinfo);

    let mut args = sysinfo.to_hashmap();

    args.insert(String::from("username"), String::from("mnowesxi"));
    args.insert(
        String::from("password"),
        String::from("1v2PQUcA(?(BRD7ro>Ta<I"),
    );

    eprintln!("login...");
    let login = LoginRequest::new(&soapclient, &args).await.expect("login");
    println!("login: {:?}", login);

    // let hostsystems = HostsytemsRequest::new(&soapclient, &args)
    //     .await
    //     .expect("hostsystems");
    // println!("hostsystems: {:?}", hostsystems);

    args.insert(String::from("perf_manager"), String::from("ha-perfmgr"));
    args.insert(String::from("esxhost"), String::from("ha-host"));

    eprintln!("available_counters...");
    let available_counters = AvailableCountersRequest::new(&soapclient, &args)
        .await
        .expect("available_counters");
    println!("available_counters: {:?}", available_counters);

    args.insert(
        String::from("counterids"),
        available_counters
            .body
            .query_available_perf_metric_response
            .returnval
            .iter()
            .filter_map(|v| {
                let _ = v.instance.data.as_ref()?;
                Some(format!(
                    "<ns1:counterId>{}</ns1:counterId>",
                    v.counter_id.data
                ))
            })
            .collect::<Vec<_>>()
            .concat(),
    );

    eprintln!("perfcounter_syntax...");
    let perfcounter_syntax = PerfCounterSyntaxRequest::new(&soapclient, &args)
        .await
        .expect("perfcounter_syntax_request");
    println!("perfcounter_syntax_request: {:?}", perfcounter_syntax);

    args.insert(String::from("samples"), format!("{}", 5 * 3));
    args.insert(
        String::from("counters"),
        available_counters
            .body
            .query_available_perf_metric_response
            .returnval
            .iter()
            .filter_map(|v| {
                let id = v.counter_id.data;
                let instance = v.instance.data.as_ref()?;
                Some(format!(
                    "<ns1:metricId><ns1:counterId>{}</ns1:counterId>\
					 <ns1:instance>{}</ns1:instance></ns1:metricId>",
                    &id, &instance
                ))
            })
            .collect::<Vec<_>>()
            .concat(),
    );

    eprintln!("perfcounter_data...");
    let perfcounter_data = PerfCounterDataRequest::new(&soapclient, &args)
        .await
        .expect("perfcounter_data_request");
    println!("perfcounter_data_request: {:?}", perfcounter_data);
}
