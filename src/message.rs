use std::ops::Index;

use crate::{
    ast::{Node, NodeId, NodeKind},
    parser::Parser,
    strings, IrcError,
};

#[derive(Debug, Clone)]
struct Nodes<T>(Box<[T]>);

impl<T> Index<NodeId> for Nodes<T> {
    type Output = T;

    fn index(&self, index: NodeId) -> &Self::Output {
        &self.0[index.0 as usize]
    }
}

#[derive(Debug, Clone)]
pub struct Message {
    text: Box<[u8]>,
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
    pub name: &'a str,
    pub user: Option<&'a str>,
    pub host: Option<&'a str>,
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

    JOIN {
        channels: &'a str,
        keys: Option<&'a str>,
    },

    PRIVMSG {
        targets: &'a str,
        text: &'a str,
    },

    QUIT {
        reason: Option<&'a str>,
    },

    RPLWELCOME {
        client: &'a str,
        text: &'a str,
    },
    RPLYOURHOST {
        client: &'a str,
        text: &'a str,
    },
    RPLCREATED {
        client: &'a str,
        text: &'a str,
    },
    RPLMYINFO {
        client: &'a str,
        servername: &'a str,
        version: &'a str,
        user_modes: &'a str,
        channel_modes: &'a str,
        //TODO: channel_modes_wiht_params
    },

    ERRPASSWDMISMATCH {
        client: &'a str,
    },

    ERRNICKNAMEINUSE {
        client: &'a str,
        nick: &'a str,
    },
}

impl<'a> Command<'a> {
    fn command_to_str(command: &[u8]) -> &str {
        str::from_utf8(command).expect("predefined string must be valid utf8")
    }

    pub fn command(&self) -> &str {
        match self {
            Self::PING { .. } => Self::command_to_str(strings::PING),
            Self::PONG { .. } => Self::command_to_str(strings::PONG),

            Self::CAP { .. } => Self::command_to_str(strings::CAP),
            Self::PASS { .. } => Self::command_to_str(strings::PASS),
            Self::NICK { .. } => Self::command_to_str(strings::NICK),
            Self::USER { .. } => Self::command_to_str(strings::USER),
            Self::QUIT { .. } => Self::command_to_str(strings::QUIT),

            Self::JOIN { .. } => Self::command_to_str(strings::JOIN),
            Self::PRIVMSG { .. } => Self::command_to_str(strings::PRIVMSG),

            Self::RPLWELCOME { .. } => Self::command_to_str(strings::RPL_WELCOME),
            Self::RPLYOURHOST { .. } => Self::command_to_str(strings::RPL_YOURHOST),
            Self::RPLCREATED { .. } => Self::command_to_str(strings::RPL_CREATED),
            Self::RPLMYINFO { .. } => Self::command_to_str(strings::RPL_MYINFO),

            Self::ERRPASSWDMISMATCH { .. } => Self::command_to_str(strings::ERR_PASSWDMISMATCH),
            Self::ERRNICKNAMEINUSE { .. } => Self::command_to_str(strings::ERR_NICKNAMEINUSE),
        }
    }

    pub fn params(&self) -> Params<'a> {
        match self {
            Self::PING { token } => Params::One([*token].into_iter()),
            Self::PONG { server, token } => match server {
                Some(server) => Params::Two([*server, *token].into_iter()),
                None => Params::One([*token].into_iter()),
            },

            Self::CAP {
                subcommand,
                capabilities,
            } => match capabilities {
                Some(capabilities) => Params::Two([*subcommand, *capabilities].into_iter()),
                None => Params::One([*subcommand].into_iter()),
            },
            Self::PASS { password } => Params::One([*password].into_iter()),
            Self::NICK { nickname } => Params::One([*nickname].into_iter()),
            Self::USER {
                user,
                mode,
                unused,
                realname,
            } => Params::Four([*user, *mode, *unused, *realname].into_iter()),

            Self::PRIVMSG { targets, text } => Params::Two([*targets, *text].into_iter()),
            Self::JOIN { channels, keys } => match keys {
                Some(keys) => Params::Two([*channels, *keys].into_iter()),
                None => Params::One([*channels].into_iter()),
            },

            Self::QUIT { reason } => match reason {
                Some(reason) => Params::One([*reason].into_iter()),
                None => Params::Empty,
            },

            Self::RPLWELCOME { client, text } => Params::Two([*client, *text].into_iter()),
            Self::RPLYOURHOST { client, text } => Params::Two([*client, *text].into_iter()),
            Self::RPLCREATED { client, text } => Params::Two([*client, *text].into_iter()),
            Self::RPLMYINFO {
                client,
                servername,
                version,
                user_modes,
                channel_modes,
            } => Params::Five(
                [*client, *servername, *version, *user_modes, *channel_modes].into_iter(),
            ),

            Self::ERRPASSWDMISMATCH { client } => Params::One([*client].into_iter()),
            Self::ERRNICKNAMEINUSE { client, nick } => Params::Two([*client, *nick].into_iter()),
        }
    }
}

pub enum Params<'a> {
    Empty,
    One(std::array::IntoIter<&'a str, 1>),
    Two(std::array::IntoIter<&'a str, 2>),
    Three(std::array::IntoIter<&'a str, 3>),
    Four(std::array::IntoIter<&'a str, 4>),
    Five(std::array::IntoIter<&'a str, 5>),
}

impl<'a> Iterator for Params<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Empty => None,
            Self::One(iter) => iter.next(),
            Self::Two(iter) => iter.next(),
            Self::Three(iter) => iter.next(),
            Self::Four(iter) => iter.next(),
            Self::Five(iter) => iter.next(),
        }
    }
}

#[derive(Debug)]
pub struct MessageBuilder<'a> {
    tags: Vec<Tag<'a>>,
    source: Option<Source<'a>>,
    command: Command<'a>,
}

