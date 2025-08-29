use core::str;

use crate::types::{Command, Message, Source, Tag, TagKey};

impl Message {
    pub fn new(tags: Option<Vec<Tag>>, source: Option<Source>, command: Command) -> Self {
        return Message {
            tags,
            source,
            command,
        };
    }

    pub fn to_bytes(self) -> String {
        let mut output: String = String::new();

        if let Some(tags) = &self.tags {
            output.push('@');
            for tag in tags {
                if let Some(client_prefix) = &tag.key.client_prefix {
                    output.push_str(client_prefix);
                }
                if let Some(vendor) = &tag.key.vendor {
                    output.push_str(vendor);
                    output.push('/');
                }
                output.push_str(&tag.key.value);
                if let Some(value) = &tag.value {
                    output.push('=');
                    output.push_str(value);
                }
            }
            output.push(' ');
        }

        if let Some(source) = &self.source {
            output.push(':');
            output.push_str(&source.name);
            if let Some(user) = &source.user {
                output.push('!');
                output.push_str(&user);
            }
            if let Some(host) = &source.host {
                output.push('@');
                output.push_str(&host);
            }
            output.push(' ');
        }

        output.push_str(&self.command.command());
        for argument in self.command.params() {
            output.push(' ');
            if argument.contains(' ') {
                output.push(':');
            }
            output.push_str(&argument);
        }
        output.push_str("\r\n");

        return output;
    }

    pub fn from_bytes<'a>(src: &'a [u8]) -> Option<Message> {
        // message ::= ['@' <tags> SPACE] [':' <source> SPACE] <command> <parameters>

        let mut input = match str::from_utf8(src) {
            Ok(s) => s,
            Err(_) => return None,
        };

        let mut message: Message = Message {
            tags: None,
            source: None,
            command: Command::UNKNOWN,
        };

        if input.starts_with('@') {
            if let Some(space_pos) = input.find(' ') {
                match Message::parse_tags(&input[1..space_pos]) {
                    Ok(tags) => message.tags = Some(tags),
                    Err(_) => return None,
                };
                input = &input[space_pos + 1..]
            } else {
                return None;
            }
        }

        if input.starts_with(':') {
            if let Some(space_pos) = input.find(' ') {
                match Message::parse_source(&input[1..space_pos]) {
                    Ok(source) => message.source = Some(source),
                    Err(_) => return None,
                };
                input = &input[space_pos + 1..]
            } else {
                return None;
            }
        }

        if let Some((command, params_input)) = input.split_once(' ') {
            if let Some((middle, trailing)) = params_input.split_once(" :") {
                let mut params = middle
                    .split(' ')
                    .map(|s| s.to_string())
                    .collect::<Vec<String>>();
                params.push(trailing.to_string());
                message.command = Command::new(command, params)
            } else {
                message.command = Command::new(
                    command,
                    params_input
                        .split(' ')
                        .map(|s| s.to_string())
                        .collect::<Vec<String>>(),
                );
            }
        } else {
            message.command = Command::new(input, Vec::new());
        }

        match message.command {
            Command::UNKNOWN => return None,
            _ => return Some(message),
        };
    }

    fn parse_tags(input: &str) -> Result<Vec<Tag>, ()> {
        // <tags>          ::= <tag> [';' <tag>]*

        let mut tags = Vec::new();

        for tag_input in input.split(';') {
            tags.push(Message::parse_tag(tag_input)?);
        }

        return Ok(tags);
    }

    fn parse_tag(input: &str) -> Result<Tag, ()> {
        // <tag>           ::= <key> ['=' <escaped value>]

        if let Some((key, value)) = input.split_once('=') {
            return Ok(Tag {
                key: Message::parse_key(key)?,
                value: Some(value.to_string()),
            });
        } else {
            return Ok(Tag {
                key: Message::parse_key(input)?,
                value: None,
            });
        }
    }

    fn parse_key(input: &str) -> Result<TagKey, ()> {
        // <key>           ::= [ <client_prefix> ] [ <vendor> '/' ] <sequence of letters, digits, hyphens (`-`)>
        // <client_prefix> ::= '+'
        // <escaped value> ::= <sequence of any characters except NUL, CR, LF, semicolon (`;`) and SPACE>
        // <vendor>        ::= <host>

        let mut key: TagKey = TagKey {
            client_prefix: None,
            vendor: None,
            value: String::new(),
        };
        let mut mut_input = input;

        if input.starts_with('+') {
            key.client_prefix = Some("+".to_string());
            mut_input = &input[1..]
        }

        if let Some((vendor, value)) = mut_input.split_once('=') {
            key.vendor = Some(vendor.to_string());
            key.value = value.to_string();
        } else {
            key.value = mut_input.to_string();
        }
        return Ok(key);
    }

    fn parse_source(input: &str) -> Result<Source, ()> {
        // source          ::=  <servername> / ( <nickname> [ "!" <user> ] [ "@" <host> ] )
        // nick            ::=  <any characters except NUL, CR, LF, chantype character, and SPACE> <possibly empty sequence of any characters except NUL, CR, LF, and SPACE>
        // user            ::=  <sequence of any characters except NUL, CR, LF, and SPACE>

        if let Some((rest, host)) = input.split_once('@') {
            if let Some((name, user)) = rest.split_once('!') {
                return Ok(Source {
                    name: name.to_string(),
                    user: Some(user.to_string()),
                    host: Some(host.to_string()),
                });
            } else {
                return Ok(Source {
                    name: rest.to_string(),
                    user: None,
                    host: Some(host.to_string()),
                });
            }
        } else {
            if let Some((name, user)) = input.split_once('!') {
                return Ok(Source {
                    name: name.to_string(),
                    user: Some(user.to_string()),
                    host: None,
                });
            } else {
                return Ok(Source {
                    name: input.to_string(),
                    user: None,
                    host: None,
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::types::Message;

    #[test]
    fn test1() {
        let message: Message = Message::from_bytes(
            "@id=234AB :dan!d@localhost PRIVMSG #chan :Hey what's up!".as_bytes(),
        )
        .unwrap();
        assert_eq!(
            "@id=234AB :dan!d@localhost PRIVMSG #chan :Hey what's up!\r\n",
            message.to_bytes()
        );
    }

    #[test]
    fn test2() {
        let message: Message = Message::from_bytes(
            ":irc.example.com CAP REQ :multi-prefix extended-join sasl".as_bytes(),
        )
        .unwrap();
        assert_eq!(
            ":irc.example.com CAP REQ :multi-prefix extended-join sasl\r\n",
            message.to_bytes()
        );
    }
}
