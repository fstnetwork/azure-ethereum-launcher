use std::net::IpAddr;

use ethereum_types::U256;

mod consensus_engine;
mod context;
mod error;
mod node_type;

pub use self::consensus_engine::ConsensusEngine;
pub use self::context::Context;
pub use self::error::{Error, ErrorKind};
pub use self::node_type::NodeType;

#[derive(Debug, Clone, Copy)]
pub enum EthereumProgram {
    Parity,
    GoEthereum,
}

#[derive(Debug, Clone, Copy)]
pub struct LauncherParameters {
    pub network_port: u16,
    pub http_jsonrpc_port: u16,
    pub websocket_jsonrpc_port: u16,
    pub genesis_block_gas_limit: U256,
}

#[derive(Debug, Copy, Clone)]
#[allow(dead_code)]
pub enum RestartPolicy {
    No,
    Always,
    OnFailure,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EthereumNodeUrl {
    pub node_id: String,
    pub addr: IpAddr,
    pub port: u16,
}

impl ToString for EthereumNodeUrl {
    fn to_string(&self) -> String {
        format!(
            "enode://{}@{}:{}",
            self.node_id,
            self.addr.to_string(),
            self.port
        )
    }
}
