use irc_proto::{ast::NodeKind, message_v2::MessageV2};

fn main() {
    let message_strings = vec![
        // ":irc.example.com PING server1 token\r\n",
        // ":irc.example.com PONG server1 token\r\n",
        "@id=234AB :dan!d@localhost PRIVMSG #chan :Hey what's up!\r\n",
        // ":irc.example.com USER username1 0 * realname1\r\n",
        // ":irc.example.com CAP REQ :multi-prefix extended-join sasl\r\n",
    ]
    .repeat(1000);

    let mut results = Vec::new();

    for s in message_strings.clone() {
        let message = MessageV2::new(s.as_bytes().to_vec()).unwrap();

        let text = match message.get_command().kind() {
            NodeKind::CommandPrivMsg { text, .. } => {
                message.get_value(text.clone()).map(|s| s.to_string())
            }
            _ => None,
        };
        results.push(text);
    }

    println!("{:?}", results);
}
