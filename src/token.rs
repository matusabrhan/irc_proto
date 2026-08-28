#[derive(Debug, PartialEq, Eq, Default, Clone)]
pub struct Token {
    start: u16,
    length: u16,
    kind: TokenKind,
}

#[derive(Debug, PartialEq, Eq, Default, Clone)]
pub enum TokenKind {
    Text,
    Space,
    At,
    Colon,
    Semicolon,
    Equals,
    Bang,
    Hash,
    SingleQuote,
    DoubleQuote,
    Slash,
    Star,
    EOM,
    #[default]
    Invalid,
}

impl Token {
    pub fn new(kind: TokenKind, start: u16, length: u16) -> Token {
        Self {
            kind,
            start,
            length,
        }
    }

    pub fn kind(&self) -> TokenKind {
        self.kind.clone()
    }

    pub fn start(&self) -> u16 {
        self.start
    }

    pub fn length(&self) -> u16 {
        self.length
    }
}
