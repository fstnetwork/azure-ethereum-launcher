error_chain! {
    foreign_links {
        AddrParse(std::net::AddrParseError);
        Hyper(hyper::Error);
        Timer(tokio_timer::Error);
        Json(serde_json::Error);
        UrlParse(url::ParseError);
    }

    errors {
        InvalidStateTransfer(current_state: String, expected_state: String) {
            description("Invalid state transfer")
            display("Invalid state transfer, current: {}, expected: {}", current_state, expected_state)
        }
        JsonRpc(t: jsonrpc_core::Error) {
            description("JSON RPC Error")
            display("JSON RPC Error: {:?}", t)
        }
    }
}
