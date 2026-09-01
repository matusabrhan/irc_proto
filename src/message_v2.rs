use std::ops::Index;

use crate::{
    ast::{Node, NodeId, NodeKind},
    parser::Parser,
    strings::*,
};

struct Nodes<T>(Box<[T]>);

impl<T> Index<NodeId> for Nodes<T> {
    type Output = T;

    fn index(&self, index: NodeId) -> &Self::Output {
        &self.0[index.0 as usize]
    }
}

pub struct MessageV2 {
    text: Box<str>,
    tags_id: Option<NodeId>,
    source_id: Option<NodeId>,
    command_id: NodeId,
    nodes: Nodes<Node>,
}

#[derive(Debug)]
pub struct Tag<'a> {
    key: &'a str,
    value: Option<&'a str>,
}

#[derive(Debug)]
pub struct Source<'a> {
    name: &'a str,
    user: Option<&'a str>,
    host: Option<&'a str>,
}

#[derive(Debug, PartialEq)]
pub enum Command<'a> {
    PING {
        token: &'a str,
    },
    PONG {
        server: Option<&'a str>,
        token: &'a str,
    },

    CAP {
        subcommand: &'a str,
        capabilities: Option<&'a str>,
    },
    PASS {
        password: &'a str,
    },
    NICK {
        nickname: &'a str,
    },
    USER {
        user: &'a str,
        mode: &'a str,
        unused: &'a str,
        realname: &'a str,
    },

    PRIVMSG {
        targets: &'a str,
        text: &'a str,
    },
}

impl<'a> Command<'a> {
    fn command(&self) -> &str {
        match self {
            Self::PING { .. } => PING,
            Self::PONG { .. } => PONG,

            Self::CAP { .. } => CAP,
            Self::PASS { .. } => PASS,
            Self::NICK { .. } => NICK,
            Self::USER { .. } => USER,
            Self::PRIVMSG { .. } => PRIVMSG,
        }
    }

    fn params(&self) -> Box<[&str]> {
        match self {
            Self::PING { token } => Box::new([token]),
            Self::PONG { server, token } => {
                if let Some(server) = server {
                    return Box::new([server, token]);
                }
                Box::new([token])
            }

            Self::CAP {
                subcommand,
                capabilities,
            } => {
                if let Some(capabilities) = capabilities {
                    return Box::new([subcommand, capabilities]);
                }
                Box::new([subcommand])
            }
            Self::PASS { password } => Box::new([password]),
            Self::NICK { nickname } => Box::new([nickname]),
            Self::USER {
                user,
                mode,
                unused,
                realname,
            } => Box::new([user, mode, unused, realname]),
            Self::PRIVMSG { targets, text } => Box::new([targets, text]),
        }
    }
}

#[derive(Debug)]
pub struct MessageBuilder<'a> {
    tags: Vec<Tag<'a>>,
    source: Option<Source<'a>>,
    command: Command<'a>,
}

impl MessageV2 {
    pub fn new(input: Vec<u8>) -> Option<Self> {
        let text: Box<str> = String::from_utf8(input).ok()?.into_boxed_str();
        let (root, nodes) = {
            let mut parser = Parser::new(text.as_ref());
            (parser.parse_message().ok()?, Nodes(parser.get_nodes()))
        };

        match nodes.index(root).kind() {
            NodeKind::Message {
                tags,
                source,
                command,
            } => Some(Self {
                text,
                tags_id: tags.clone(),
                source_id: source.clone(),
                command_id: command.clone(),
                nodes,
            }),
            _ => None,
        }
    }

    fn get_node(&self, id: NodeId) -> &Node {
        self.nodes.index(id)
    }

    fn get_value(&self, id: NodeId) -> &str {
        let node = self.nodes.index(id);
        &self.text[node.start()..node.start() + node.length()]
    }

    pub fn contents(&self) -> &str {
        &self.text
    }

    pub fn get_command(&self) -> Command<'_> {
        match self.get_node(self.command_id.clone()).kind() {
            NodeKind::CommandPing { token } => Command::PING {
                token: self.get_value(token.clone()),
            },
            NodeKind::CommandPong { server, token } => {
                let server = server.as_ref().map(|server| self.get_value(server.clone()));
                Command::PONG {
                    server,
                    token: self.get_value(token.clone()),
                }
            }
            NodeKind::CommandCap {
                subcommand,
                capabilities,
            } => {
                let capabilities = capabilities
                    .as_ref()
                    .map(|capabilities| self.get_value(capabilities.clone()));

                Command::CAP {
                    subcommand: self.get_value(subcommand.clone()),
                    capabilities,
                }
            }
            NodeKind::CommandPass { password } => Command::PASS {
                password: self.get_value(password.clone()),
            },
            NodeKind::CommandNick { nickname } => Command::NICK {
                nickname: self.get_value(nickname.clone()),
            },
            NodeKind::CommandUser {
                user,
                mode,
                unused,
                realname,
            } => Command::USER {
                user: self.get_value(user.clone()),
                mode: self.get_value(mode.clone()),
                unused: self.get_value(unused.clone()),
                realname: self.get_value(realname.clone()),
            },

            NodeKind::CommandPrivMsg { targets, text } => Command::PRIVMSG {
                targets: self.get_value(targets.clone()),
                text: self.get_value(text.clone()),
            },

            _ => unreachable!(),
        }
    }

    pub fn get_source(&self) -> Option<Source<'_>> {
        if let Some(source) = self.source_id.clone() {
            match self.get_node(source).kind() {
                NodeKind::Source { name, user, host } => {
                    return Some(Source {
                        name: self.get_value(name.clone()),
                        user: user.as_ref().map(|id| self.get_value(id.clone())),
                        host: host.as_ref().map(|id| self.get_value(id.clone())),
                    })
                }
                _ => unreachable!(),
            }
        }
        None
    }

    pub fn get_tags(&self) -> Option<Box<[Tag<'_>]>> {
        if let Some(tags) = self.tags_id.clone() {
            match self.get_node(tags).kind() {
                NodeKind::Tags(tag_ids) => {
                    let tags: Vec<Tag<'_>> = tag_ids
                        .iter()
                        .filter_map(|tag_id| match self.get_node(tag_id.clone()).kind() {
                            NodeKind::Tag { key, value } => Some(Tag {
                                key: self.get_value(key.clone()),
                                value: value.as_ref().map(|id| self.get_value(id.clone())),
                            }),
                            _ => None,
                        })
                        .collect();
                    return Some(tags.into_boxed_slice());
                }
                _ => unreachable!(),
            }
        }
        None
    }
}

