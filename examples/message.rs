use irc_proto::message::{Command, Message, MessageBuilder};

fn main() {
    let mut message_strings = Vec::new();

    message_strings.push(
        MessageBuilder::with_command(Command::PONG {
            server: None,
            token: "token",
        })
        .with_source("irc.example.com", None, None)
        .build()
        .unwrap()
        .contents()
        .to_string(),
    );

    message_strings.push(
        MessageBuilder::with_command(Command::PRIVMSG {
            targets: "#chan",
            text: "Hey what's up!",
        })
        .with_source("dan", Some("d"), Some("localhost"))
        .build()
        .unwrap()
        .contents()
        .to_string(),
    );

    message_strings.push(
        MessageBuilder::with_command(Command::USER {
            user: "username1",
            mode: "0",
            unused: "*",
            realname: "realname1",
        })
        .with_source("irc.example.com", None, None)
        .build()
        .unwrap()
        .contents()
        .to_string(),
    );

    message_strings.push(
        MessageBuilder::with_command(Command::CAP {
            subcommand: "REQ",
            capabilities: Some("multi-prefix extended-join sasl"),
        })
        .with_source("irc.example.com", None, None)
        .build()
        .unwrap()
        .contents()
        .to_string(),
    );

    for s in message_strings {
        let message = Message::new(s.as_bytes()).unwrap();

        let text = message.contents();
        println!("{:?}", text);
    }
}
