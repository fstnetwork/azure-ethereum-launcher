use std::io::Write;
use std::path::PathBuf;

use ethereum_types::{Address, U256};
use ethkey::KeyPair;

use emerald::keystore::{Kdf, KeyFile};
use emerald::PrivateKey;

use super::Error;
use super::{ConsensusEngine, EthereumNodeUrl, NodeType};

pub fn create_key_directory(config_dir_path: &PathBuf) -> Result<PathBuf, Error> {
    let mut path = PathBuf::from(config_dir_path);
    path.push("keys");

    std::fs::create_dir(path.clone())?;
    Ok(path)
}

pub fn create_sealer_key_file(
    key_dir_path: &PathBuf,
    sealer_private_key: &KeyPair,
    passphrase: &String,
) -> Result<PathBuf, Error> {
    let secret: [u8; 32] = (**sealer_private_key.secret()).into();
    let private_key = PrivateKey::from(secret);

    let mut rng = rand::thread_rng();

    let keyfile = {
        let keyfile = KeyFile::new_custom(
            private_key,
            passphrase.as_str(),
            Kdf::default(),
            &mut rng,
            None,
            None,
        )?;
        let mut value = serde_json::to_value(keyfile)?;
        let obj = value
            .as_object_mut()
            .expect("keyfile must be an object; qed");

        // remove unused fields
        obj.remove("name");
        obj.remove("description");
        obj.remove("visible");

        value.clone()
    };

    let mut path = PathBuf::from(key_dir_path);
    path.push("signer_keyfile.json");

    serde_json::to_writer(
        std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .open(path.clone())?,
        &keyfile,
    )?;

    Ok(path)
}

pub fn create_sealer_passphrase_file(
    config_dir: &PathBuf,
    passphrase: &String,
) -> Result<PathBuf, Error> {
    let mut path = PathBuf::from(config_dir);
    path.push("sealer_passphrase");

    std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .open(path.clone())?
        .write(passphrase.as_bytes())?;

    Ok(path)
}

pub fn create_spec_file(
    config_dir: &PathBuf,
    consensus_engine: &ConsensusEngine,
    validators: &Vec<Address>,
    genesis_gas_limit: U256,
) -> Result<PathBuf, Error> {
    let mut path = PathBuf::from(config_dir);
    path.push("spec.json");

    let (chain_name, network_id, engine, seal) = match consensus_engine {
        ConsensusEngine::ParityAura {
            block_period,
            block_reward,
        } => (
            "ParityAura",
            U256::from(0x2323),
            json!({
                "authorityRound": {
                    "params": {
                        "stepDuration": block_period.to_string(),
                        "blockReward": block_reward.to_string(),
                        "validators": {
                            "list": validators
                        }
                    }
                }
            }),
            json!({
            "authorityRound": {
                "step": "0x0",
                "signature": "0x0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000"
            }
            }),
        ),
        _ => {
            unimplemented!();
        }
    };

    let spec = json!({
        "name": chain_name,
        "genesis": {
            "difficulty": "0x1",
            "gasLimit": format!("0x{:x}", genesis_gas_limit),
            "seal": seal
        } ,
        "params": {
            "maximumExtraDataSize": "0x20",
            "minGasLimit":          "0x1388",
            "gasLimitBoundDivisor": "0x400",
            "networkID":  format!("0x{:x}", network_id),
            "eip155Transition": 0,
            "maxCodeSize": 24576,
            "maxCodeSizeTransition": 0,
            "validateChainIdTransition": 0,
            "validateReceiptsTransition": 0,
            "eip140Transition": 0,
            "eip211Transition": 0,
            "eip214Transition": 0,
            "eip658Transition": 0,
            "wasmActivationTransition": 0
        },
        "engine": engine,
        "accounts": {
            "0x0000000000000000000000000000000000000001": {
                "balance": "1", "builtin": {
                    "name": "ecrecover",
                    "pricing": {
                        "linear": {
                            "base": 3000,
                            "word": 0
                        }
                    }
                }
            },
            "0x0000000000000000000000000000000000000002": {
                "balance": "1",
                "builtin": {
                    "name": "sha256",
                    "pricing": {
                        "linear": {
                            "base": 60,
                            "word": 12
                        }
                    }
                }
            },
            "0x0000000000000000000000000000000000000003": {
                "balance": "1",
                "builtin": {
                    "name": "ripemd160",
                    "pricing": {
                        "linear": {
                            "base": 600,
                            "word": 120
                        }
                    }
                }
            },
            "0x0000000000000000000000000000000000000004": {
                "balance": "1",
                "builtin": {
                    "name": "identity",
                    "pricing": {
                        "linear": {
                            "base": 15,
                            "word": 3
                        }
                    }
                }
            }
        }
    });

    serde_json::to_writer(
        std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .open(path.clone())?,
        &spec,
    )?;
    Ok(path)
}