impl<'a> MessageBuilder<'a> {
    pub fn with_command(command: Command<'a>) -> Self {
        Self {
            tags: Vec::new(),
            source: None,
            command,
        }
    }

    pub fn with_source(self, name: &'a str, user: Option<&'a str>, host: Option<&'a str>) -> Self {
        Self {
            tags: self.tags,
            source: Some(Source { name, user, host }),
            command: self.command,
        }
    }

    pub fn with_tag(self, key: &'a str, value: Option<&'a str>) -> Self {
        let mut tags: Vec<Tag> = self.tags;
        tags.push(Tag { key, value });
        Self {
            tags,
            source: self.source,
            command: self.command,
        }
    }

    pub fn build(self) -> Option<MessageV2> {
        let mut buffer: Vec<u8> = Vec::with_capacity(1024);

        if !self.tags.is_empty() {
            buffer.push(AT as u8);
        }
        for tag in self.tags {
            buffer.extend_from_slice(tag.key.as_bytes());
            if let Some(value) = tag.value.as_ref() {
                buffer.extend_from_slice(value.as_bytes());
            }
        }

        if let Some(source) = self.source {
            buffer.push(COLON as u8);
            buffer.extend_from_slice(source.name.as_bytes());
            if let Some(user) = &source.user {
                buffer.push(BANG as u8);
                buffer.extend_from_slice(user.as_bytes());
            }
            if let Some(host) = source.host {
                buffer.push(AT as u8);
                buffer.extend_from_slice(host.as_bytes());
            }
            buffer.push(SPACE as u8);
        }

        buffer.extend_from_slice(self.command.command().as_bytes());

        for param in self.command.params() {
            buffer.push(SPACE as u8);
            if param.contains(" ") {
                buffer.push(COLON as u8);
            }
            buffer.extend_from_slice(param.as_bytes());
        }

        buffer.push(CR as u8);
        buffer.push(LF as u8);

        MessageV2::new(buffer)
    }
}

#[cfg(test)]
mod tests {

    use crate::message_v2::{Command, MessageBuilder, MessageV2};

    #[test]
    fn test_message_parse1() {
        let input = "@id=234AB :dan!d@localhost PRIVMSG #chan :Hey what's up!\r\n"
            .as_bytes()
            .to_vec();
        let message = MessageV2::new(input).unwrap();

        assert_eq!(
            message.get_command(),
            Command::PRIVMSG {
                targets: "#chan",
                text: "Hey what's up!"
            }
        )
    }

    #[test]
    fn test_message_parse2() {
        let input = ":irc.example.com CAP REQ :multi-prefix extended-join sasl\r\n"
            .as_bytes()
            .to_vec();
        let message = MessageV2::new(input).unwrap();

        assert_eq!(
            message.get_command(),
            Command::CAP {
                subcommand: "REQ",
                capabilities: Some("multi-prefix extended-join sasl")
            }
        )
    }

    #[test]
    fn test_message_parse3() {
        let input = ":irc.example.com PONG server1 token\r\n"
            .as_bytes()
            .to_vec();
        let message = MessageV2::new(input).unwrap();

        assert_eq!(
            message.get_command(),
            Command::PONG {
                server: Some("server1"),
                token: "token"
            }
        )
    }

    #[test]
    fn test_message_parse4() {
        let input = ":irc.example.com USER username1 0 * realname1\r\n"
            .as_bytes()
            .to_vec();
        let message = MessageV2::new(input).unwrap();

        assert_eq!(
            message.get_command(),
            Command::USER {
                user: "username1",
                mode: "0",
                unused: "*",
                realname: "realname1"
            }
        )
    }

    #[test]
    fn test_message_builder1() {
        let message_builder = MessageBuilder::with_command(Command::PRIVMSG {
            targets: "target1,target2",
            text: "aaaa bbbbb cccccc",
        });

        let message = message_builder.build().unwrap();

        assert_eq!(
            message.get_command(),
            Command::PRIVMSG {
                targets: "target1,target2",
                text: "aaaa bbbbb cccccc"
            }
        )
    }

    #[test]
    fn test_message_builder2() {
        let message_builder = MessageBuilder::with_command(Command::PING { token: "token" })
            .with_source("irc.example.com", None, None);

        let message = message_builder.build().unwrap();

        assert_eq!(message.get_command(), Command::PING { token: "token" })
    }
}
