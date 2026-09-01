#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub(crate) struct NodeId(pub(crate) u8);

#[derive(Debug, PartialEq, Eq, Default)]
pub(crate) enum NodeKind {
    Message {
        tags: Option<NodeId>,
        source: Option<NodeId>,
        command: NodeId,
    },

    Tags(Box<[NodeId]>),
    Tag {
        key: NodeId,
        value: Option<NodeId>,
    },
    TagKey,
    TagValue,

    Source {
        name: NodeId,
        user: Option<NodeId>,
        host: Option<NodeId>,
    },
    SourceName,
    SourceUser,
    SourceHost,

    CommandPing {
        token: NodeId,
    },
    CommandPong {
        server: Option<NodeId>,
        token: NodeId,
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
    CommandQuit {
        reason: Option<NodeId>,
    },

    CommandJoin {
        channels: NodeId,
        keys: Option<NodeId>,
    },
    CommandPrivMsg {
        targets: NodeId,
        text: NodeId,
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
    pub(crate) fn new(kind: NodeKind, start: u16, length: u16) -> Self {
        Self {
            kind,
            start,
            length,
        }
    }

    pub(crate) fn kind(&self) -> &NodeKind {
        &self.kind
    }

    pub fn start(&self) -> usize {
        self.start.into()
    }

    pub fn length(&self) -> usize {
        self.length.into()
    }
}
