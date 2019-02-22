use std::path::PathBuf;
use std::process::Command;
use tokio_process::{Child as ChildProcess, CommandExt};

use super::types::{self, *};

mod error;
mod geth;
mod parity;

pub use self::error::{Error, ErrorKind};

const PARITY_EXECUTABLE_PATH: &'static str = "parity";
const GETH_EXECUTABLE_PATH: &'static str = "geth";
const DEFAULT_SEALER_KEYFILE_PASSPHRASE: &'static str = "0123456789";

pub struct EthereumLauncher {
    pub node_type: NodeType,
    pub engine: ConsensusEngine,
    pub bootnodes: Vec<EthereumNodeUrl>,
    pub launcher_parameters: LauncherParameters,
}

impl EthereumLauncher {
    pub fn chain_data_dir_path(&self) -> PathBuf {
        PathBuf::from(std::env::var("CHAIN_DATA_ROOT").unwrap_or("/chain-data".into()))
    }

    pub fn config_dir_path(&self) -> PathBuf {
        let mut path = PathBuf::from(std::env::var("CONFIG_ROOT").unwrap_or("/".into()));
        path.push(match self.engine.program() {
            EthereumProgram::Parity => PathBuf::from("parity-config"),
            EthereumProgram::GoEthereum => PathBuf::from("geth-config"),
        });
        path
    }

    pub fn local_jsonrpc_url(&self) -> String {
        format!(
            "http://127.0.0.1:{}/",
            self.launcher_parameters.http_jsonrpc_port
        )
    }

    pub fn ipc_path(&self) -> PathBuf {
        let mut path = self.config_dir_path();
        path.push(match self.engine.program() {
            EthereumProgram::Parity => "parity.ipc",
            EthereumProgram::GoEthereum => "geth.ipc",
        });
        path
    }

    pub fn config_file_path(&self) -> PathBuf {
        let mut path_buf = self.config_dir_path();
        path_buf.push(match self.engine.program() {
            EthereumProgram::Parity => "config.toml",
            EthereumProgram::GoEthereum => "config.toml",
        });
        path_buf
    }

