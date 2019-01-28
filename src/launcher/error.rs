error_chain! {
    foreign_links {
        StdIo(std::io::Error);
        EmeraldKeyStore(emerald::keystore::Error);
        SerdeJson(serde_json::Error);
        Type(super::types::Error);
    }

    errors {

    }
}
