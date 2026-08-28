use criterion::{criterion_group, criterion_main, Criterion};
use irc_proto::message::{Command, IrcSerializable, Message};
use std::hint::black_box;

fn bench_parse_message(c: &mut Criterion) {
    c.bench_function("bench message parsing", |b| {
        let message_strings = vec![
            // ":irc.example.com PING server1 token",
            // ":irc.example.com PONG server1 token",
            "@id=234AB :dan!d@localhost PRIVMSG #chan :Hey what's up!",
            // ":irc.example.com USER username1 0 * realname1",
            // ":irc.example.com CAP REQ :multi-prefix extended-join sasl",
        ]
        .repeat(1000);
        b.iter(|| {
            for s in message_strings.clone() {
                let message = Message::from_u8(s.to_string().as_bytes()).unwrap();
                let text = match message.command() {
                    Command::PRIVMSG { text, .. } => Some(text),
                    _ => None,
                };
                black_box(text);
            }
        })
    });
}

criterion_group!(benches, bench_parse_message);
criterion_main!(benches);
