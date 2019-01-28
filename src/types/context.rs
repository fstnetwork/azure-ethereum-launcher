use std::net::IpAddr;
use std::time::Duration;

use ethereum_types::{self, U256};

use super::{ConsensusEngine, EthereumProgram, LauncherParameters, NodeType, RestartPolicy};
use super::{Error, ErrorKind};

#[derive(Debug, Clone)]
pub struct Context {
    /// is this the first time we run Ethereum service
    pub first_run: bool,

    /// public IP address of this container
    pub public_ip: IpAddr,

    /// name of this ethereum network
    pub network_name: String,

    /// consensus engine type and its parameters
    pub consensus_engine: ConsensusEngine,

    /// which Ethereum program should we execute
    pub ethereum_program: EthereumProgram,

    /// node type of this container
    pub node_type: NodeType,

    /// restart policy
    pub restart_policy: RestartPolicy,

    /// common launcher parameters
    pub launcher_parameters: LauncherParameters,

    /// hostname of bootnode service
    pub bootnode_service_host: String,

    /// port of bootnode service
    pub bootnode_service_port: u16,

    /// interval for update enode URL to bootnode service
    pub bootnode_update_interval: Duration,
}

fn is_first_run() -> Result<bool, Error> {
    use std::env;
    use std::path::PathBuf;

    let home_dir = env::var("HOME")?;
    std::fs::create_dir_all(home_dir.clone()).expect("this should always successful");
    let lock_path = {
        let mut path = PathBuf::new();
        path.push(home_dir);
        path.push("first-run-lock");
        path
    };

    match std::fs::File::open(lock_path.clone()) {
        Ok(_) => {
            info!(target: "context", "first-run lock is existed, this is not first run");
            Ok(false)
        }
        Err(_err) => {
            info!(target: "context",
                "first-run lock is not existed, this is first run, try to create first-run lock: {:?}",
                lock_path);

            // create first-run lock
            std::fs::File::create(lock_path)?;
            Ok(true)
        }
    }
}

impl Context {
    pub fn from_system() -> Result<Context, Error> {
        use std::env;
        let network_name = env::var("NETWORK_NAME")?;

        let node_type = {
            let node_type = env::var("NODE_TYPE")?;

            match node_type.to_lowercase().as_ref() {
                "transactor" => {
                    let seed = env::var("SEALER_MASTER_SEED")?;
                    let count: usize = env::var("MINER_COUNT")?.parse()?;

                    NodeType::Transactor {
                        sealer_mnemonic: seed,
                        miner_count: count,
                    }
                }
                "miner" => {
                    let seed = env::var("SEALER_MASTER_SEED")?;
                    let index: usize = env::var("MINER_INDEX")?.parse()?;
                    let miner_count: usize = env::var("MINER_COUNT")?.parse()?;

                    if index >= miner_count {
                        return Err(Error::from(ErrorKind::TooLargeMinerIndex(
                            index,
                            miner_count,
                        )));
                    }

                    NodeType::Miner {
                        sealer_mnemonic: seed,
                        index,
                        miner_count,
                    }
                }
                _ => return Err(Error::from(ErrorKind::UnknownNodeType(node_type))),
            }
        };

        let consensus_engine = {
            use serde_json::Value as JsonValue;

            let engine = env::var("CONSENSUS_ENGINE")?;
            match engine.to_lowercase().as_ref() {
                "ethash" => {
                    let engine_parameters: JsonValue =
                        serde_json::from_str(env::var("ETHASH_CONSENSUS_PARAMETERS")?.as_str())?;
                    let genesis_difficulty: U256 = engine_parameters["genesisBlockDifficulty"]
                        .as_u64()
                        .unwrap_or(16384)
                        .into();
                    ConsensusEngine::Ethash { genesis_difficulty }
                }
                "aura" => {
                    let engine_parameters: JsonValue =
                        serde_json::from_str(env::var("AURA_CONSENSUS_PARAMETERS")?.as_str())?;
                    let block_period = engine_parameters["blockPeriod"].as_u64().unwrap_or(7);

                    let block_reward = U256::from_dec_str(
                        engine_parameters["blockReward"]
                            .as_str()
                            .unwrap_or("5000000000000000000".into()),
                    )
                    .unwrap_or(U256::from(5) * U256::from(10).pow(18.into()));

                    ConsensusEngine::ParityAura {
                        block_period,
                        block_reward,
                    }
                }
                "clique" => {
                    let engine_parameters: JsonValue =
                        serde_json::from_str(env::var("CLIQUE_CONSENSUS_PARAMETERS")?.as_str())?;
                    let block_period = engine_parameters["blockPeriod"].as_u64().unwrap_or(7);
                    ConsensusEngine::GethClique { block_period }
                }
                _ => {
                    return Err(Error::from(ErrorKind::InvalidConsensusEngineType(engine)));
                }
            }
        };

        let launcher_parameters = LauncherParameters {
            network_port: env::var("P2P_NETWORK_SERVICE_PORT")?.parse()?,
            http_jsonrpc_port: env::var("HTTP_JSON_RPC_PORT")?.parse()?,
            websocket_jsonrpc_port: env::var("WEBSOCKET_JSON_RPC_PORT")?.parse()?,
            genesis_block_gas_limit: {
                use std::str::FromStr;
                let raw_value = env::var("GENESIS_BLOCK_GAS_LIMIT")?;
                match U256::from_str(ethereum_types::clean_0x(raw_value.as_str())) {
                    Ok(v) => v,
                    Err(_) => return Err(Error::from(ErrorKind::InvalidGasLimitValue(raw_value))),
                }
            },
        };

        let public_ip = env::var("PUBLIC_IP").unwrap_or("0.0.0.0".into()).parse()?;

        Ok(Context {
            first_run: is_first_run()?,

            public_ip,
            network_name,

            consensus_engine,
            ethereum_program: consensus_engine.program(),

            node_type,
            launcher_parameters,

            restart_policy: RestartPolicy::Always,

            bootnode_service_host: env::var("BOOTNODE_SERVICE_HOST")?,
            bootnode_service_port: env::var("BOOTNODE_SERVICE_PORT")?.parse()?,
            bootnode_update_interval: Duration::from_secs(
                env::var("BOOTNODE_SERVICE_UPDATE_INTERVAL")
                    .unwrap_or("10".into())
                    .parse()?,
            ),
        })
    }

    pub fn is_first_miner(&self) -> bool {
        match self.node_type {
            NodeType::Miner { index, .. } => return 0 == index,
            NodeType::Transactor { .. } => false,
        }
    }
}
