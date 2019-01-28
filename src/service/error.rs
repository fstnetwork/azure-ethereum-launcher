error_chain! {
    foreign_links {
        EthereumError(super::EthereumError);
        BootnodeServiceError(super::BootnodeServiceError);
        TimerError(tokio_timer::Error);
    }

    errors {
    }
}
