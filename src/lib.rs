pub mod connection;
pub mod error;
pub mod message;

pub fn enable_logging() {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "debug");
    }
    env_logger::try_init();
}
