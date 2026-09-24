#[cfg(feature = "std-stream")]
use std::io::{Read, Write};

#[cfg(feature = "tokio-stream")]
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::{broadcast, mpsc},
    task::JoinHandle,
};

use crate::message::Message;
use crate::parser::ParserErrorKind;
use crate::IrcError;

const MAX_MESSAGE_SIZE: usize = 512;
const BUFFER_SIZE: usize = 1024 * 2;

#[derive(Debug)]
struct Transport {
    #[cfg(feature = "std-stream")]
    stream: std::net::TcpStream,
    #[cfg(feature = "tokio-stream")]
    stream: tokio::net::TcpStream,
    buffer: [u8; BUFFER_SIZE],
    length: usize,
    cursor: usize,
}

#[cfg(feature = "std-stream")]
impl Transport {
    pub fn new(stream: std::net::TcpStream) -> Self {
        Self {
            stream,
            buffer: [0; BUFFER_SIZE],
            length: 0,
            cursor: 0,
        }
    }

    pub fn read(&mut self) -> Result<Message, IrcError> {
        if self.cursor >= self.length {
            self.length = self
                .stream
                .read(&mut self.buffer)
                .map_err(|_| IrcError::ConnectionError)?;
            self.cursor = 0;
        }

        match Message::new(&self.buffer[self.cursor..self.length]) {
            Ok(message) => {
                self.cursor += message.contents().len();
                Ok(message)
            }
            Err(IrcError::ParseError(err)) => {
                self.cursor += err.offset;
                Err(IrcError::ParseError(err))
            }
            Err(IrcError::ConnectionError) => unreachable!(),
        }
    }

    pub fn write(&mut self, msg: Message) -> Result<(), IrcError> {
        self.stream
            .write_all(msg.contents().as_bytes())
            .map_err(|_| IrcError::ConnectionError)?;
        Ok(())
    }

    pub fn close(&mut self) -> Result<(), ()> {
        self.stream
            .shutdown(std::net::Shutdown::Both)
            .map_err(|_| ())
    }
}

#[cfg(feature = "tokio-stream")]
impl Transport {
    pub fn new(stream: tokio::net::TcpStream) -> Self {
        Self {
            stream,
            buffer: [0; BUFFER_SIZE],
            length: 0,
            cursor: 0,
        }
    }

    pub async fn read(&mut self) -> Result<Message, IrcError> {
        if self.cursor >= self.length {
            self.length = match self
                .stream
                .read(&mut self.buffer)
                .await
                .map_err(|err| IrcError::ConnectionError(err))?
            {
                0 => return Err(IrcError::EOF),
                n => n,
            };
            self.cursor = 0;
        }

        loop {
            match Message::new(&self.buffer[self.cursor..self.length]) {
                Ok(message) => {
                    self.cursor += message.contents().len();
                    return Ok(message);
                }
                Err(err) => match err.kind {
                    ParserErrorKind::MissingEndOfMessage => {
                        if self.cursor > 0 {
                            self.buffer.copy_within(self.cursor..self.length, 0);
                            self.length -= self.cursor;
                            self.cursor = 0
                        }
                        if self.length >= MAX_MESSAGE_SIZE {
                            self.cursor = self.length;
                            return Err(IrcError::ParseError(err));
                        }
                        self.length += match self
                            .stream
                            .read(&mut self.buffer[self.length..])
                            .await
                            .map_err(|err| IrcError::ConnectionError(err))?
                        {
                            0 => return Err(IrcError::EOF),
                            n => n,
                        };
                    }
                    _ => {
                        self.cursor += err.offset;
                        return Err(IrcError::ParseError(err));
                    }
                },
            };
        }
    }

    pub async fn write(&mut self, msg: Message) -> Result<(), IrcError> {
        self.stream
            .write_all(msg.contents().as_bytes())
            .await
            .map_err(|err| IrcError::ConnectionError(err))?;

        Ok(())
    }

    pub async fn close(&mut self) -> Result<(), IrcError> {
        self.stream
            .shutdown()
            .await
            .map_err(|err| IrcError::ConnectionError(err))?;
        Ok(())
    }
}

pub enum IrcEvent {
    Message(Message),
    Error(IrcError),
    Closed,
}

