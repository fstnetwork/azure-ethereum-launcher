#!/usr/bin/env bash

# WARN: make sure you execute this script in root directory of project
cargo build

ROOT_PREFIX="/tmp/ethereum-launcher-test"

mkdir -p $ROOT_PREFIX

export NETWORK_NAME="Parity-Aura"
export SEALER_MASTER_SEED="rose rocket invest real refuse margin festival danger anger border idle brown"
export MINER_COUNT=2
export TRANSACTOR_COUNT=3

export CONSENSUS_ENGINE="Aura"
export AURA_CONSENSUS_PARAMETERS="{\"blockPeriod\": 5}"

export GENESIS_BLOCK_GAS_LIMIT="0x6422c40"

export BOOTNODE_SERVICE_HOST="localhost"
export BOOTNODE_SERVICE_PORT=3000

export RUST_BACKTRACE=1

trap "pkill ethereum-launch; pkill parity; exit 0" INT

for ((i = 0; i < $MINER_COUNT; i++)); do
    export NODE_TYPE=Miner
    export MINER_INDEX=$i

    export P2P_NETWORK_SERVICE_PORT=$(($i + 30303))
    export HTTP_JSON_RPC_PORT=$(($i + 8545))
    export WEBSOCKET_JSON_RPC_PORT=$(($i + 18546))

    export CHAIN_DATA_ROOT="$ROOT_PREFIX/miner-$MINER_INDEX/chain-data"
    export CONFIG_ROOT="$ROOT_PREFIX/miner-$MINER_INDEX"
    export XDG_CONFIG_HOME=$CONFIG_ROOT/config
    export XDG_DATA_HOME=$CONFIG_ROOT/data
    export HOME=$CONFIG_ROOT

    echo $i $CONFIG_ROOT $HOME

    rm -v -rf $CONFIG_ROOT/first-run-lock
    rm -v -rf $CONFIG_ROOT/parity-config

    RUST_LOG=info ./target/debug/ethereum-launcher &
done

for ((i = 0; i < $TRANSACTOR_COUNT; i++)); do
    export NODE_TYPE=Transactor
    export TRANSACTOR_INDEX=$i

    export P2P_NETWORK_SERVICE_PORT=$(($i + 40303))
    export HTTP_JSON_RPC_PORT=$(($i + 28545))
    export WEBSOCKET_JSON_RPC_PORT=$(($i + 38546))

    export CHAIN_DATA_ROOT="$ROOT_PREFIX/transactor-$TRANSACTOR_INDEX/chain-data"
    export CONFIG_ROOT="$ROOT_PREFIX/transactor-$TRANSACTOR_INDEX"
    export XDG_CONFIG_HOME=$CONFIG_ROOT/config
    export XDG_DATA_HOME=$CONFIG_ROOT/data
    export HOME=$CONFIG_ROOT

    echo $i $CONFIG_ROOT $HOME

    rm -v -rf $CONFIG_ROOT/first-run-lock
    rm -v -rf $CONFIG_ROOT/parity-config

    RUST_LOG=info ./target/debug/ethereum-launcher &
done

while true; do
    sleep 1
done
