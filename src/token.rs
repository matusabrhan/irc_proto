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
    Lesser,
    Equals,
    Greater,
    Bang,
    QuestionMark,
    Hash,
    SingleQuote,
    DoubleQuote,
    Slash,
    Asterisk,
    Period,
    Comma,
    Dash,
    Minus,
    Plus,
    DollarSign,
    EndOfMessage,
    #[default]
    Null,
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
