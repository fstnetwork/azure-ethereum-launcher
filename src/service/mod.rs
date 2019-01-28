use futures::{Async, Poll, Stream};
use std::time::Duration;
use tokio_timer::Interval;

use super::bootnode::{Error as BootnodeServiceError, Service as BootnodeService};
use super::ethereum::{Error as EthereumError, Service as EthereumService};

mod error;

pub use self::error::{Error, ErrorKind};

pub struct Service {
    ethereum: EthereumService,
    bootnode: BootnodeService,
    ticker: Interval,
}

impl Service {
    pub fn new(
        ethereum: EthereumService,
        bootnode: BootnodeService,
        bootnode_update_interval: Duration,
    ) -> Service {
        let ticker = Interval::new_interval(bootnode_update_interval);

        // force update
        let mut bootnode = bootnode;
        bootnode.send_event();

        Service {
            ethereum,
            bootnode,
            ticker,
        }
    }
}

impl Stream for Service {
    type Item = ();
    type Error = Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        loop {
            match self.ethereum.poll() {
                Ok(Async::Ready(_)) => {
                    return Ok(Async::Ready(Some(())));
                }
                Ok(Async::NotReady) => {}
                Err(err) => return Err(Error::from(err)),
            }

            if let Err(err) = self.bootnode.poll() {
                return Err(Error::from(err));
            }

            match self.ticker.poll() {
                Ok(Async::Ready(_)) => {
                    self.bootnode.send_event();
                }
                Ok(Async::NotReady) => return Ok(Async::NotReady),
                Err(err) => return Err(Error::from(err)),
            }
        }
    }
}
