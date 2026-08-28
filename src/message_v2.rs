use std::ops::Index;

use crate::{
    ast::{Node, NodeId, NodeKind},
    message::{Command, AT, BANG, COLON},
    parser::Parser,
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
    source_node: Option<NodeId>,
    command_node: NodeId,
    nodes: Nodes<Node>,
}

struct Source {
    name: String,
    user: Option<String>,
    host: Option<String>,
}

#[derive(Clone)]
pub struct Tag {
    key: String,
    value: Option<String>,
}

struct MessageBuilder {
    tags: Vec<Tag>,
    source: Option<Source>,
    command: Command,
}

impl MessageV2 {
    pub fn new(input: Vec<u8>) -> Option<Self> {
        let text: Box<str> = String::from_utf8(input).ok()?.into_boxed_str();
        let (root, nodes) = {
            let mut parser = Parser::new(text.as_ref());
            (parser.parse_message().ok()?, Nodes(parser.get_nodes()))
        };

        let mut source_node: Option<NodeId> = None;
        let mut command_node: Option<NodeId> = None;
        match nodes.index(root).kind() {
            NodeKind::Message {
                source, command, ..
            } => {
                source_node = source.clone();
                command_node = Some(command.clone());
            }
            _ => return None,
        };

        Some(Self {
            text,
            source_node,
            command_node: command_node?,
            nodes,
        })
    }

    pub fn get_node(&self, id: NodeId) -> &Node {
        self.nodes.index(id)
    }

    pub fn get_value(&self, id: NodeId) -> Option<&str> {
        let node = self.nodes.index(id);
        self.text.get(node.start()..node.start() + node.length())
    }

    pub fn get_command(&self) -> &Node {
        &self.nodes.index(self.command_node.clone())
    }

    pub fn get_source(&self) -> Option<&Node> {
        if let Some(source_node) = &self.source_node {
            return Some(self.nodes.index(source_node.clone()));
        }
        None
    }
}

impl MessageBuilder {
    pub fn with_command(&mut self, command: Command) -> Self {
        Self {
            tags: Vec::new(),
            source: None,
            command,
        }
    }

    pub fn with_source(self, name: String, user: Option<String>, host: Option<String>) -> Self {
        Self {
            tags: self.tags,
            source: Some(Source { name, user, host }),
            command: self.command,
        }
    }

    pub fn with_tag(self, key: String, value: Option<String>) -> Self {
        let mut tags: Vec<Tag> = self.tags.clone();
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
            buffer.extend_from_slice(&AT);
        }
        for tag in self.tags {
            buffer.extend_from_slice(tag.key.as_bytes());
            if let Some(value) = tag.value.as_ref() {
                buffer.extend_from_slice(value.as_bytes());
            }
        }

        if let Some(source) = self.source {
            buffer.extend_from_slice(source.name.as_bytes());
            if let Some(user) = &source.user {
                buffer.extend_from_slice(&BANG);
                buffer.extend_from_slice(user.as_bytes());
            }
            if let Some(host) = source.host {
                buffer.extend_from_slice(&AT);
                buffer.extend_from_slice(host.as_bytes());
            }
        }

        buffer.extend_from_slice(self.command.command().as_bytes());

        for param in self.command.params() {
            if param.contains(" ") {
                buffer.extend_from_slice(&COLON);
            }
            buffer.extend_from_slice(param.as_bytes());
        }

        MessageV2::new(buffer)
    }
}

#[cfg(test)]
mod tests {

    use crate::{ast::NodeKind, enable_logging, message_v2::MessageV2};

    #[test]
    fn test_message_parse1() {
        let input = "@id=234AB :dan!d@localhost PRIVMSG #chan :Hey what's up!\r\n"
            .as_bytes()
            .to_vec();
        let message = MessageV2::new(input).unwrap();
        let command_node = message.get_command();

        match command_node.kind() {
            NodeKind::CommandPrivMsg { targets, text } => {
                assert_eq!(message.get_value(targets.clone()), Some("#chan"));
                assert_eq!(message.get_value(text.clone()), Some("Hey what's up!"));
            }
            _ => assert!(false),
        }
    }

    #[test]
    fn test_message_parse2() {
        let input = ":irc.example.com CAP REQ :multi-prefix extended-join sasl\r\n"
            .as_bytes()
            .to_vec();
        let message = MessageV2::new(input).unwrap();
        let command_node = message.get_command();
        match command_node.kind() {
            NodeKind::CommandCap {
                subcommand,
                capabilities,
            } => {
                assert_eq!(message.get_value(subcommand.clone()), Some("REQ"));
                assert_eq!(
                    message.get_value(capabilities.clone().unwrap()),
                    Some("multi-prefix extended-join sasl")
                );
            }
            _ => assert!(false),
        }
    }

    #[test]
    fn test_message_parse3() {
        let input = ":irc.example.com PONG server1 token\r\n"
            .as_bytes()
            .to_vec();
        let message = MessageV2::new(input).unwrap();
        let command_node = message.get_command();
        match command_node.kind() {
            NodeKind::CommandPong { server, token } => {
                assert_eq!(message.get_value(server.clone().unwrap()), Some("server1"));
                assert_eq!(message.get_value(token.clone()), Some("token"));
            }
            _ => assert!(false),
        }
    }

    #[test]
    fn test_message_parse4() {
        let input = ":irc.example.com USER username1 0 * realname1"
            .as_bytes()
            .to_vec();
        let message = MessageV2::new(input).unwrap();
        let command_node = message.get_command();
        match command_node.kind() {
            NodeKind::CommandUser {
                user,
                mode,
                unused,
                realname,
            } => {
                assert_eq!(message.get_value(user.clone()), Some("username1"));
                assert_eq!(message.get_value(mode.clone()), Some("0"));
                assert_eq!(message.get_value(unused.clone()), Some("*"));
                assert_eq!(message.get_value(realname.clone()), Some("realname1"));
            }
            _ => assert!(false),
        }
    }
}
