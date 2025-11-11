#[derive(Debug, Clone)]
pub struct ConnectionError {
    description: String,
}

#[derive(Debug, Clone)]
pub struct IrcParseError {
    description: String,
}

impl ConnectionError {
    pub fn new(description: String) -> Self {
        Self { description }
    }
}

impl std::fmt::Display for ConnectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ConnectionError: {}", self.description)
    }
}

impl std::error::Error for ConnectionError {
    fn description(&self) -> &str {
        &self.description
    }
}

impl IrcParseError {
    pub fn new() -> Self {
        Self {
            description: String::new(),
        }
    }
}

impl std::fmt::Display for IrcParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ParseError: {}", self.description)
    }
}

impl std::error::Error for IrcParseError {
    fn description(&self) -> &str {
        &self.description
    }
}
