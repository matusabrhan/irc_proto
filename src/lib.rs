pub mod channel;
pub mod command;
pub mod connection;
pub mod message;
pub mod types;

pub fn enable_logging() {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "debug");
    }
    let _ = env_logger::try_init();
}
