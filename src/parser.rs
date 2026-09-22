use crate::{
    ast::{Node, NodeId, NodeKind},
    lexer::Lexer,
    strings,
    token::{Token, TokenKind},
};

pub struct Parser<'a> {
    input: &'a str,
    lexer: Lexer<'a>,
    current: Token,
    peek: Token,
    nodes: Vec<Node>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ParserErrorKind {
    InvalidToken,
    MissingEndOfMessage,
    ExpectedToken(TokenKind),
    UnexpectedToken,
    MissingCommand,
    UnknownCommand,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ParserError {
    pub offset: usize,
    pub kind: ParserErrorKind,
}

impl<'a> Parser<'a> {
    pub(crate) fn new(input: &'a str) -> Self {
        let mut lexer = Lexer::new(input);
        Self {
            input,
            current: lexer.next_token(),
            peek: lexer.next_token(),
            lexer,
            nodes: Vec::with_capacity(16),
        }
    }

    fn next_token(&mut self) -> Result<Token, ParserError> {
        let last: Token = std::mem::take(&mut self.current);
        self.current = std::mem::replace(&mut self.peek, self.lexer.next_token());
        if self.current.kind() == TokenKind::Invalid {
            return Err(ParserError {
                offset: self.current.start() as usize,
                kind: ParserErrorKind::InvalidToken,
            });
        }
        Ok(last)
    }

    fn store_node(&mut self, kind: NodeKind, start: u16, length: u16) -> NodeId {
        let id = NodeId(self.nodes.len() as u8);
        self.nodes.push(Node::new(kind, start, length));
        id
    }

    pub(crate) fn parse_message(&mut self) -> Result<NodeId, ParserError> {
        let tags = match self.current.kind() == TokenKind::At {
            true => {
                let tags = Some(self.parse_tags()?);
                self.next_token()?;
                tags
            }
            false => None,
        };

        let source = match self.current.kind() == TokenKind::Colon {
            true => {
                let source = Some(self.parse_source()?);
                self.next_token()?;
                source
            }
            false => None,
        };

        let command = self.parse_command()?;

        if self.current.kind() != TokenKind::EndOfMessage {
            return Err(ParserError {
                offset: self.current.start() as usize,
                kind: ParserErrorKind::ExpectedToken(TokenKind::EndOfMessage),
            });
        }

        Ok(self.store_node(
            NodeKind::Message {
                tags,
                source,
                command,
            },
            0,
            self.current.start() + self.current.length(),
        ))
    }

    pub(crate) fn parse_tags(&mut self) -> Result<NodeId, ParserError> {
        if self.current.kind() != TokenKind::At {
            return Err(ParserError {
                offset: self.current.start() as usize,
                kind: ParserErrorKind::ExpectedToken(TokenKind::At),
            });
        }
        let start_token = self.next_token()?;

        let mut tag_node_ids = Vec::new();
        tag_node_ids.push(self.parse_tag()?);

        while self.current.kind() == TokenKind::Semicolon {
            self.next_token()?;
            tag_node_ids.push(self.parse_tag()?);
        }

        if self.current.kind() != TokenKind::Space {
            return Err(ParserError {
                offset: self.current.start() as usize,
                kind: ParserErrorKind::ExpectedToken(TokenKind::Space),
            });
        }

        Ok(self.store_node(
            NodeKind::Tags(tag_node_ids.into()),
            start_token.start(),
            self.current.start() + self.current.length() - start_token.start(),
        ))
    }

    fn parse_tag(&mut self) -> Result<NodeId, ParserError> {
        let start = self.current.start();

        let key = self.parse_tag_key()?;
        let value = match self.current.kind() == TokenKind::Equals {
            true => {
                self.next_token()?;
                Some(self.parse_tag_value()?)
            }
            false => None,
        };

        Ok(self.store_node(
            NodeKind::Tag { key, value },
            start,
            self.current.start() - start,
        ))
    }

    fn parse_tag_key(&mut self) -> Result<NodeId, ParserError> {
        let start_token = self.next_token()?;
        loop {
            match self.current.kind() {
                TokenKind::EndOfMessage
                | TokenKind::Space
                | TokenKind::Equals
                | TokenKind::Semicolon => {
                    return Ok(self.store_node(
                        NodeKind::TagKey,
                        start_token.start(),
                        self.current.start() - start_token.start(),
                    ));
                }
                _ => self.next_token()?,
            };
        }
    }

    fn parse_tag_value(&mut self) -> Result<NodeId, ParserError> {
        let start_token = self.next_token()?;
        loop {
            match self.current.kind() {
                TokenKind::EndOfMessage | TokenKind::Space | TokenKind::Semicolon => {
                    return Ok(self.store_node(
                        NodeKind::TagValue,
                        start_token.start(),
                        self.current.start() - start_token.start(),
                    ));
                }
                _ => self.next_token()?,
            };
        }
    }

    pub(crate) fn parse_source(&mut self) -> Result<NodeId, ParserError> {
        if self.current.kind() != TokenKind::Colon {
            return Err(ParserError {
                offset: self.current.start() as usize,
                kind: ParserErrorKind::ExpectedToken(TokenKind::Colon),
            });
        }
        let start_token = self.next_token()?;

        let name = self.parse_source_name()?;

        let user = match self.current.kind() == TokenKind::Bang {
            true => Some(self.parse_source_user()?),
            false => None,
        };

        let host = match self.current.kind() == TokenKind::At {
            true => Some(self.parse_source_host()?),
            false => None,
        };

        if self.current.kind() != TokenKind::Space {
            return Err(ParserError {
                offset: self.current.start() as usize,
                kind: ParserErrorKind::ExpectedToken(TokenKind::Space),
            });
        }

        Ok(self.store_node(
            NodeKind::Source { name, user, host },
            start_token.start(),
            (self.current.start() + self.current.length()) - start_token.start(),
        ))
    }

    fn parse_source_name(&mut self) -> Result<NodeId, ParserError> {
        if TokenKind::DollarSign == self.current.kind() || TokenKind::Colon == self.current.kind() {
            return Err(ParserError {
                offset: self.current.start() as usize,
                kind: ParserErrorKind::UnexpectedToken,
            });
        }
        let start_token = self.next_token()?;
        loop {
            match self.current.kind() {
                TokenKind::EndOfMessage | TokenKind::Space | TokenKind::Bang | TokenKind::At => {
                    return Ok(self.store_node(
                        NodeKind::SourceName,
                        start_token.start(),
                        self.current.start() - start_token.start(),
                    ));
                }
                TokenKind::Comma | TokenKind::Asterisk | TokenKind::QuestionMark => {
                    return Err(ParserError {
                        offset: self.current.start() as usize,
                        kind: ParserErrorKind::UnexpectedToken,
                    })
                }
                _ => {
                    self.next_token()?;
                }
            }
        }
    }

    fn parse_source_user(&mut self) -> Result<NodeId, ParserError> {
        self.next_token()?;
        let start_token = self.next_token()?;
        loop {
            match self.current.kind() {
                TokenKind::EndOfMessage | TokenKind::Space | TokenKind::At => {
                    return Ok(self.store_node(
                        NodeKind::SourceUser,
                        start_token.start(),
                        self.current.start() - start_token.start(),
                    ));
                }
                _ => self.next_token()?,
            };
        }
    }

    fn parse_source_host(&mut self) -> Result<NodeId, ParserError> {
        self.next_token()?;
        let start_token = self.next_token()?;
        loop {
            match self.current.kind() {
                TokenKind::EndOfMessage | TokenKind::Space => {
                    return Ok(self.store_node(
                        NodeKind::SourceHost,
                        start_token.start(),
                        self.current.start() - start_token.start(),
                    ));
                }
                _ => {
                    self.next_token()?;
                }
            }
        }
    }

    fn pop_or_missing(&self, param_ids: &mut Vec<NodeId>) -> Result<NodeId, ParserError> {
        param_ids.pop().ok_or(ParserError {
            offset: self.current.start() as usize,
            kind: ParserErrorKind::MissingCommand,
        })
    }

    pub(crate) fn parse_command(&mut self) -> Result<NodeId, ParserError> {
        let start_token = self.next_token()?;
        let command_str = match self
            .input
            .get(
                start_token.start() as usize..(start_token.start() + start_token.length()) as usize,
            )
            .map(|s| s.to_uppercase())
        {
            Some(command) => command,
            None => unreachable!(),
        };

        let mut param_ids = Vec::new();
        while self.current.kind() == TokenKind::Space {
            self.next_token()?;
            param_ids.push(self.parse_param()?);
        }
        param_ids.reverse();

        match command_str.as_str() {
            strings::PING => Ok(self.store_node(
                NodeKind::CommandPing {
                    token: self.pop_or_missing(&mut param_ids)?,
                },
                start_token.start(),
                self.current.start() - start_token.start(),
            )),
            strings::PONG => {
                let param1 = self.pop_or_missing(&mut param_ids)?;
                if let Some(param2) = param_ids.pop() {
                    return Ok(self.store_node(
                        NodeKind::CommandPong {
                            server: Some(param1),
                            token: param2,
                        },
                        start_token.start(),
                        self.current.start() - start_token.start(),
                    ));
                }

                Ok(self.store_node(
                    NodeKind::CommandPong {
                        server: None,
                        token: param1,
                    },
                    start_token.start(),
                    self.current.start() - start_token.start(),
                ))
            }

            strings::CAP => Ok(self.store_node(
                NodeKind::CommandCap {
                    subcommand: self.pop_or_missing(&mut param_ids)?,
                    capabilities: param_ids.pop(),
                },
                start_token.start(),
                self.current.start() - start_token.start(),
            )),
            strings::PASS => Ok(self.store_node(
                NodeKind::CommandPass {
                    password: self.pop_or_missing(&mut param_ids)?,
                },
                start_token.start(),
                self.current.start() - start_token.start(),
            )),
            strings::NICK => Ok(self.store_node(
                NodeKind::CommandNick {
                    nickname: self.pop_or_missing(&mut param_ids)?,
                },
                start_token.start(),
                self.current.start() - start_token.start(),
            )),
            strings::USER => Ok(self.store_node(
                NodeKind::CommandUser {
                    user: self.pop_or_missing(&mut param_ids)?,
                    mode: self.pop_or_missing(&mut param_ids)?,
                    unused: self.pop_or_missing(&mut param_ids)?,
                    realname: self.pop_or_missing(&mut param_ids)?,
                },
                start_token.start(),
                self.current.start() - start_token.start(),
            )),
            strings::QUIT => Ok(self.store_node(
                NodeKind::CommandQuit {
                    reason: param_ids.pop(),
                },
                start_token.start(),
                self.current.start() - start_token.start(),
            )),

            strings::JOIN => Ok(self.store_node(
                NodeKind::CommandJoin {
                    channels: self.pop_or_missing(&mut param_ids)?,
                    keys: param_ids.pop(),
                },
                start_token.start(),
                self.current.start() - start_token.start(),
            )),
            strings::PRIVMSG => Ok(self.store_node(
                NodeKind::CommandPrivMsg {
                    targets: self.pop_or_missing(&mut param_ids)?,
                    text: self.pop_or_missing(&mut param_ids)?,
                },
                start_token.start(),
                self.current.start() - start_token.start(),
            )),

            strings::RPL_WELCOME => Ok(self.store_node(
                NodeKind::RplWelcome {
                    client: self.pop_or_missing(&mut param_ids)?,
                    text: self.pop_or_missing(&mut param_ids)?,
                },
                start_token.start(),
                self.current.start() - start_token.start(),
            )),
            strings::RPL_YOURHOST => Ok(self.store_node(
                NodeKind::RplYourhost {
                    client: self.pop_or_missing(&mut param_ids)?,
                    text: self.pop_or_missing(&mut param_ids)?,
                },
                start_token.start(),
                self.current.start() - start_token.start(),
            )),
            strings::RPL_CREATED => Ok(self.store_node(
                NodeKind::RplCreated {
                    client: self.pop_or_missing(&mut param_ids)?,
                    text: self.pop_or_missing(&mut param_ids)?,
                },
                start_token.start(),
                self.current.start() - start_token.start(),
            )),
            strings::RPL_MYINFO => Ok(self.store_node(
                NodeKind::RplMyinfo {
                    client: self.pop_or_missing(&mut param_ids)?,
                    servername: self.pop_or_missing(&mut param_ids)?,
                    version: self.pop_or_missing(&mut param_ids)?,
                    user_modes: self.pop_or_missing(&mut param_ids)?,
                    channel_modes: self.pop_or_missing(&mut param_ids)?,
                },
                start_token.start(),
                self.current.start() - start_token.start(),
            )),

            strings::ERR_PASSWDMISMATCH => Ok(self.store_node(
                NodeKind::ErrPasswdmismatch {
                    client: self.pop_or_missing(&mut param_ids)?,
                },
                start_token.start(),
                self.current.start() - start_token.start(),
            )),

            strings::ERR_NICKNAMEINUSE => Ok(self.store_node(
                NodeKind::ErrNicknameinuse {
                    client: self.pop_or_missing(&mut param_ids)?,
                    nick: self.pop_or_missing(&mut param_ids)?,
                },
                start_token.start(),
                self.current.start() - start_token.start(),
            )),

            _ => Err(ParserError {
                offset: start_token.start() as usize,
                kind: ParserErrorKind::MissingCommand,
            }),
        }
    }

    fn parse_param(&mut self) -> Result<NodeId, ParserError> {
        let trailing = self.current.kind() == TokenKind::Colon;
        if trailing {
            self.next_token()?;
        }
        let start_token = self.next_token()?;
        loop {
            match self.current.kind() {
                TokenKind::Space => {
                    if trailing {
                        self.next_token()?;
                        continue;
                    }
                    let param = self.store_node(
                        NodeKind::Parameter,
                        start_token.start(),
                        self.current.start() - start_token.start(),
                    );

                    return Ok(param);
                }

                TokenKind::EndOfMessage => {
                    return Ok(self.store_node(
                        NodeKind::Parameter,
                        start_token.start(),
                        self.current.start() - start_token.start(),
                    ));
                }

                _ => {
                    self.next_token()?;
                }
            }
        }
    }

    pub(crate) fn get_nodes(self) -> Box<[Node]> {
        self.nodes.into_boxed_slice()
    }
}

mod tests {

    use crate::{
        ast::{Node, NodeId, NodeKind},
        enable_logging,
        parser::Parser,
        token::{Token, TokenKind},
    };

    #[test]
    fn test_parser_tags1() {
        let input = "@id=234AB;foo \r\n";
        let mut parser = Parser::new(input);
        assert_eq!(Ok(NodeId(5)), parser.parse_tags());
        let expected = vec![
            Node::new(NodeKind::TagKey, 1, 2),
            Node::new(NodeKind::TagValue, 4, 5),
            Node::new(
                NodeKind::Tag {
                    key: NodeId(0),
                    value: Some(NodeId(1)),
                },
                1,
                8,
            ),
            Node::new(NodeKind::TagKey, 10, 3),
            Node::new(
                NodeKind::Tag {
                    key: NodeId(3),
                    value: None,
                },
                10,
                3,
            ),
            Node::new(NodeKind::Tags(vec![NodeId(2), NodeId(4)].into()), 0, 14),
        ];

        assert_eq!(expected.len(), parser.nodes.len());
        for i in 0..expected.len() {
            assert_eq!(expected[i], parser.nodes[i])
        }
    }

    #[test]
    fn test_parse_source1() {
        let input = ":dan!d@localhost ";
        let mut parser = Parser::new(input);
        assert_eq!(Ok(NodeId(3)), parser.parse_source());
        let expected = vec![
            Node::new(NodeKind::SourceName, 1, 3),
            Node::new(NodeKind::SourceUser, 5, 1),
            Node::new(NodeKind::SourceHost, 7, 9),
            Node::new(
                NodeKind::Source {
                    name: NodeId(0),
                    user: Some(NodeId(1)),
                    host: Some(NodeId(2)),
                },
                0,
                17,
            ),
        ];

        assert_eq!(expected.len(), parser.nodes.len());
        for i in 0..expected.len() {
            assert_eq!(expected[i], parser.nodes[i])
        }
    }

    #[test]
    fn test_parse_command1() {
        let input = "PRIVMSG #chan :Hey what's up!\r\n";
        let mut parser = Parser::new(input);
        assert_eq!(Ok(NodeId(2)), parser.parse_command());
        let expected = vec![
            Node::new(NodeKind::Parameter, 8, 5),
            Node::new(NodeKind::Parameter, 15, 14),
            Node::new(
                NodeKind::CommandPrivMsg {
                    targets: NodeId(0),
                    text: NodeId(1),
                },
                0,
                29,
            ),
        ];

        assert_eq!(expected.len(), parser.nodes.len());
        for i in 0..expected.len() {
            assert_eq!(expected[i], parser.nodes[i])
        }
    }

    #[test]
    fn test_parse_message1() {
        let input = "@id=234AB;foo :dan!d@localhost PRIVMSG #chan :Hey what's up!\r\n";
        let mut parser = Parser::new(input);
        assert_eq!(Ok(NodeId(13)), parser.parse_message());
        let expected = vec![
            Node::new(NodeKind::TagKey, 1, 2),
            Node::new(NodeKind::TagValue, 4, 5),
            Node::new(
                NodeKind::Tag {
                    key: NodeId(0),
                    value: Some(NodeId(1)),
                },
                1,
                8,
            ),
            Node::new(NodeKind::TagKey, 10, 3),
            Node::new(
                NodeKind::Tag {
                    key: NodeId(3),
                    value: None,
                },
                10,
                3,
            ),
            Node::new(NodeKind::Tags(vec![NodeId(2), NodeId(4)].into()), 0, 14),
            Node::new(NodeKind::SourceName, 15, 3),
            Node::new(NodeKind::SourceUser, 19, 1),
            Node::new(NodeKind::SourceHost, 21, 9),
            Node::new(
                NodeKind::Source {
                    name: NodeId(6),
                    user: Some(NodeId(7)),
                    host: Some(NodeId(8)),
                },
                14,
                17,
            ),
            Node::new(NodeKind::Parameter, 39, 5),
            Node::new(NodeKind::Parameter, 46, 14),
            Node::new(
                NodeKind::CommandPrivMsg {
                    targets: NodeId(10),
                    text: NodeId(11),
                },
                31,
                29,
            ),
            Node::new(
                NodeKind::Message {
                    tags: Some(NodeId(5)),
                    source: Some(NodeId(9)),
                    command: NodeId(12),
                },
                0,
                62,
            ),
        ];

        assert_eq!(expected.len(), parser.nodes.len());
        for i in 0..expected.len() {
            assert_eq!(expected[i], parser.nodes[i])
        }
    }
}