    pub fn initialize(&self) -> Result<String, Error> {
        match (self.engine.program(), self.node_type.clone()) {
            (EthereumProgram::Parity, NodeType::Miner { index, .. }) => {
                let config_dir = self.config_dir_path();
                std::fs::create_dir_all(config_dir.clone())?;

                let db_path = self.chain_data_dir_path();
                std::fs::create_dir_all(db_path.clone())?;

                let passphrase = String::from(DEFAULT_SEALER_KEYFILE_PASSPHRASE);

                let sealer_key_pair = self
                    .node_type
                    .validator_keypairs()?
                    .get(index)
                    .expect("index must be valid")
                    .clone();
                let key_dir = parity::create_key_directory(&config_dir)?;
                let key_file_path =
                    parity::create_sealer_key_file(&key_dir, &sealer_key_pair, &passphrase)?;

                info!(target: "launcher", "create key file {:?} for {:?}",
                      key_file_path, sealer_key_pair.address());

                let sealer_password_file_path =
                    parity::create_sealer_passphrase_file(&config_dir, &passphrase)?;

                let spec_file_path = parity::create_spec_file(
                    &config_dir,
                    &self.engine,
                    &self.node_type.validators()?,
                    self.launcher_parameters.genesis_block_gas_limit,
                )?;

                let reserved_peers_file_path =
                    parity::create_reserverd_peers_file(&config_dir, &self.bootnodes)?;

                let config_file_path: String = {
                    let config = parity::ParityConfig {
                        db_path: db_path.to_str().expect("db directory path").to_owned(),
                        node_type: self.node_type.clone(),

                        identity: format!("miner-{}", index),
                        spec_path: spec_file_path.to_str().expect("spec file path").to_owned(),
                        bootnodes: self.bootnodes.clone(),
                        reserved_peers_file_path: reserved_peers_file_path
                            .to_str()
                            .expect("reserved peers file")
                            .to_owned(),
                        force_sealing: true,
                        sealer_address: sealer_key_pair.address(),
                        sealer_passphrase_file_path: sealer_password_file_path
                            .to_str()
                            .expect("sealer passphrase file path")
                            .to_owned(),
                        ipc_path: self.ipc_path().to_str().expect("ipc path").to_owned(),
                        network_port: self.launcher_parameters.network_port,
                        http_jsonrpc_port: self.launcher_parameters.http_jsonrpc_port,
                        websocket_jsonrpc_port: self.launcher_parameters.websocket_jsonrpc_port,
                    };

                    config
                        .save(&self.config_file_path())?
                        .to_str()
                        .expect("config file path")
                        .into()
                };

                Command::new(PARITY_EXECUTABLE_PATH)
                    .arg(format!("--config={}", config_file_path))
                    .arg("account")
                    .arg("import")
                    .arg(key_dir.to_str().expect("key directory"))
                    .spawn()?;

                Ok(config_file_path)
            }
            (EthereumProgram::Parity, NodeType::Transactor { .. }) => {
                let config_dir = self.config_dir_path();
                std::fs::create_dir_all(config_dir.clone())?;

                let db_path = self.chain_data_dir_path();
                std::fs::create_dir_all(db_path.clone())?;

                let spec_file_path = parity::create_spec_file(
                    &config_dir,
                    &self.engine,
                    &self.node_type.validators()?,
                    self.launcher_parameters.genesis_block_gas_limit,
                )?;

                let reserved_peers_file_path =
                    parity::create_reserverd_peers_file(&config_dir, &self.bootnodes)?;
                let fake_sealer = {
                    use ethkey::{Brain, Generator};
                    Brain::new("0123456789".into())
                        .generate()
                        .expect("generate keypair from random")
                };

                let config_file_path: String = {
                    let config = parity::ParityConfig {
                        db_path: db_path.to_str().expect("db directory path").to_owned(),
                        node_type: self.node_type.clone(),

                        identity: "transactor".into(),
                        spec_path: spec_file_path.to_str().expect("spec file path").to_owned(),
                        bootnodes: self.bootnodes.clone(),
                        reserved_peers_file_path: reserved_peers_file_path
                            .to_str()
                            .expect("reserved peers file")
                            .to_owned(),

                        force_sealing: false,
                        sealer_address: fake_sealer.address(),
                        sealer_passphrase_file_path: parity::create_sealer_passphrase_file(
                            &config_dir,
                            &DEFAULT_SEALER_KEYFILE_PASSPHRASE.into(),
                        )?
                        .to_str()
                        .expect("sealer passphrase file path")
                        .to_owned(),
                        ipc_path: self.ipc_path().to_str().expect("ipc path").to_owned(),
                        network_port: self.launcher_parameters.network_port,
                        http_jsonrpc_port: self.launcher_parameters.http_jsonrpc_port,
                        websocket_jsonrpc_port: self.launcher_parameters.websocket_jsonrpc_port,
                    };

                    config
                        .save(&self.config_file_path())?
                        .to_str()
                        .expect("config file path")
                        .into()
                };

                Ok(config_file_path)
            }
            (EthereumProgram::GoEthereum, NodeType::Miner { .. }) => {
                unimplemented!();
            }
            (EthereumProgram::GoEthereum, NodeType::Transactor { .. }) => {
                unimplemented!();
            }
        }
    }

    fn execute_command(&self) -> (Command, Vec<String>) {
        let config_file_path =
            String::from(self.config_file_path().to_str().expect("config file path"));
        match self.engine.program() {
            EthereumProgram::Parity => (
                Command::new(PARITY_EXECUTABLE_PATH),
                vec![
                    format!("--config={}", config_file_path),
                    "--no-download".into(),
                    "--no-hardware-wallets".into(),
                ],
            ),
            EthereumProgram::GoEthereum => (Command::new(GETH_EXECUTABLE_PATH), vec![]),
        }
    }

    pub fn execute_async(&self) -> Result<ChildProcess, std::io::Error> {
        let (mut cmd, args) = self.execute_command();
        cmd.args(args).spawn_async()
    }
}
