use crate::error::IrcParseError;
use bytes::Buf;
use std::io::{BufRead, Cursor, Read};

pub const MESSAGE_MAX_LENGTH: usize = 512;
pub const TAGS_MAX_LENGTH: usize = 4096;
pub const COMBINED_TAGS_MAX_LENGTH: usize = (TAGS_MAX_LENGTH * 2) - 1;
pub const MESSAGE_END: [u8; 2] = [b'\r', b'\n'];
pub const AT: [u8; 1] = [b'@'];
pub const COLON: [u8; 1] = [b':'];
pub const SEMICOLON: [u8; 1] = [b';'];
pub const EQUAL: [u8; 1] = [b'='];
pub const PLUS: [u8; 1] = [b'+'];
pub const SLASH: [u8; 1] = [b'/'];
pub const BANG: [u8; 1] = [b'!'];
pub const SPACE: [u8; 1] = [b' '];

pub trait IrcSerializable {
    fn from_u8(value: &[u8]) -> Result<Self, IrcParseError>
    where
        Self: Sized;
    fn to_vec_u8(&self) -> Vec<u8>;
}

pub trait IrcCursor {
    fn split_at(&mut self, delimiter: &[u8]) -> Option<(&[u8], &[u8])>;
    fn split_frame(&mut self) -> Option<&[u8]>;
    fn get_after(&self) -> &[u8];
}

impl IrcCursor for Cursor<&[u8]> {
    fn split_at(&mut self, delimiter: &[u8]) -> Option<(&[u8], &[u8])> {
        let pos = self.position() as usize;
        let delimiter_len = delimiter.len();
        while self.remaining() >= delimiter_len {
            if self.get_after().starts_with(delimiter) {
                let slice = &self.get_ref()[pos..];
                let (left, mut right) = slice.split_at(self.position() as usize - pos);
                right = &right[delimiter_len..];
                self.advance(delimiter_len);
                return Some((left, right));
            }
            self.advance(1);
        }
        self.set_position(pos as u64);
        return None;
    }

    fn split_frame(&mut self) -> Option<&[u8]> {
        let pos = self.position() as usize;
        while self.remaining() > 0 {
            let after = self.get_after();
            if after.starts_with(&[b'\r']) || after.starts_with(&[b'\n']) {
                let slice = &self.get_ref()[pos..];
                let (left, _) = slice.split_at(self.position() as usize - pos);
                while self.get_after().starts_with(&[b'\r'])
                    || self.get_after().starts_with(&[b'\n'])
                {
                    self.advance(1);
                }
                return Some(left);
            }
            self.advance(1);
        }
        self.set_position(pos as u64);
        return None;
    }

    fn get_after(&self) -> &[u8] {
        &self.get_ref()[self.position() as usize..]
    }
}

#[derive(Debug, Clone, Default)]
pub struct Message {
    tags: Option<Tags>,
    source: Option<Source>,
    command: Command,
}

#[derive(Debug, Clone, Default)]
pub struct Tags {
    tags: Box<[Tag]>,
}

#[derive(Debug, Clone, Default)]
pub struct Tag {
    key: TagKey,
    value: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct TagKey {
    client_prefix: Option<String>,
    vendor: Option<String>,
    value: String,
}

#[derive(Debug, Clone, Default)]
pub struct Source {
    pub name: String,
    user: Option<String>,
    host: Option<String>,
}

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Default)]
pub enum Command {
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
    JOIN {
        channels: Vec<String>,
        keys: Option<Vec<String>>,
    },
    PRIVMSG {
        targets: Vec<String>,
        text: String,
    },
    WHO {
        mask: String,
    },
    RPL_WELCOME {
        text: String,
    },
    RPL_YOURHOST {
        text: String,
    },
    RPL_CREATED {
        text: String,
    },
    RPL_MYINFO {
        text: String,
    },
    RPL_ENDOFWHO {
        client: String,
        mask: String,
    },
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
    RPL_NAMREPLY {
        client: String,
        symbol: String,
        channel: String,
        members: Vec<String>,
    },
    RPL_ENDOFNAMES {
        client: String,
        channel: String,
    },
    RPL_MOTD {
        client: String,
        line: String,
    },
    RPL_MOTDSTART {
        client: String,
        line: String,
    },
    RPL_ENDOFMOTD {
        client: String,
    },
    ERR_NOTEXTTOSEND {
        client: String,
    },
    ERR_NONICKNAMEGIVEN {
        client: String,
    },
    ERR_ERRONEUSNICKNAME {
        client: String,
        nick: String,
    },
    ERR_NICKNAMEINUSE {
        client: String,
        nick: String,
    },
    ERR_NICKCOLLISION {
        client: String,
        nick: String,
        user: String,
        host: String,
    },
    ERR_NEEDMOREPARAMS {
        client: String,
        command: String,
    },
    ERR_ALREADYREGISTERED {
        client: String,
    },
    ERR_PASSWDMISMATCH {
        client: String,
    },
    #[default]
    UNKNOWN,
}

