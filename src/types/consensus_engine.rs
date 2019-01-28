use ethereum_types::U256;

use super::EthereumProgram;

#[derive(Debug, Clone, Copy)]
pub enum ConsensusEngine {
    Ethash {
        genesis_difficulty: U256,
    },
    ParityAura {
        block_period: u64,
        block_reward: U256,
    },
    GethClique {
        block_period: u64,
    },
}

impl ConsensusEngine {
    pub fn program(&self) -> EthereumProgram {
        match self {
            ConsensusEngine::Ethash { .. } => EthereumProgram::Parity,
            ConsensusEngine::ParityAura { .. } => EthereumProgram::Parity,
            ConsensusEngine::GethClique { .. } => EthereumProgram::GoEthereum,
        }
    }
}
