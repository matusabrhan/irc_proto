pub mod ast;
pub mod connection;
pub mod error;
pub mod lexer;
pub mod message;
pub mod message_v2;
pub mod parser;
pub mod token;

pub mod strings {
    pub const PING: &'static str = "PING";
    pub const PONG: &'static str = "PONG";
    pub const CAP: &'static str = "CAP";
    pub const USER: &'static str = "USER";
    pub const PRIVMSG: &'static str = "PRIVMSG";

    pub const CR: char = '\r';
    pub const LF: char = '\n';
    pub const AT: char = '@';
    pub const COLON: char = ':';
    pub const SEMICOLON: char = ';';
    pub const SPACE: char = ' ';
    pub const EQUALS: char = '=';
    pub const BANG: char = '!';
    pub const SINGLE_QUOTE: char = '\'';
    pub const DOUBLE_QUOTE: char = '"';
    pub const SLASH: char = '/';
    pub const HASH: char = '#';
    pub const STAR: char = '*';
}

pub fn enable_logging() {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "debug");
    }
    env_logger::try_init();
}