impl Command {
    pub fn new(command: &str, params: Vec<String>) -> Self {
        use Command::*;

        let mut params_iter = params.into_iter();

        macro_rules! required {
            () => {
                match params_iter.next() {
                    Some(param) => param,
                    None => return UNKNOWN,
                }
            };
        }

        macro_rules! optional {
            () => {
                params_iter.next()
            };
        }

        match command {
            "CAP" => CAP {
                subcommand: required!(),
                capabilities: optional!(),
            },
            "PASS" => PASS {
                password: required!(),
            },
            "NICK" => NICK {
                nickname: required!(),
            },
            "USER" => USER {
                user: required!(),
                mode: required!(),
                unused: required!(),
                realname: required!(),
            },
            "PING" => PING { token: required!() },
            "PONG" => PONG {
                server: optional!(),
                token: required!(),
            },
            "OPER" => OPER {
                name: required!(),
                password: required!(),
            },
            "QUIT" => QUIT {
                reason: optional!(),
            },
            "ERROR" => ERROR {
                reason: required!(),
            },
            "JOIN" => JOIN {
                channels: required!().split(",").map(|c| c.to_string()).collect(),
                keys: optional!().and_then(|k| Some(k.split(",").map(|k| k.to_string()).collect())),
            },
            "PRIVMSG" => PRIVMSG {
                targets: required!().split(",").map(|t| t.to_string()).collect(),
                text: required!(),
            },
            "WHO" => WHO { mask: required!() },

            "001" => RPL_WELCOME { text: required!() },
            "002" => RPL_YOURHOST { text: required!() },
            "003" => RPL_CREATED { text: required!() },
            "004" => RPL_MYINFO { text: required!() },
            "315" => RPL_ENDOFWHO {
                client: required!(),
                mask: required!(),
            },
            "352" => RPL_WHOREPLY {
                client: required!(),
                channel: required!(),
                username: required!(),
                host: required!(),
                server: required!(),
                nick: required!(),
                flags: required!(),
                hopcount: required!(),
                realname: required!(),
            },
            "353" => RPL_NAMREPLY {
                client: required!(),
                symbol: required!(),
                channel: required!(),
                members: params_iter.collect(),
            },
            "366" => RPL_ENDOFNAMES {
                client: required!(),
                channel: required!(),
            },
            "372" => RPL_MOTD {
                client: required!(),
                line: required!(),
            },
            "375" => RPL_MOTDSTART {
                client: required!(),
                line: required!(),
            },
            "376" => RPL_ENDOFMOTD {
                client: required!(),
            },

            "412" => ERR_NOTEXTTOSEND {
                client: required!(),
            },
            "431" => ERR_NONICKNAMEGIVEN {
                client: required!(),
            },
            "432" => ERR_ERRONEUSNICKNAME {
                client: required!(),
                nick: required!(),
            },
            "433" => ERR_NICKNAMEINUSE {
                client: required!(),
                nick: required!(),
            },
            "436" => ERR_NICKCOLLISION {
                client: required!(),
                nick: required!(),
                user: required!(),
                host: required!(),
            },
            "461" => ERR_NEEDMOREPARAMS {
                client: required!(),
                command: required!(),
            },
            "462" => ERR_ALREADYREGISTERED {
                client: required!(),
            },
            "464" => ERR_PASSWDMISMATCH {
                client: required!(),
            },

            _ => UNKNOWN,
        }
    }

