#![recursion_limit = "128"]
extern crate rand;

#[macro_use]
extern crate error_chain;

#[macro_use]
extern crate log;
extern crate env_logger;

extern crate futures;
extern crate tokio;
extern crate tokio_process;
extern crate tokio_timer;

extern crate serde;

#[macro_use]
extern crate serde_json;

#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate toml;

extern crate emerald_rs as emerald;
extern crate ethereum_types;
extern crate ethkey;
extern crate hyper;
extern crate jsonrpc_core;

mod bootnode;
mod ethereum;
mod launcher;
mod service;
mod types;

use futures::Stream;
use std::thread;
use std::time::Duration;
use tokio::runtime::Runtime;

use self::bootnode::Service as BootnodeService;
use self::ethereum::Service as EthereumService;
use self::launcher::EthereumLauncher;
use self::service::Service;
use self::types::Context;

fn main() {
    env_logger::init();

    let ctx = match Context::from_system() {
        Ok(ctx) => {
            info!("Context: {:?}", ctx);
            ctx
        }
        Err(err) => {
            error!("{:?}", err);
            panic!(err)
        }
    };

    let mut runtime = match Runtime::new() {
        Ok(runtime) => runtime,
        Err(err) => panic!(err),
    };

    // try to fetch static nodes
    let static_nodes = match ctx.is_first_miner() {
        true => Vec::new(),
        false => {
            let retry_limit = 100;
            let mut retry_count = 0;
            let timeout = Duration::from_secs(5);

            let mut nodes = Vec::new();
            while nodes.is_empty() && retry_count <= retry_limit {
                info!(
                    "Try to fetch static nodes ({}/{})",
                    retry_count, retry_limit
                );
                retry_count += 1;
                nodes = match runtime.block_on(bootnode::fetch_static_enodes(
                    &ctx.bootnode_service_host,
                    ctx.bootnode_service_port,
                    &ctx.network_name,
                )) {
                    Ok(nodes) => nodes,
                    Err(err) => {
                        warn!("failed to fetch static nodes, error: {:?}", err);
                        thread::sleep(timeout);
                        vec![]
                    }
                };

                if nodes.is_empty() {
                    thread::sleep(timeout);
                }
            }

            nodes
        }
    };

    let (ethereum, ethereum_node_endpoint) = {
        let launcher = EthereumLauncher {
            node_type: ctx.node_type.clone(),
            engine: ctx.consensus_engine,
            launcher_parameters: ctx.launcher_parameters,
            bootnodes: static_nodes,
        };

        if ctx.first_run {
            // initialize Ethereum
            match launcher.initialize() {
                Ok(_) => {}
                Err(err) => {
                    println!("{:?}", err);
                    panic!(err)
                }
            }
        }

        let local_jsonrpc_url = launcher.local_jsonrpc_url();
        (
            EthereumService::new(launcher, ctx.restart_policy),
            local_jsonrpc_url,
        )
    };

    let bootnode = BootnodeService::new(
        ctx.network_name,
        ctx.ethereum_program,
        ctx.node_type.clone(),
        ctx.bootnode_service_host,
        ctx.bootnode_service_port,
        ctx.public_ip,
        ethereum_node_endpoint,
    );

    let service = Service::new(ethereum, bootnode, ctx.bootnode_update_interval);
    match runtime.block_on(service.into_future()) {
        Ok(_) => {}
        Err(err) => {
            panic!(err);
        }
    }
}
