pub mod ast;
pub mod connection;
pub mod lexer;
pub mod message;
pub mod parser;
pub mod token;

#[cfg(all(feature = "std-stream", feature = "tokio-stream"))]
compile_error!("features `std-stream` and `tokio-stream` are mutually exclusive");

#[cfg(not(any(feature = "std-stream", feature = "tokio-stream")))]
compile_error!("enable either `std-stream` or `tokio-stream`");

pub mod strings {
    pub const PING: &str = "PING";
    pub const PONG: &str = "PONG";
    pub const CAP: &str = "CAP";
    pub const PASS: &str = "PASS";
    pub const NICK: &str = "NICK";
    pub const USER: &str = "USER";
    pub const QUIT: &str = "QUIT";
    pub const JOIN: &str = "JOIN";
    pub const PRIVMSG: &str = "PRIVMSG";

    pub const RPL_WELCOME: &str = "001";
    pub const RPL_YOURHOST: &str = "002";
    pub const RPL_CREATED: &str = "003";
    pub const RPL_MYINFO: &str = "004";

    pub const ERR_PASSWDMISMATCH: &str = "464";
    pub const ERR_NICKNAMEINUSE: &str = "433";

    pub const CR: char = '\r';
    pub const LF: char = '\n';
    pub const AT: char = '@';
    pub const COLON: char = ':';
    pub const SEMICOLON: char = ';';
    pub const SPACE: char = ' ';
    pub const LESSER: char = '<';
    pub const EQUALS: char = '=';
    pub const GREATER: char = '>';
    pub const BANG: char = '!';
    pub const SINGLE_QUOTE: char = '\'';
    pub const DOUBLE_QUOTE: char = '"';
    pub const SLASH: char = '/';
    pub const HASH: char = '#';
    pub const STAR: char = '*';
    pub const PERIOD: char = '.';
    pub const COMMA: char = ',';
    pub const DASH: char = '_';
    pub const MINUS: char = '-';
    pub const PLUS: char = '+';
}

pub fn enable_logging() {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "debug");
    }
    env_logger::try_init();
}