    pub fn params(&self) -> Vec<String> {
        use Command::*;

        match self {
            // TODO: fix CAP subcommands
            CAP {
                subcommand,
                capabilities,
            } => {
                if let Some(cap) = capabilities {
                    vec![subcommand.to_string(), cap.to_string()]
                } else {
                    vec![subcommand.to_string()]
                }
            }
            PING { token } => vec![token.to_string()],
            PONG { server, token } => {
                if let Some(server) = server {
                    vec![server.to_string(), token.to_string()]
                } else {
                    vec![token.to_string()]
                }
            }
            JOIN { channels, keys } => {
                if let Some(keys) = keys {
                    let mut res = Vec::new();
                    res.extend(channels.iter().map(|c| c.clone()));
                    res.extend(keys.iter().map(|k| k.clone()));
                    res
                } else {
                    channels.iter().map(|c| c.clone()).collect()
                }
            }
            PRIVMSG { targets, text } => vec![targets.join(","), text.to_string()],
            PASS { password } => vec![password.to_string()],
            NICK { nickname } => vec![nickname.to_string()],
            USER {
                user,
                mode,
                unused,
                realname,
            } => vec![
                user.to_string(),
                mode.to_string(),
                unused.to_string(),
                realname.to_string(),
            ],
            WHO { mask } => vec![mask.to_string()],

            RPL_WELCOME { text } => vec![text.to_string()],
            RPL_YOURHOST { text } => vec![text.to_string()],
            RPL_CREATED { text } => vec![text.to_string()],
            RPL_MYINFO { text } => vec![text.to_string()],
            RPL_ENDOFWHO { client, mask } => vec![client.to_string(), mask.to_string()],
            // RPL_WHOREPLY{client, channel, username, host, server, nick, flags, hopcount, realname} => vec![client, channel, username, host, server, nick, flags, hopcount, realname],
            // RPL_NAMREPLY{client, symbol, channel, members} => vec![client, symbol, channel, members],
            RPL_ENDOFNAMES { client, channel } => vec![client.to_string(), channel.to_string()],
            RPL_MOTD { client, line } => vec![client.to_string(), line.to_string()],
            RPL_MOTDSTART { client, line } => vec![client.to_string(), line.to_string()],
            RPL_ENDOFMOTD { client } => vec![client.to_string()],

            ERR_NOTEXTTOSEND { client } => vec![client.to_string()],
            ERR_NONICKNAMEGIVEN { client } => vec![client.to_string()],
            ERR_ERRONEUSNICKNAME { client, nick } => vec![client.to_string(), nick.to_string()],
            ERR_NICKNAMEINUSE { client, nick } => vec![client.to_string(), nick.to_string()],
            ERR_NICKCOLLISION {
                client,
                nick,
                user,
                host,
            } => vec![
                client.to_string(),
                nick.to_string(),
                user.to_string(),
                host.to_string(),
            ],
            ERR_NEEDMOREPARAMS { client, command } => vec![client.to_string(), command.to_string()],
            ERR_ALREADYREGISTERED { client } => vec![client.to_string()],
            ERR_PASSWDMISMATCH { client } => vec![client.to_string()],

            _ => vec![],
        }
    }

    pub fn command(&self) -> String {
        use Command::*;

        match self {
            CAP { .. } => "CAP".to_string(),
            PING { .. } => "PING".to_string(),
            PONG { .. } => "PONG".to_string(),
            JOIN { .. } => "JOIN".to_string(),
            PRIVMSG { .. } => "PRIVMSG".to_string(),
            PASS { .. } => "PASS".to_string(),
            NICK { .. } => "NICK".to_string(),
            USER { .. } => "USER".to_string(),
            WHO { .. } => "WHO".to_string(),
            QUIT { .. } => "QUIT".to_string(),

            RPL_WELCOME { .. } => "001".to_string(),
            RPL_YOURHOST { .. } => "002".to_string(),
            RPL_CREATED { .. } => "003".to_string(),
            RPL_MYINFO { .. } => "004".to_string(),
            RPL_ENDOFWHO { .. } => "315".to_string(),
            RPL_WHOREPLY { .. } => "352".to_string(),
            RPL_NAMREPLY { .. } => "353".to_string(),
            RPL_ENDOFNAMES { .. } => "366".to_string(),
            RPL_MOTD { .. } => "372".to_string(),
            RPL_MOTDSTART { .. } => "375".to_string(),
            RPL_ENDOFMOTD { .. } => "376".to_string(),

            ERR_NOTEXTTOSEND { .. } => "412".to_string(),
            ERR_NONICKNAMEGIVEN { .. } => "431".to_string(),
            ERR_ERRONEUSNICKNAME { .. } => "432".to_string(),
            ERR_NICKNAMEINUSE { .. } => "433".to_string(),
            ERR_NICKCOLLISION { .. } => "436".to_string(),
            ERR_NEEDMOREPARAMS { .. } => "461".to_string(),
            ERR_ALREADYREGISTERED { .. } => "462".to_string(),
            ERR_PASSWDMISMATCH { .. } => "464".to_string(),

            _ => "".to_string(),
        }
    }
}

