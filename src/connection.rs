use std::cell::RefCell;
#[cfg(feature = "std-stream")]
use std::io::{Read, Write};
use std::time::Duration;

#[cfg(feature = "tokio-stream")]
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::{broadcast, mpsc};
use tokio::task::JoinHandle;

use crate::message::Message;
use crate::parser::ParserErrorKind;
use crate::IrcError;

const MAX_MESSAGE_SIZE: usize = 512;
const BUFFER_SIZE: usize = 1024 * 2;

#[derive(Debug)]
pub struct Connection {
    #[cfg(feature = "std-stream")]
    stream: std::net::TcpStream,
    #[cfg(feature = "tokio-stream")]
    stream: tokio::net::TcpStream,
    buffer: [u8; BUFFER_SIZE],
    length: usize,
    cursor: usize,
}

#[cfg(feature = "std-stream")]
impl Connection {
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
            Err(IrcError::NonUtf8Input) => {
                self.cursor = self.length;
                return Err(IrcError::NonUtf8Input);
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
impl Connection {
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
                .map_err(|_| IrcError::ConnectionError)?
            {
                0 => return Err(IrcError::ConnectionError),
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
                Err(IrcError::ConnectionError) => unreachable!(),
                Err(IrcError::ParseError(err)) => match err.kind {
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
                            .map_err(|_| IrcError::ConnectionError)?
                        {
                            0 => return Err(IrcError::ConnectionError),
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
            .map_err(|_| IrcError::ConnectionError)?;

        Ok(())
    }

    pub async fn close(&mut self) -> Result<(), IrcError> {
        self.stream
            .shutdown()
            .await
            .map_err(|_| ())
            .map_err(|_| IrcError::ConnectionError)?;
        Ok(())
    }
}

pub struct Transport {
    handle: JoinHandle<()>,
    tx: mpsc::Sender<Message>,
    rx: RefCell<mpsc::Receiver<Message>>,
    cancel: broadcast::Sender<()>,
}

impl Transport {
    pub fn start(stream: tokio::net::TcpStream, channel_size: usize) -> Self {
        let (cancel_tx, mut cancel_rx) = broadcast::channel(1);
        let (server_tx, mut client_rx) = mpsc::channel::<Message>(channel_size);
        let (client_tx, server_rx) = mpsc::channel::<Message>(channel_size);

        let mut conn = Connection::new(stream);
        let handle = tokio::spawn(async move {
            loop {
                tokio::select! {
                    msg = conn.read() => {
                        match msg {
                            Ok(msg) => if client_tx.send(msg).await.is_err() { break }
                            Err(IrcError::ConnectionError) => {},
                            Err(IrcError::ParseError { .. }) => {},
                        }
                    }

                    msg = client_rx.recv() => {
                        match msg {
                            Some(msg) => if conn.write(msg).await.is_err() { break }
                            None => break,
                        }
                    }
                    _ = cancel_rx.recv() => break,
                }
            }
            client_rx.close();
        });

        Self {
            handle,
            tx: server_tx,
            rx: RefCell::new(server_rx),
            cancel: cancel_tx,
        }
    }

    pub async fn recv(&self) -> Option<Message> {
        self.rx.borrow_mut().recv().await
    }

    pub async fn send(&self, msg: Message) -> Result<(), ()> {
        self.tx.send(msg).await.map_err(|_| ())
    }

    pub async fn stop(&self) {
        while self.cancel.send(()).is_ok() {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        if !self.handle.is_finished() {
            self.handle.abort();
        }
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
        connection::Connection,
        message::{Command, MessageBuilder},
    };

    fn start_listen() -> (TcpListener, SocketAddr) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        return (listener, addr);
    }

    #[test]
    fn test_connection_write1() {
        let (listener, _) = start_listen();
        let stream = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        let (mut server, _) = listener.accept().unwrap();
        let mut client = Connection::new(stream);

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
        let mut client = Connection::new(stream);

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
    use super::Connection;
    use crate::{
        connection::Transport,
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
    async fn test_connection_write1() {
        let (listener, _) = start_listen().await;
        let stream = TcpStream::connect(listener.local_addr().unwrap())
            .await
            .unwrap();
        let (mut server, _) = listener.accept().await.unwrap();
        let mut client = Connection::new(stream);

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
    async fn test_connection_write2() {
        let (listener, _) = start_listen().await;
        let stream = TcpStream::connect(listener.local_addr().unwrap())
            .await
            .unwrap();
        let (mut server, _) = listener.accept().await.unwrap();
        let mut client = Connection::new(stream);
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
    async fn test_connection_read1() {
        let (listener, _) = start_listen().await;
        let stream = TcpStream::connect(listener.local_addr().unwrap())
            .await
            .unwrap();
        let (mut server, _) = listener.accept().await.unwrap();
        let mut client = Connection::new(stream);

        server.write_all(b"PRIVMSG #chan Hello\r\n").await.unwrap();

        assert_eq!(
            "PRIVMSG #chan Hello\r\n",
            client.read().await.unwrap().contents(),
        );
        client.close().await.unwrap();
        server.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_connection_read2() {
        let (listener, _) = start_listen().await;
        let stream = TcpStream::connect(listener.local_addr().unwrap())
            .await
            .unwrap();
        let (mut server, _) = listener.accept().await.unwrap();
        let mut client = Connection::new(stream);

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
    async fn test_connection_read3() {
        let (listener, _) = start_listen().await;
        let stream = TcpStream::connect(listener.local_addr().unwrap())
            .await
            .unwrap();
        let (mut server, _) = listener.accept().await.unwrap();
        let mut client = Connection::new(stream);

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
    async fn test_transport() {
        let (listener, _) = start_listen().await;
        let client = Transport::start(
            TcpStream::connect(listener.local_addr().unwrap())
                .await
                .unwrap(),
            10,
        );
        let (stream, _) = listener.accept().await.unwrap();
        let server = Transport::start(stream, 10);

        let message_in = MessageBuilder::with_command(Command::PRIVMSG {
            targets: "foo",
            text: "bar",
        })
        .build()
        .unwrap();
        client.send(message_in.clone()).await.unwrap();

        let message_out = server.recv().await.unwrap();

        assert_eq!(message_in.contents(), message_out.contents())
    }
}
