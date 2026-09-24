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
}

pub mod strings {
    pub const PING: &[u8] = "PING".as_bytes();
    pub const PONG: &[u8] = "PONG".as_bytes();
    pub const CAP: &[u8] = "CAP".as_bytes();
    pub const PASS: &[u8] = "PASS".as_bytes();
    pub const NICK: &[u8] = "NICK".as_bytes();
    pub const USER: &[u8] = "USER".as_bytes();
    pub const QUIT: &[u8] = "QUIT".as_bytes();
    pub const JOIN: &[u8] = "JOIN".as_bytes();
    pub const PRIVMSG: &[u8] = "PRIVMSG".as_bytes();

    pub const RPL_WELCOME: &[u8] = "001".as_bytes();
    pub const RPL_YOURHOST: &[u8] = "002".as_bytes();
    pub const RPL_CREATED: &[u8] = "003".as_bytes();
    pub const RPL_MYINFO: &[u8] = "004".as_bytes();

    pub const ERR_PASSWDMISMATCH: &[u8] = "464".as_bytes();
    pub const ERR_NICKNAMEINUSE: &[u8] = "433".as_bytes();

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
    pub const QUESTION_MARK: u8 = b'?';
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
    pub const DOLLAR_SIGN: u8 = b'$';
    pub const NULL: u8 = b'\0';
}

pub fn enable_logging() {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "debug");
    }
    env_logger::try_init();
}

pub fn is_valid_command(input: &[u8]) -> bool {
    let mut parser = Parser::new(input);
    parser.parse_command().is_ok()
}

pub fn is_valid_source(input: &[u8]) -> bool {
    let mut parser = Parser::new(input);
    parser.parse_source().is_ok()
}

pub fn is_valid_tags(input: &[u8]) -> bool {
    let mut parser = Parser::new(input);
    parser.parse_tags().is_ok()
}