impl IrcSerializable for Command {
    fn from_u8(value: &[u8]) -> Result<Self, IrcParseError>
    where
        Self: Sized,
    {
        let mut cursor = Cursor::new(value);
        match cursor.split_at(&SPACE) {
            Some((left, _)) => {
                let command = match String::from_utf8(left.to_vec()) {
                    Ok(command) => command,
                    Err(_) => return Err(IrcParseError::new()),
                };
                let params: Vec<String> = match cursor.split_at(&COLON) {
                    Some((left, right)) => {
                        let mut params_vec: Vec<String> = Vec::new();
                        let left = left.trim_ascii();
                        if !left.is_empty() {
                            match String::from_utf8(left.to_vec()) {
                                Ok(params) => {
                                    params_vec.append(
                                        &mut params
                                            .split(' ')
                                            .map(|s| s.to_string())
                                            .collect::<Vec<String>>(),
                                    );
                                }
                                Err(_) => return Err(IrcParseError::new()),
                            };
                        }
                        let right = right.trim_ascii();
                        if !right.is_empty() {
                            match String::from_utf8(right.to_vec()) {
                                Ok(last_param) => params_vec.push(last_param),
                                Err(_) => return Err(IrcParseError::new()),
                            }
                        }
                        params_vec
                    }
                    None => match String::from_utf8(cursor.get_after().to_vec()) {
                        Ok(params) => params.split(' ').map(|s| s.to_string()).collect(),
                        Err(_) => return Err(IrcParseError::new()),
                    },
                };
                Ok(Self::new(&command, params))
            }
            None => match String::from_utf8(cursor.get_after().to_vec()) {
                Ok(command) => Ok(Self::new(&command, Vec::new())),
                Err(_) => return Err(IrcParseError::new()),
            },
        }
    }

    fn to_vec_u8(&self) -> Vec<u8> {
        let mut bytes: Vec<u8> = Vec::new();
        bytes.append(&mut self.command().as_bytes().to_vec());
        for param in self.params() {
            bytes.append(&mut SPACE.to_vec());
            match param.contains(SPACE[0] as char) {
                true => {
                    bytes.append(&mut COLON.to_vec());
                    bytes.append(&mut param.as_bytes().to_vec());
                }
                false => {
                    bytes.append(&mut param.as_bytes().to_vec());
                }
            }
        }
        bytes
    }
}

impl Message {
    pub fn new(tags: Option<Tags>, source: Option<Source>, command: Command) -> Self {
        Self {
            tags,
            source,
            command,
        }
    }

    pub fn tags(&self) -> Option<&Tags> {
        self.tags.as_ref()
    }

    pub fn source(&self) -> Option<&Source> {
        self.source.as_ref()
    }

    pub fn command(&self) -> &Command {
        &self.command
    }

    pub fn with_tags(mut self, tags: Tags) -> Self {
        self.tags = Some(tags);
        self
    }

    pub fn with_source(mut self, source: Source) -> Self {
        self.source = Some(source);
        self
    }

    pub fn with_command(mut self, command: Command) -> Self {
        self.command = command;
        self
    }
}

