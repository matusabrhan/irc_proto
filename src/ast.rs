use crate::token::Token;

#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub struct NodeId(pub u8);

#[derive(Debug, PartialEq, Eq, Default)]
pub enum NodeKind {
    Message {
        tags: Option<NodeId>,
        source: Option<NodeId>,
        command: NodeId,
    },

    Tags(Box<[NodeId]>),
    Tag {
        key: Token,
        value: Option<Token>,
    },
    Source {
        name: Token,
        user: Option<Token>,
        host: Option<Token>,
    },

    CommandCap {
        subcommand: NodeId,
        capabilities: Option<NodeId>,
    },
    CommandPass {
        password: NodeId,
    },
    CommandNick {
        nickname: NodeId,
    },
    CommandUser {
        user: NodeId,
        mode: NodeId,
        unused: NodeId,
        realname: NodeId,
    },
    CommandPrivMsg {
        targets: NodeId,
        text: NodeId,
    },
    CommandPing {
        token: NodeId,
    },
    CommandPong {
        server: Option<NodeId>,
        token: NodeId,
    },

    Parameter,

    #[default]
    Invalid,
}

#[derive(Debug, PartialEq, Eq, Default)]
pub struct Node {
    kind: NodeKind,
    start: u16,
    length: u16,
}

impl Node {
    pub fn new(kind: NodeKind, start: u16, length: u16) -> Self {
        Self {
            kind,
            start,
            length,
        }
    }

    pub fn kind(&self) -> &NodeKind {
        &self.kind
    }

    pub fn start(&self) -> usize {
        self.start.into()
    }

    pub fn length(&self) -> usize {
        self.length.into()
    }
}
