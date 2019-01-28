error_chain! {
    foreign_links {
        StdIoError(std::io::Error);
        EnvVarError(std::env::VarError);
        NumParseIntError(std::num::ParseIntError);
        AddrParseError(std::net::AddrParseError);
        JsonParseError(serde_json::Error);
        EthKeyError(ethkey::Error);
        MnemonicError(emerald::mnemonic::Error);
    }

    errors {
        UnknownNodeType(t: String) {
            description("Unknown node type")
            display("Unknown node type: {}", t)
        }
        TooLargeMinerIndex(miner_index: usize, miner_count: usize) {
            description("Too large miner index")
            display("Too larget miner index: {}, miner count: {}", miner_index, miner_count)
        }
        InvalidSealerMasterSeed(s: String) {
            description("Invalid sealer master seed")
            display("Invalid sealer master seed: {}", s)
        }
        InvalidMinerCount(s: String) {
            description("Invalid miner count")
            display("Invalid miner count: {}", s)
        }
        InvalidMinerIndex(s: String) {
            description("Invalid miner index")
            display("Invalid miner index: {}", s)
        }
        InvalidConsensusEngineType(s: String) {
            description("Invalid consensus engine type")
            display("Invalid consensus engine type: {}", s)
        }
        InvalidGasLimitValue(s: String) {
            description("Invalid gas limit value")
            display("Invalid gas limit value: {}", s)
        }
        InvalidPrivateKey(s: String) {
            description("Invalid private key")
            display("Invalid private key: {}", s)
        }
        InvalidHDPath(s: String) {
            description("Invalid HD path")
            display("Invalid HD path: {}", s)
        }
        InvalidMnemonicPhrase(phrase: String) {
            description("Invalid mnemonic phrase")
            display("Invalid mnemonic phrase {}", phrase)
        }
        FailedToGeneratePrivateKey(seed: Vec<u8>, path: String) {
            description("Failed to generate private key")
            display("Failed to generate private key from seed {:?} and path {}", seed, path)
        }
    }
}