impl IrcSerializable for Message {
    fn from_u8(value: &[u8]) -> Result<Self, IrcParseError> {
        let mut cursor = Cursor::new(value);
        let tags = match cursor.get_after().starts_with(&AT) {
            true => {
                cursor.advance(AT.len());
                match cursor.split_at(&SPACE) {
                    Some((left, _)) => Some(Tags::from_u8(left)?),
                    None => return Err(IrcParseError::new()),
                }
            }
            false => None,
        };
        let source = match cursor.get_after().starts_with(&COLON) {
            true => {
                cursor.advance(COLON.len());
                match cursor.split_at(&SPACE) {
                    Some((left, _)) => Some(Source::from_u8(left)?),
                    None => return Err(IrcParseError::new()),
                }
            }
            false => None,
        };
        Ok(Self::new(
            tags,
            source,
            Command::from_u8(cursor.get_after())?,
        ))
    }

    fn to_vec_u8(&self) -> Vec<u8> {
        let mut bytes: Vec<u8> = Vec::new();

        if let Some(tags) = &self.tags {
            bytes.append(&mut AT.to_vec());
            bytes.append(&mut tags.to_vec_u8());
            bytes.append(&mut SPACE.to_vec());
        }

        if let Some(source) = &self.source {
            bytes.append(&mut COLON.to_vec());
            bytes.append(&mut source.to_vec_u8());
            bytes.append(&mut SPACE.to_vec());
        }

        bytes.append(&mut self.command.to_vec_u8());
        bytes.append(&mut MESSAGE_END.to_vec());
        return bytes;
    }
}

impl Tags {
    fn new(tags: Vec<Tag>) -> Self {
        Self {
            tags: tags.into_boxed_slice(),
        }
    }
}

impl IrcSerializable for Tags {
    fn from_u8(value: &[u8]) -> Result<Self, IrcParseError> {
        let mut tags_vec: Vec<Tag> = Vec::new();
        let cursor = Cursor::new(value);
        for elem in cursor.split(SEMICOLON[0]) {
            match elem {
                Ok(tag) => {
                    tags_vec.push(Tag::from_u8(tag.as_slice())?);
                }
                Err(_) => return Err(IrcParseError::new()),
            }
        }
        Ok(Self::new(tags_vec))
    }

    fn to_vec_u8(&self) -> Vec<u8> {
        let mut buffer: Vec<u8> = Vec::new();
        for tag in &self.tags {
            buffer.append(&mut tag.to_vec_u8());
        }
        buffer
    }
}

impl Tag {
    pub fn new(key: TagKey, value: Option<String>) -> Self {
        Self { key, value }
    }
}

impl IrcSerializable for Tag {
    fn from_u8(value: &[u8]) -> Result<Self, IrcParseError> {
        let mut cursor = Cursor::new(value);
        match cursor.split_at(&EQUAL) {
            Some((left, right)) => match String::from_utf8(right.to_vec()) {
                Ok(tag_value) => Ok(Self::new(TagKey::from_u8(left)?, Some(tag_value))),
                Err(_) => return Err(IrcParseError::new()),
            },
            None => Ok(Self {
                key: TagKey::from_u8(value)?,
                value: None,
            }),
        }
    }

    fn to_vec_u8(&self) -> Vec<u8> {
        let mut buffer: Vec<u8> = Vec::new();
        buffer.append(&mut self.key.to_vec_u8());
        if let Some(value) = &self.value {
            buffer.push(EQUAL[0]);
            buffer.append(&mut Vec::from(value.as_bytes()));
        }
        buffer
    }
}

impl TagKey {
    pub fn new(client_prefix: Option<String>, vendor: Option<String>, value: String) -> Self {
        Self {
            client_prefix,
            vendor,
            value,
        }
    }
}

impl IrcSerializable for TagKey {
    fn from_u8(value: &[u8]) -> Result<Self, IrcParseError> {
        // FIX: do not user Self::default() here
        let mut cursor = Cursor::new(value);
        let mut tag_key = Self::default();
        if cursor.get_ref().starts_with(&PLUS) {
            cursor.advance(PLUS.len());
            tag_key.client_prefix = Some(String::from("+"))
        }
        match cursor.split_at(&EQUAL) {
            Some(_) => todo!(),
            None => {
                if let Err(_) = cursor.read_to_string(&mut tag_key.value) {
                    return Err(IrcParseError::new());
                }
            }
        }
        Ok(tag_key)
    }