impl Message {
    pub fn new(input: &[u8]) -> Result<Self, IrcError> {
        let mut parser = Parser::new(input);
        let root_id = match parser.parse_message() {
            Ok(root) => root,
            Err(err) => {
                return Err(IrcError::ParseError(err));
            }
        };
        let nodes = Nodes(parser.get_nodes());
        let root_node = nodes.index(root_id);

        match root_node.kind() {
            NodeKind::Message {
                tags,
                source,
                command,
            } => {
                let length = root_node.length();
                let mut text: Vec<u8> = Vec::with_capacity(length);
                text.extend_from_slice(&input[..length]);
                Ok(Self {
                    text: text.into_boxed_slice(),
                    tags_id: tags.clone(),
                    source_id: source.clone(),
                    command_id: command.clone(),
                    nodes,
                })
            }
            _ => unreachable!("root node must be of kind Message"),
        }
    }

    fn get_node(&self, id: NodeId) -> &Node {
        self.nodes.index(id)
    }

    fn get_value(&self, id: NodeId) -> &str {
        let node = self.nodes.index(id);
        str::from_utf8(&self.text[node.start()..node.start() + node.length()])
            .expect("value is parsed")
    }

    pub fn contents(&self) -> &str {
        str::from_utf8(&self.text).expect("value is parsed")
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
            NodeKind::CommandQuit { reason } => {
                let reason = reason.as_ref().map(|reason| self.get_value(reason.clone()));
                Command::QUIT { reason }
            }
            NodeKind::CommandJoin { channels, keys } => Command::JOIN {
                channels: self.get_value(channels.clone()),
                keys: keys.as_ref().map(|keys| self.get_value(keys.clone())),
            },
            NodeKind::CommandPrivMsg { targets, text } => Command::PRIVMSG {
                targets: self.get_value(targets.clone()),
                text: self.get_value(text.clone()),
            },

            NodeKind::RplWelcome { client, text } => Command::RPLWELCOME {
                client: self.get_value(client.clone()),
                text: self.get_value(text.clone()),
            },
            NodeKind::RplYourhost { client, text } => Command::RPLYOURHOST {
                client: self.get_value(client.clone()),
                text: self.get_value(text.clone()),
            },
            NodeKind::RplCreated { client, text } => Command::RPLCREATED {
                client: self.get_value(client.clone()),
                text: self.get_value(text.clone()),
            },
            NodeKind::RplMyinfo {
                client,
                servername,
                version,
                user_modes,
                channel_modes,
            } => Command::RPLMYINFO {
                client: self.get_value(client.clone()),
                servername: self.get_value(servername.clone()),
                version: self.get_value(version.clone()),
                user_modes: self.get_value(user_modes.clone()),
                channel_modes: self.get_value(channel_modes.clone()),
            },

            NodeKind::ErrPasswdmismatch { client } => Command::ERRPASSWDMISMATCH {
                client: self.get_value(client.clone()),
            },
            NodeKind::ErrNicknameinuse { client, nick } => Command::ERRNICKNAMEINUSE {
                client: self.get_value(client.clone()),
                nick: self.get_value(nick.clone()),
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

    pub fn build(self) -> Option<Message> {
        let mut buffer: Vec<u8> = Vec::with_capacity(1024);

        if !self.tags.is_empty() {
            buffer.push(strings::AT);
        }
        let num_tags = self.tags.len();
        for (idx, tag) in self.tags.iter().enumerate() {
            buffer.extend_from_slice(tag.key.as_bytes());
            if let Some(value) = tag.value.as_ref() {
                buffer.push(strings::EQUALS);
                buffer.extend_from_slice(value.as_bytes());
            }
            if idx + 1 < num_tags {
                buffer.push(strings::SEMICOLON);
            }
        }

        if let Some(source) = self.source {
            buffer.push(strings::COLON);
            buffer.extend_from_slice(source.name.as_bytes());
            if let Some(user) = &source.user {
                buffer.push(strings::BANG);
                buffer.extend_from_slice(user.as_bytes());
            }
            if let Some(host) = source.host {
                buffer.push(strings::AT);
                buffer.extend_from_slice(host.as_bytes());
            }
            buffer.push(strings::SPACE);
        }

        buffer.extend_from_slice(self.command.command().as_bytes());

        for param in self.command.params() {
            buffer.push(strings::SPACE);
            if param.contains(" ") {
                buffer.push(strings::COLON);
            }
            buffer.extend_from_slice(param.as_bytes());
        }

        buffer.push(strings::CR);
        buffer.push(strings::LF);

        Message::new(&buffer).ok()
    }
}

#[cfg(test)]
mod tests {

    use crate::message::{Command, Message, MessageBuilder};

    #[test]
    fn test_message_parse1() {
        let input = "@id=234AB :dan!d@localhost PRIVMSG #chan :Hey what's up!\r\n"
            .as_bytes()
            .to_vec();
        let message = Message::new(&input).unwrap();

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
        let message = Message::new(&input).unwrap();

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
        let message = Message::new(&input).unwrap();

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
        let message = Message::new(&input).unwrap();

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
