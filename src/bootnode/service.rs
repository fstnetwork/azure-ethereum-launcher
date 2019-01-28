use futures::{sync::mpsc, Async, Future, Poll, Stream};
use std::net::IpAddr;

use hyper::{Body, Client, Request, Uri};

use super::jsonrpc_client::JsonRpcClient;
use super::{Error, ErrorKind};
use super::{EthereumNodeUrl, EthereumProgram, NodeType};

#[derive(Copy, Clone)]
enum State {
    Idle,
    FetchingEthereumNodeUrl,
    UpdatingEthereumNodeUrl,
}

impl ToString for State {
    fn to_string(&self) -> String {
        match self {
            State::Idle => "Idle".to_owned(),
            State::FetchingEthereumNodeUrl => "FetchingEnodeUrl".to_owned(),
            State::UpdatingEthereumNodeUrl => "UpdatingEnodeUrl".to_owned(),
        }
    }
}

type UrlFetcher = Box<Future<Item = EthereumNodeUrl, Error = Error> + Send>;
type UrlUpdater = Box<Future<Item = bool, Error = Error> + Send>;

enum StateWorker {
    Idle,
    Fetcher { fetcher: UrlFetcher },
    Updater { updater: UrlUpdater },
}

impl StateWorker {
    fn new_fetcher(client: &JsonRpcClient, _ethereum_program: EthereumProgram) -> StateWorker {
        StateWorker::Fetcher {
            fetcher: Box::new(client.parity_enode().from_err()),
        }
    }

    fn new_updater(
        bootstrap_service_url: &Uri,
        public_ip: &IpAddr,
        enode_url: EthereumNodeUrl,
        network_name: String,
        is_miner: bool,
    ) -> StateWorker {
        #[derive(Clone, Serialize)]
        struct EnodeInfo {
            enode: String,
            port: u16,
            ip: String,
            #[serde(rename = "publicIp")]
            public_ip: String,
            network: String,
            miner: bool,
        }

        let req = Request::builder()
            .uri(bootstrap_service_url)
            .method("POST")
            .header("Content-Type", "application/json")
            .body(Body::from({
                let enode_info = EnodeInfo {
                    enode: enode_url.node_id,
                    port: enode_url.port,
                    ip: enode_url.addr.to_string(),
                    public_ip: public_ip.to_string(),
                    network: network_name,
                    miner: is_miner,
                };
                let info =
                    serde_json::to_string(&enode_info).expect("EnodeInfo is serializable; qed");

                info!(target: "bootnode", "Update ethereum node info {}", info);

                info
            }))
            .expect("request builder");

        let future = Client::new()
            .request(req)
            .and_then(|res| res.into_body().concat2())
            .from_err::<Error>()
            .and_then(|_data| Ok(true))
            .from_err::<Error>();

        StateWorker::Updater {
            updater: Box::new(future),
        }
    }
}

pub struct Service {
    network_name: String,
    ethereum_program: EthereumProgram,
    node_type: NodeType,
    bootnode_service_uri: Uri,
    public_ip: IpAddr,
    jsonrpc_client: JsonRpcClient,
    state: State,
    state_worker: StateWorker,

    event_sender: mpsc::UnboundedSender<()>,
    event_receiver: mpsc::UnboundedReceiver<()>,
}

impl Service {
    pub fn new(
        network_name: String,
        ethereum_program: EthereumProgram,
        node_type: NodeType,
        bootnode_service_host: String,
        bootnode_service_port: u16,
        public_ip: IpAddr,
        ethereum_node_endpoint: String,
    ) -> Service {
        let bootnode_service_uri = format!(
            "http://{}:{}/",
            bootnode_service_host, bootnode_service_port
        )
        .parse()
        .expect("bootnode URI");

        let (event_sender, event_receiver) = mpsc::unbounded();

        Service {
            network_name,
            ethereum_program,
            bootnode_service_uri,
            jsonrpc_client: JsonRpcClient::new(&ethereum_node_endpoint),
            node_type,
            state: State::Idle,
            state_worker: StateWorker::Idle,
            public_ip,

            event_sender,
            event_receiver,
        }
    }

    pub fn send_event(&mut self) {
        self.event_sender
            .unbounded_send(())
            .expect("receiver always existed; qed");
    }

    fn reset(&mut self) {
        self.state = State::Idle;
        self.state_worker = StateWorker::Idle;
    }

    fn poll_idle(&mut self) -> Poll<Option<()>, Error> {
        if let StateWorker::Idle = self.state_worker {
            match self.event_receiver.poll().unwrap() {
                Async::Ready(Some(_)) => {}
                _ => return Ok(Async::NotReady),
            }
        } else {
            return Err(Error::from(ErrorKind::InvalidStateTransfer(
                self.state.to_string(),
                State::Idle.to_string(),
            )));
        }

        // transfer state
        self.state = State::FetchingEthereumNodeUrl;
        self.state_worker = StateWorker::new_fetcher(&self.jsonrpc_client, self.ethereum_program);

        Ok(Async::NotReady)
    }

    fn poll_fetching(&mut self) -> Poll<Option<()>, Error> {
        let enode_url = if let StateWorker::Fetcher { ref mut fetcher } = self.state_worker {
            match fetcher.poll() {
                Ok(Async::Ready(url)) => url,
                Ok(Async::NotReady) => return Ok(Async::NotReady),
                Err(err) => return Err(Error::from(err)),
            }
        } else {
            return Err(Error::from(ErrorKind::InvalidStateTransfer(
                self.state.to_string(),
                State::FetchingEthereumNodeUrl.to_string(),
            )));
        };

        info!(
            "Update enode url {:?} to {}",
            enode_url.to_string(),
            self.bootnode_service_uri
        );

        // transfer state
        self.state = State::UpdatingEthereumNodeUrl;
        self.state_worker = StateWorker::new_updater(
            &self.bootnode_service_uri,
            &self.public_ip,
            enode_url,
            self.network_name.clone(),
            self.node_type.is_miner(),
        );

        Ok(Async::NotReady)
    }

    fn poll_updating(&mut self) -> Poll<Option<()>, Error> {
        let ok = if let StateWorker::Updater { ref mut updater } = self.state_worker {
            match updater.poll() {
                Ok(Async::Ready(result)) => result,
                Ok(Async::NotReady) => return Ok(Async::NotReady),
                Err(err) => return Err(Error::from(err)),
            }
        } else {
            return Err(Error::from(ErrorKind::InvalidStateTransfer(
                self.state.to_string(),
                State::UpdatingEthereumNodeUrl.to_string(),
            )));
        };

        if !ok {
            warn!(target: "bootnode", "Failed to update enode URL");
        }

        // transfer state
        self.state = State::Idle;
        self.state_worker = StateWorker::Idle;

        Ok(Async::NotReady)
    }
}

impl Stream for Service {
    type Item = ();
    type Error = Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Error> {
        loop {
            let result = match self.state {
                State::Idle => self.poll_idle(),
                State::FetchingEthereumNodeUrl => self.poll_fetching(),
                State::UpdatingEthereumNodeUrl => self.poll_updating(),
            };

            if result.is_err() {
                self.reset();
                return Ok(Async::NotReady);
            }

            return Ok(Async::NotReady);
        }
    }
}
