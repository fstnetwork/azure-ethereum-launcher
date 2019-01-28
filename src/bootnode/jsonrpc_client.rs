use std::sync::atomic::{AtomicUsize, Ordering};

use futures::{Future, Stream};
use hyper::{client::HttpConnector, http::HttpTryFrom, Body, Client, Request, Uri};
use jsonrpc_core::request::MethodCall;
use jsonrpc_core::response::{
    Failure as JsonRpcFailure, Output as JsonRpcOutput, Success as JsonRpcSuccess,
};
use jsonrpc_core::{Id, Params, Version};
use serde_json::Value as JsonValue;

use super::EthereumNodeUrl;
use super::{Error, ErrorKind};

pub struct JsonRpcClient {
    host: Uri,
    client: Client<HttpConnector, Body>,
    counter: AtomicUsize,
}

impl JsonRpcClient {
    pub fn new(host: &String) -> JsonRpcClient {
        let host = Uri::try_from(host).unwrap();
        let client = Client::builder().keep_alive(true).build_http();
        JsonRpcClient {
            host,
            client,
            counter: AtomicUsize::default(),
        }
    }

    pub fn request(
        &self,
        method: &'static str,
        params: Vec<JsonValue>,
    ) -> impl Future<Item = JsonRpcOutput, Error = Error> {
        let id = self.counter.fetch_add(1, Ordering::Relaxed);
        let method_call = MethodCall {
            jsonrpc: Some(Version::V2),
            method: method.to_owned(),
            params: Params::Array(params),
            id: Id::Num(id as u64),
        };

        let serialized = serde_json::to_string(&method_call).expect("request is serializable; qed");
        let request = Request::post(&self.host)
            .header("Content-Type", "application/json")
            .body(serialized.into())
            .unwrap();

        self.client
            .request(request)
            .and_then(|res| res.into_body().concat2())
            .from_err::<Error>()
            .and_then(|data| Ok(serde_json::from_slice::<JsonRpcOutput>(&data)?))
            .from_err()
    }

    pub fn parity_enode(&self) -> impl Future<Item = EthereumNodeUrl, Error = Error> {
        self.request("parity_enode", vec![]).and_then(|data| {
            use url::Url;
            let url = {
                let url: String = serde_json::from_value(extract_result(data)?)?;
                Url::parse(url.as_str())?
            };

            Ok(EthereumNodeUrl {
                node_id: url.username().to_owned(),
                addr: url.host_str().unwrap().parse()?,
                port: url.port().unwrap(),
            })
        })
    }
}

pub fn extract_result(value: JsonRpcOutput) -> Result<JsonValue, Error> {
    match value {
        JsonRpcOutput::Success(JsonRpcSuccess { result, .. }) => Ok(result),
        JsonRpcOutput::Failure(JsonRpcFailure { error, .. }) => {
            Err(Error::from(ErrorKind::JsonRpc(error.clone())))
        }
    }
}
