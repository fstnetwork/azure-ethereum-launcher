error_chain! {
    foreign_links {
        StdIoError(std::io::Error);
    }
}