pub fn create_reserverd_peers_file(
    config_dir: &PathBuf,
    bootnodes: &Vec<EthereumNodeUrl>,
) -> Result<PathBuf, Error> {
    let mut path = PathBuf::from(config_dir);
    path.push("reserved_peers");

    let data = bootnodes.iter().fold(String::new(), |mut s, url| {
        s.push_str(url.to_string().as_str());
        s.push('\n');
        s
    });

    std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .open(path.clone())?
        .write_all(data.as_bytes())?;
    Ok(path)
}

#[derive(Debug, Clone)]
pub struct ParityConfig {
    pub node_type: NodeType,

    pub identity: String,
    pub spec_path: String,

    pub bootnodes: Vec<EthereumNodeUrl>,
    pub reserved_peers_file_path: String,

    pub sealer_address: Address,
    pub sealer_passphrase_file_path: String,

    pub ipc_path: String,
    pub network_port: u16,
    pub http_jsonrpc_port: u16,
    pub websocket_jsonrpc_port: u16,
}

impl ParityConfig {
    pub fn toml_config(&self) -> toml::Value {
        let chain = self.spec_path.clone();
        let identity = self.identity.clone();
        let (engine_signer, author, unlock) = {
            let engine_signer = format!("{:x?}", self.sealer_address);
            (engine_signer.clone(), engine_signer.clone(), engine_signer)
        };

        let password = self.sealer_passphrase_file_path.clone();
        let bootnodes: Vec<_> = self
            .bootnodes
            .iter()
            .map(EthereumNodeUrl::to_string)
            .collect();
        let reserved_peers = self.reserved_peers_file_path.clone();
        let ipc_path = self.ipc_path.clone();
        let network_port = self.network_port;
        let http_jsonrpc_port = self.http_jsonrpc_port;
        let websocket_jsonrpc_port = self.websocket_jsonrpc_port;

        match self.node_type {
            NodeType::Miner { .. } => {
                toml! {
                    [parity]
                    chain = chain
                    identity = identity
                    no_persistent_txqueue = false
                    light = false
                    no_download = true

                    [network]
                    bootnodes = bootnodes
                    port = network_port
                    reserved_peers = reserved_peers
                    reserved_only = false

                    [account]
                    unlock = [ unlock ]
                    password = [ password ]

                    [mining]
                    author = author
                    engine_signer = engine_signer
                    reseal_on_txs = "none"
                    usd_per_tx = "0"

                    [websockets]
                    interface = "0.0.0.0"
                    port = websocket_jsonrpc_port
                    hosts = ["all"]
                    apis = ["eth", "net", "parity"]
                    origins = ["http://127.0.0.1:8180", "http://127.0.0.1:8181", "http://127.0.0.1:8182"]

                    [rpc]
                    interface = "0.0.0.0"
                    port = http_jsonrpc_port
                    hosts = ["all"]
                    apis = ["eth", "net", "parity"]

                    [ipc]
                    disable = false
                    path = ipc_path
                    apis = ["all"]

                    [misc]
                    logging = "network=warn,miner=warn,mode=warn"
                    color = true
                }
            }
            NodeType::Transactor { .. } => {
                toml! {
                    [parity]
                    chain = chain
                    identity = identity
                    no_persistent_txqueue = false
                    light = false
                    no_download = true

                    [network]
                    bootnodes = bootnodes
                    port = network_port
                    reserved_peers = reserved_peers
                    reserved_only = false

                    [websockets]
                    interface = "0.0.0.0"
                    port = websocket_jsonrpc_port
                    hosts = ["all"]
                    apis = ["eth", "net", "parity"]
                    origins = ["http://127.0.0.1:8180", "http://127.0.0.1:8181", "http://127.0.0.1:8182"]

                    [rpc]
                    interface = "0.0.0.0"
                    port = http_jsonrpc_port
                    hosts = ["all"]
                    apis = ["eth", "net", "parity"]

                    [ipc]
                    disable = false
                    path = ipc_path
                    apis = ["all"]

                    [misc]
                    logging = "network=warn,miner=warn,mode=warn"
                    color = true
                }
            }
        }

        // log_file = "/var/log/parity.log"
    }

    pub fn save(&self, config_file_path: &PathBuf) -> Result<PathBuf, Error> {
        let config = self.toml_config();
        let data = toml::to_string(&config).expect("config is serializable; qed");
        std::fs::File::create(config_file_path)?.write_all(data.as_bytes())?;
        Ok(config_file_path.clone())
    }
}