#[cfg(feature = "tokio-stream")]
pub struct Connection {
    handle: JoinHandle<()>,
    tx: mpsc::Sender<Message>,
    rx: mpsc::Receiver<IrcEvent>,
    cancel: broadcast::Sender<()>,
}

#[cfg(feature = "tokio-stream")]
impl Connection {
    pub fn start(stream: tokio::net::TcpStream, channel_size: usize) -> Self {
        let (cancel_tx, mut cancel_rx) = broadcast::channel(1);
        let (source_tx, mut sink_rx) = mpsc::channel::<Message>(channel_size);
        let (sink_tx, source_rx) = mpsc::channel::<IrcEvent>(channel_size);

        let mut conn = Transport::new(stream);
        let handle = tokio::spawn(async move {
            loop {
                tokio::select! {
                    msg = conn.read() => {
                        match msg {
                            Ok(msg) => {
                                sink_tx.send(IrcEvent::Message(msg)).await;
                            }
                            Err(err) => {
                                sink_tx.send(IrcEvent::Error(err)).await;
                            },
                        }
                    }

                    msg = sink_rx.recv() => {
                        match msg {
                            Some(msg) => {
                                conn.write(msg).await;
                            }
                            None => {
                                sink_tx.send(IrcEvent::Closed).await;
                            },
                        }
                    }
                    _ = cancel_rx.recv() => break,
                }
            }
            sink_rx.close();
            conn.close().await;
        });

        Self {
            handle,
            tx: source_tx,
            rx: source_rx,
            cancel: cancel_tx,
        }
    }

    pub async fn recv(&mut self) -> Option<IrcEvent> {
        self.rx.recv().await
    }

    pub async fn send(&self, msg: Message) -> Result<(), ()> {
        self.tx.send(msg).await.map_err(|_| ())
    }

    pub async fn stop(self) -> Result<(), tokio::task::JoinError> {
        self.cancel.send(()).ok();
        self.handle.await
    }
}

#[cfg(feature = "std-stream")]
#[cfg(test)]
mod tests {
    use std::{
        io::Read,
        net::{SocketAddr, TcpListener, TcpStream},
    };

    use crate::{
        connection::Transport,
        message::{Command, MessageBuilder},
    };

    fn start_listen() -> (TcpListener, SocketAddr) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        return (listener, addr);
    }

    #[test]
    fn test_transport_write1() {
        let (listener, _) = start_listen();
        let stream = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        let (mut server, _) = listener.accept().unwrap();
        let mut client = Transport::new(stream);

        let message = MessageBuilder::with_command(Command::PRIVMSG {
            targets: "#chan",
            text: "Hello",
        })
        .build()
        .unwrap();
        client.write(message).unwrap();
        client.close();

        let mut buf = String::new();
        server.read_to_string(&mut buf);

        assert_eq!("PRIVMSG #chan Hello\r\n", buf);
    }

    #[test]
    fn test_message_parse5() {
        let (listener, _) = start_listen();
        let stream = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        let (mut server, _) = listener.accept().unwrap();
        let mut client = Transport::new(stream);

        let message1 = MessageBuilder::with_command(Command::PASS {
            password: "password",
        })
        .build()
        .unwrap();
        let message2 = MessageBuilder::with_command(Command::USER {
            user: "username_user1",
            mode: "0",
            unused: "*",
            realname: "realname_user1",
        })
        .build()
        .unwrap();
        let message3 = MessageBuilder::with_command(Command::NICK { nickname: "nick1" })
            .build()
            .unwrap();

        client.write(message1).unwrap();
        client.write(message2).unwrap();
        client.write(message3).unwrap();
        client.close();

        let mut buf = String::new();
        server.read_to_string(&mut buf);

        assert_eq!(
            "PASS password\r\nUSER username_user1 0 * realname_user1\r\nNICK nick1\r\n",
            buf
        );
    }
}

