use futures::{Async, Future, Poll, Stream};

use tokio_process::Child as ChildProcess;

use super::Error;
use super::{EthereumLauncher, RestartPolicy};

pub struct Service {
    restart_policy: RestartPolicy,
    ethereum_launcher: EthereumLauncher,
    ethereum_process: Option<ChildProcess>,
}

impl Service {
    pub fn new(ethereum_launcher: EthereumLauncher, restart_policy: RestartPolicy) -> Service {
        let ethereum_process = Some(
            ethereum_launcher
                .execute_async()
                .expect("spawn Ethereum client process"),
        );

        Service {
            restart_policy,
            ethereum_launcher,
            ethereum_process,
        }
    }

    pub fn restart(&mut self) {
        std::mem::replace(
            &mut self.ethereum_process,
            Some(
                self.ethereum_launcher
                    .execute_async()
                    .expect("spawn Ethereum client process"),
            ),
        );
    }

    #[allow(dead_code)]
    pub fn stop(&mut self) {
        std::mem::replace(&mut self.ethereum_process, None);
    }
}

impl Stream for Service {
    type Item = bool;
    type Error = Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Error> {
        loop {
            if let Some(ref mut process) = self.ethereum_process {
                match process.poll() {
                    Ok(Async::Ready(exit_status)) => match self.restart_policy {
                        RestartPolicy::No => return Ok(Async::Ready(Some(exit_status.success()))),
                        RestartPolicy::OnFailure | RestartPolicy::Always => {
                            self.restart();
                            return Ok(Async::Ready(Some(exit_status.success())));
                        }
                    },
                    Ok(Async::NotReady) => {
                        return Ok(Async::NotReady);
                    }
                    Err(err) => return Err(Error::from(err)),
                }
            }

            return Ok(Async::Ready(Some(true)));
        }
    }
}
