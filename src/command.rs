use crate::types::Command;

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
                token: required!(),
                server: optional!(),
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
                channels: required!(),
                keys: optional!(),
            },
            "PRIVMSG" => PRIVMSG {
                targets: required!(),
                text: required!(),
            },
            "WHO" => WHO { mask: required!() },

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
            JOIN { channels, keys } => std::iter::once(channels.to_string())
                .chain(keys.clone())
                .collect(),
            PRIVMSG { targets, text } => vec![targets.to_string(), text.to_string()],
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

            _ => "".to_string(),
        }
    }

    pub fn numeric(&self) -> u16 {
        use Command::*;
        match self {
            RPL_ENDOFWHO { .. } => 315,
            RPL_WHOREPLY { .. } => 352,
            RPL_NAMREPLY { .. } => 353,
            RPL_ENDOFNAMES { .. } => 366,
            RPL_MOTD { .. } => 372,
            RPL_MOTDSTART { .. } => 375,
            RPL_ENDOFMOTD { .. } => 376,

            ERR_NOTEXTTOSEND { .. } => 412,
            ERR_NONICKNAMEGIVEN { .. } => 431,
            ERR_ERRONEUSNICKNAME { .. } => 432,
            ERR_NICKNAMEINUSE { .. } => 433,
            ERR_NICKCOLLISION { .. } => 436,
            ERR_NEEDMOREPARAMS { .. } => 461,
            ERR_ALREADYREGISTERED { .. } => 462,
            ERR_PASSWDMISMATCH { .. } => 464,

            _ => 0,
        }
    }
}
