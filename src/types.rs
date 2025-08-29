#[derive(Debug, Clone)]
pub struct Message {
    pub tags: Option<Vec<Tag>>,
    pub source: Option<Source>,
    pub command: Command,
}

#[derive(Debug, Clone)]
pub struct Tag {
    pub key: TagKey,
    pub value: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TagKey {
    pub client_prefix: Option<String>,
    pub vendor: Option<String>,
    pub value: String,
}

#[derive(Debug, Clone)]
pub struct Source {
    pub name: String,
    pub user: Option<String>,
    pub host: Option<String>,
}

#[derive(Debug)]
pub struct ServerSource {
    pub servername: String,
}

#[derive(Debug)]
pub struct ClientSource {
    pub nickname: String,
    pub user: Option<String>,
    pub host: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Nickname {
    pub value: String,
    pub optional_value: Option<String>,
}

#[allow(non_camel_case_types)]
#[derive(Debug, Clone)]
pub enum Command {
    // Connection Messages
    CAP {
        subcommand: String,
        capabilities: Option<String>,
    },
    PASS {
        password: String,
    },
    NICK {
        nickname: String,
    },
    USER {
        user: String,
        mode: String,
        unused: String,
        realname: String,
    },
    PING {
        token: String,
    },
    PONG {
        server: Option<String>,
        token: String,
    },
    OPER {
        name: String,
        password: String,
    },
    QUIT {
        reason: Option<String>,
    },
    ERROR {
        reason: String,
    },

    // Channel Operations
    JOIN {
        channels: String,
        keys: Option<String>,
    },

    // Sending Messages
    PRIVMSG {
        targets: String,
        text: String,
    },

    // User-Based Queries
    WHO {
        mask: String,
    },

    /// Reply 315
    RPL_ENDOFWHO {
        client: String,
        mask: String,
    },
    /// Reply 352
    RPL_WHOREPLY {
        client: String,
        channel: String,
        username: String,
        host: String,
        server: String,
        nick: String,
        flags: String,
        hopcount: String,
        realname: String,
    },
    /// Reply 353
    RPL_NAMREPLY {
        client: String,
        symbol: String,
        channel: String,
        members: Vec<String>,
    },
    /// Reply 366
    RPL_ENDOFNAMES {
        client: String,
        channel: String,
    },
    /// Reply 372
    RPL_MOTD {
        client: String,
        line: String,
    },
    /// Reply 375
    RPL_MOTDSTART {
        client: String,
        line: String,
    },
    /// Reply 376
    RPL_ENDOFMOTD {
        client: String,
    },

    /// Error 412
    ERR_NOTEXTTOSEND {
        client: String,
    },
    /// Error 431
    ERR_NONICKNAMEGIVEN {
        client: String,
    },
    /// Error 432
    ERR_ERRONEUSNICKNAME {
        client: String,
        nick: String,
    },
    /// Error 433
    ERR_NICKNAMEINUSE {
        client: String,
        nick: String,
    },
    /// Error 436
    ERR_NICKCOLLISION {
        client: String,
        nick: String,
        user: String,
        host: String,
    },
    /// Error 461
    ERR_NEEDMOREPARAMS {
        client: String,
        command: String,
    },
    /// Error 462
    ERR_ALREADYREGISTERED {
        client: String,
    },
    /// Error 464
    ERR_PASSWDMISMATCH {
        client: String,
    }, // 464

    // UNKNOWN
    UNKNOWN,
}
