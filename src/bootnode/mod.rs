mod error;
mod jsonrpc_client;
mod service;

use futures::{Future, Stream};
use hyper::{Body, Client, Request, Uri};

pub use self::error::{Error, ErrorKind};
pub use self::service::Service;

use super::types::{EthereumNodeUrl, EthereumProgram, NodeType};

pub fn fetch_static_enodes(
    bootnode_service_host: &String,
    bootnode_service_port: u16,
    network_name: &String,
) -> impl Future<Item = Vec<EthereumNodeUrl>, Error = Error> {
    let client = Client::new();
    let req = {
        let query_uri = Uri::builder()
            .scheme("http")
            .authority(format!("{}:{}", bootnode_service_host, bootnode_service_port).as_str())
            .path_and_query(format!("/staticenodes?network={}", network_name).as_str())
            .build()
            .unwrap();

        Request::builder()
            .uri(&query_uri)
            .method("GET")
            .header("Content-Type", "application/json")
            .body(Body::empty())
            .expect("request builder")
    };

    client
        .request(req)
        .and_then(|res| res.into_body().concat2())
        .and_then(|data| {
            use serde_json::Value as JsonValue;
            use url::Url;
            match serde_json::from_slice(&data) {
                Ok(JsonValue::Array(arr)) => Ok(arr.iter().fold(Vec::new(), |mut vec, value| {
                    let value = match value {
                        JsonValue::String(s) => s,
                        _ => return vec,
                    };
                    let url = match value.parse::<Url>() {
                        Ok(uri) => uri,
                        _ => return vec,
                    };

                    if url.scheme() != "enode"
                        || url.username().is_empty()
                        || url.host().is_none()
                        || url.port().is_none()
                    {
                        return vec;
                    }

                    let host = url.host_str().unwrap();
                    let port = url.port().unwrap_or(30303);
                    let addr = match host.parse::<std::net::IpAddr>() {
                        Ok(addr) => addr,
                        Err(_) => return vec,
                    };

                    vec.push(EthereumNodeUrl {
                        node_id: url.username().to_owned(),
                        port,
                        addr,
                    });
                    vec
                })),
                _ => Ok(vec![]),
            }
        })
        .from_err()
}
