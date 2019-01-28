use ethereum_types::Address;
use ethkey::{KeyPair, Secret};

use emerald::mnemonic::{self, HDPath, Language, Mnemonic};

use super::{Error, ErrorKind};

#[derive(Debug, Clone)]
pub enum NodeType {
    Miner {
        index: usize,
        miner_count: usize,
        sealer_mnemonic: String,
    },
    Transactor {
        miner_count: usize,
        sealer_mnemonic: String,
    },
}

impl NodeType {
    #[allow(dead_code)]
    pub fn is_miner(&self) -> bool {
        match self {
            NodeType::Miner { .. } => true,
            _ => false,
        }
    }

    #[allow(dead_code)]
    pub fn is_transactor(&self) -> bool {
        match self {
            NodeType::Transactor { .. } => true,
            _ => false,
        }
    }

    pub fn validators(&self) -> Result<Vec<Address>, Error> {
        Ok(self
            .validator_keypairs()?
            .iter()
            .map(KeyPair::address)
            .collect())
    }

    pub fn validator_keypairs(&self) -> Result<Vec<KeyPair>, Error> {
        let (miner_count, sealer_mnemonic) = match self {
            NodeType::Miner {
                miner_count,
                sealer_mnemonic,
                ..
            } => (miner_count, sealer_mnemonic),
            NodeType::Transactor {
                miner_count,
                sealer_mnemonic,
            } => (miner_count, sealer_mnemonic),
        };

        keypair_from_sealer_mnemonic(&sealer_mnemonic, *miner_count)
    }
}

fn keypair_from_sealer_mnemonic(
    sealer_mnemonic: &String,
    sealer_count: usize,
) -> Result<Vec<KeyPair>, Error> {
    let mnemonic = match Mnemonic::try_from(Language::English, sealer_mnemonic) {
        Ok(m) => m,
        Err(_) => {
            return Err(Error::from(ErrorKind::InvalidMnemonicPhrase(
                sealer_mnemonic.clone(),
            )))
        }
    };

    let mut keypairs = Vec::new();
    for i in 0..sealer_count {
        let seed = mnemonic.seed("");
        let raw_path = format!("m/44'/60'/0'/0/{}", i);
        let path = match HDPath::try_from(raw_path.as_str()) {
            Ok(path) => path,
            Err(_err) => return Err(Error::from(ErrorKind::InvalidHDPath(raw_path))),
        };
        let priv_key = match mnemonic::generate_key(&path, &seed) {
            Ok(pk) => pk,
            Err(_err) => {
                return Err(Error::from(ErrorKind::FailedToGeneratePrivateKey(
                    seed, raw_path,
                )))
            }
        };
        let secret = match Secret::from_slice(&priv_key) {
            Some(secret) => secret,
            None => {
                return Err(Error::from(ErrorKind::InvalidPrivateKey(format!(
                    "{:x?}",
                    priv_key
                ))))
            }
        };
        keypairs.push(KeyPair::from_secret(secret)?);
    }
    Ok(keypairs)
}
