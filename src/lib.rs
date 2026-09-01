pub mod ast;
pub mod connection;
pub mod error;
pub mod lexer;
pub mod message;
pub mod message_v2;
pub mod parser;
pub mod token;

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
    pub const PERIOD: char = '.';
    pub const COMMA: char = ',';
    pub const MINUS: char = '-';
    pub const PLUS: char = '+';
}

pub fn enable_logging() {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "debug");
    }
    env_logger::try_init();
}
