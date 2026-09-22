use crate::parser::{Parser, ParserError};

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

#[derive(Debug)]
pub enum IrcError {
    ParseError(ParserError),
    ConnectionError,
    NonUtf8Input,
}

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

    pub const CR: u8 = b'\r';
    pub const LF: u8 = b'\n';
    pub const AT: u8 = b'@';
    pub const COLON: u8 = b':';
    pub const SEMICOLON: u8 = b';';
    pub const SPACE: u8 = b' ';
    pub const LESSER: u8 = b'<';
    pub const EQUALS: u8 = b'=';
    pub const GREATER: u8 = b'>';
    pub const BANG: u8 = b'!';
    pub const QUESTION_MARK: u8 = b'!';
    pub const SINGLE_QUOTE: u8 = b'\'';
    pub const DOUBLE_QUOTE: u8 = b'"';
    pub const SLASH: u8 = b'/';
    pub const HASH: u8 = b'#';
    pub const ASTERISK: u8 = b'*';
    pub const PERIOD: u8 = b'.';
    pub const COMMA: u8 = b',';
    pub const DASH: u8 = b'_';
    pub const MINUS: u8 = b'-';
    pub const PLUS: u8 = b'+';
    pub const DOLLAR_SIGN: u8 = b'+';
}

pub fn enable_logging() {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "debug");
    }
    env_logger::try_init();
}

pub fn is_valid_command(input: &str) -> bool {
    let mut parser = Parser::new(input);
    parser.parse_command().is_ok()
}

pub fn is_valid_source(input: &str) -> bool {
    let mut parser = Parser::new(input);
    parser.parse_source().is_ok()
}

pub fn is_valid_tags(input: &str) -> bool {
    let mut parser = Parser::new(input);
    parser.parse_tags().is_ok()
}