    fn to_vec_u8(&self) -> Vec<u8> {
        let mut buffer: Vec<u8> = Vec::new();
        if let Some(client_prefix) = &self.client_prefix {
            buffer.append(&mut Vec::from(client_prefix.as_bytes()))
        }
        if let Some(vendor) = &self.vendor {
            buffer.append(&mut Vec::from(vendor.as_bytes()));
        }
        buffer.append(&mut Vec::from(self.value.as_bytes()));
        buffer
    }
}

impl Source {
    pub fn new(name: String, user: Option<String>, host: Option<String>) -> Self {
        Self { name, user, host }
    }

    pub fn with_name(mut self, name: String) -> Self {
        self.name = name;
        self
    }
}

impl IrcSerializable for Source {
    fn from_u8(value: &[u8]) -> Result<Self, IrcParseError> {
        // FIX: do not user Self::default() here
        let mut cursor = Cursor::new(value);
        let mut source = Self::default();
        match cursor.split_at(&BANG) {
            Some((left, _)) => {
                match String::from_utf8(left.to_vec()) {
                    Ok(name) => source.name = name,
                    Err(_) => return Err(IrcParseError::new()),
                };
                match cursor.split_at(&AT) {
                    Some((left, right)) => {
                        match String::from_utf8(left.to_vec()) {
                            Ok(user) => source.user = Some(user),
                            Err(_) => return Err(IrcParseError::new()),
                        };
                        match String::from_utf8(right.to_vec()) {
                            Ok(host) => source.host = Some(host),
                            Err(_) => return Err(IrcParseError::new()),
                        };
                    }
                    None => {
                        if cursor.read_to_string(&mut source.name).is_err() {
                            return Err(IrcParseError::new());
                        }
                    }
                }
            }
            None => {
                if cursor.read_to_string(&mut source.name).is_err() {
                    return Err(IrcParseError::new());
                }
            }
        }
        Ok(source)
    }

    fn to_vec_u8(&self) -> Vec<u8> {
        let mut buffer: Vec<u8> = Vec::new();
        buffer.append(&mut Vec::from(self.name.as_bytes()));
        if let Some(user) = &self.user {
            buffer.push(BANG[0]);
            buffer.append(&mut Vec::from(user.as_bytes()));
        }
        if let Some(host) = &self.host {
            buffer.push(AT[0]);
            buffer.append(&mut Vec::from(host.as_bytes()));
        }
        buffer
    }
}

#[cfg(test)]
mod tests {

    use crate::{
        enable_logging,
        message::{IrcSerializable, Message},
    };

    #[test]
    fn test_message_parse1() {
        let data = "@id=234AB :dan!d@localhost PRIVMSG #chan :Hey what's up!".as_bytes();
        let message = Message::from_u8(data).unwrap();
        assert_eq!(
            String::from("@id=234AB :dan!d@localhost PRIVMSG #chan :Hey what's up!\r\n"),
            String::from_utf8(message.to_vec_u8()).unwrap()
        );
    }

    #[test]
    fn test_message_parse2() {
        let data = ":irc.example.com CAP REQ :multi-prefix extended-join sasl".as_bytes();
        let message = Message::from_u8(data).unwrap();
        assert_eq!(
            String::from(":irc.example.com CAP REQ :multi-prefix extended-join sasl\r\n"),
            String::from_utf8(message.to_vec_u8()).unwrap()
        );
    }

    #[test]
    fn test_message_parse3() {
        let data = ":irc.example.com PONG server1 token".as_bytes();
        let message = Message::from_u8(data).unwrap();
        assert_eq!(
            String::from(":irc.example.com PONG server1 token\r\n"),
            String::from_utf8(message.to_vec_u8()).unwrap()
        );
    }

    #[test]
    fn test_message_parse4() {
        let data = ":irc.example.com USER username1 0 * realname1".as_bytes();
        let message = Message::from_u8(data).unwrap();
        assert_eq!(
            String::from(":irc.example.com USER username1 0 * realname1\r\n"),
            String::from_utf8(message.to_vec_u8()).unwrap()
        );
    }
}
