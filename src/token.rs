#[derive(Debug, PartialEq, Eq, Default, Clone)]
pub(crate) struct Token {
    start: u16,
    length: u16,
    kind: TokenKind,
}

#[derive(Debug, PartialEq, Eq, Default, Clone)]
pub(crate) enum TokenKind {
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
    Period,
    Comma,
    Minus,
    Plus,
    EOM,
    #[default]
    Invalid,
}

impl Token {
    pub(crate) fn new(kind: TokenKind, start: u16, length: u16) -> Token {
        Self {
            kind,
            start,
            length,
        }
    }

    pub(crate) fn kind(&self) -> TokenKind {
        self.kind.clone()
    }

    pub(crate) fn start(&self) -> u16 {
        self.start
    }

    pub(crate) fn length(&self) -> u16 {
        self.length
    }
}