#[cfg(feature = "tokio-stream")]
#[cfg(test)]
mod tests {
    use super::Transport;
    use crate::{
        connection::{Connection, IrcEvent},
        message::{Command, MessageBuilder},
    };
    use log::info;
    use std::{net::SocketAddr, time::Duration};
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::{TcpListener, TcpStream},
        time::sleep,
    };

    async fn start_listen() -> (TcpListener, SocketAddr) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        info!("Server listening on {}", addr);
        return (listener, addr);
    }

    #[tokio::test]
    async fn test_transport_write1() {
        let (listener, _) = start_listen().await;
        let stream = TcpStream::connect(listener.local_addr().unwrap())
            .await
            .unwrap();
        let (mut server, _) = listener.accept().await.unwrap();
        let mut client = Transport::new(stream);

        let message = MessageBuilder::with_command(Command::PRIVMSG {
            targets: "#chan",
            text: "Hello",
        })
        .build()
        .unwrap();
        client.write(message).await.unwrap();
        client.close().await.unwrap();

        let mut res = String::new();
        server.read_to_string(&mut res).await.unwrap();

        assert_eq!("PRIVMSG #chan Hello\r\n", res);
        server.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_transport_write2() {
        let (listener, _) = start_listen().await;
        let stream = TcpStream::connect(listener.local_addr().unwrap())
            .await
            .unwrap();
        let (mut server, _) = listener.accept().await.unwrap();
        let mut client = Transport::new(stream);
        let message = MessageBuilder::with_command(Command::PRIVMSG {
            targets: "#chan",
            text: "Hello",
        })
        .build()
        .unwrap();
        client.write(message.clone()).await.unwrap();
        client.write(message).await.unwrap();
        client.close().await.unwrap();

        let mut res = String::new();
        server.read_to_string(&mut res).await.unwrap();

        assert_eq!("PRIVMSG #chan Hello\r\nPRIVMSG #chan Hello\r\n", res);
        server.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_transport_read1() {
        let (listener, _) = start_listen().await;
        let stream = TcpStream::connect(listener.local_addr().unwrap())
            .await
            .unwrap();
        let (mut server, _) = listener.accept().await.unwrap();
        let mut client = Transport::new(stream);

        server.write_all(b"PRIVMSG #chan Hello\r\n").await.unwrap();

        assert_eq!(
            "PRIVMSG #chan Hello\r\n",
            client.read().await.unwrap().contents(),
        );
        client.close().await.unwrap();
        server.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_transport_read2() {
        let (listener, _) = start_listen().await;
        let stream = TcpStream::connect(listener.local_addr().unwrap())
            .await
            .unwrap();
        let (mut server, _) = listener.accept().await.unwrap();
        let mut client = Transport::new(stream);

        server.write_all(b"PRIVMSG #chan Hello\r\n").await.unwrap();
        server.write_all(b"PRIVMSG #chan Hello\r\n").await.unwrap();

        let mut msg = String::new();
        msg.push_str(client.read().await.unwrap().contents());
        msg.push_str(client.read().await.unwrap().contents());

        assert_eq!("PRIVMSG #chan Hello\r\nPRIVMSG #chan Hello\r\n", msg);
        client.close().await.unwrap();
        server.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_transport_read3() {
        let (listener, _) = start_listen().await;
        let stream = TcpStream::connect(listener.local_addr().unwrap())
            .await
            .unwrap();
        let (mut server, _) = listener.accept().await.unwrap();
        let mut client = Transport::new(stream);

        server.write_all(b"PRIVMSG ").await.unwrap();
        tokio::spawn(async move {
            server.write_all(b"#ch").await.unwrap();
            server.write_all(b"an Hello\r\n").await.unwrap();

            sleep(Duration::from_secs(1)).await;
            server.shutdown().await.unwrap();
        });

        assert_eq!(
            "PRIVMSG #chan Hello\r\n",
            client.read().await.unwrap().contents(),
        );
        client.close().await.unwrap();
    }

    #[tokio::test]
    async fn test_connection() {
        let (listener, _) = start_listen().await;
        let client = Connection::start(
            TcpStream::connect(listener.local_addr().unwrap())
                .await
                .unwrap(),
            10,
        );
        let (stream, _) = listener.accept().await.unwrap();
        let mut server = Connection::start(stream, 10);

        let message_in = MessageBuilder::with_command(Command::PRIVMSG {
            targets: "foo",
            text: "bar",
        })
        .build()
        .unwrap();
        client.send(message_in.clone()).await.unwrap();

        match server.recv().await.unwrap() {
            IrcEvent::Message(msg) => assert_eq!(message_in.contents(), msg.contents()),
            _ => assert!(false),
        }
    }
}
