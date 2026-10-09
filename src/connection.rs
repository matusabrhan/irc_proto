#[cfg(feature = "std-stream")]
use std::io::{Read, Write};

#[cfg(feature = "tokio-stream")]
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::{broadcast, mpsc},
    task::JoinHandle,
};

use crate::parser::ParserErrorKind;
use crate::IrcError;
use crate::{message::Message, parser::ParserError};

const MAX_MESSAGE_SIZE: usize = 512;
const BUFFER_SIZE: usize = 1024 * 2;

#[derive(Debug)]
struct Transport<S> {
    stream: S,
    decoder: IrcDecoder,
    encoder: IrcEncoder,
}

#[derive(Debug)]
struct IrcDecoder {
    buffer: [u8; BUFFER_SIZE],
    cursor: u16,
    length: u16,
}

#[derive(Debug)]
struct IrcEncoder {
    buffer: [u8; MAX_MESSAGE_SIZE],
    cursor: u16,
    length: u16,
}

impl IrcDecoder {
    fn decode(&mut self) -> Result<Option<Message>, ParserError> {
        match Message::new(&self.buffer[self.cursor as usize..self.length as usize]) {
            Ok(message) => {
                let message_len = message.contents().len() as u16;
                self.buffer.copy_within(
                    (self.cursor + message_len) as usize..self.length as usize,
                    0,
                );
                self.cursor = 0;
                self.length -= message_len;
                Ok(Some(message))
            }
            Err(err) => match err.kind {
                ParserErrorKind::MissingEndOfMessage => {
                    if self.length as usize >= MAX_MESSAGE_SIZE {
                        self.cursor = self.length;
                        return Err(err);
                    }
                    Ok(None)
                }
                _ => {
                    self.cursor += err.end as u16;
                    Err(err)
                }
            },
        }
    }

    fn has_remaining(&self) -> bool {
        self.cursor < self.length
    }

    fn set_length(&mut self, length: usize) {
        self.length = length as u16;
        self.cursor = 0;
    }

    fn buffer_mut(&mut self) -> &mut [u8] {
        &mut self.buffer[self.length as usize..]
    }

    fn update_length(&mut self, length: usize) {
        self.length += length as u16
    }
}

impl IrcEncoder {
    fn load(&mut self, msg: Message) {
        let contents = msg.contents().as_bytes();
        self.buffer[..contents.len()].copy_from_slice(contents);
        self.cursor = 0;
        self.length = contents.len() as u16;
    }

    fn encode(&self) -> &[u8] {
        &self.buffer[self.cursor as usize..self.length as usize]
    }

    fn has_remaining(&self) -> bool {
        self.cursor < self.length
    }

    fn update_cursor(&mut self, read: usize) {
        self.cursor += read as u16;
    }
}

impl<S> Transport<S> {
    pub fn new(stream: S) -> Self {
        Self {
            stream,
            decoder: IrcDecoder {
                buffer: [0; BUFFER_SIZE],
                cursor: 0,
                length: 0,
            },
            encoder: IrcEncoder {
                buffer: [0; MAX_MESSAGE_SIZE],
                cursor: 0,
                length: 0,
            },
        }
    }
}

#[cfg(feature = "std-stream")]
impl<S: Read + Write + Unpin> Transport<S> {
    pub fn read(&mut self) -> Result<Message, IrcError> {
        if !self.decoder.has_remaining() {
            match self
                .stream
                .read(self.decoder.buffer_mut())
                .map_err(|err| IrcError::ConnectionError(err))?
            {
                0 => return Err(IrcError::EOF),
                n => self.decoder.set_length(n),
            };
        }

        loop {
            match self
                .decoder
                .decode()
                .map_err(|err| IrcError::ParseError(err))?
            {
                Some(message) => return Ok(message),
                None => {
                    match self
                        .stream
                        .read(self.decoder.buffer_mut())
                        .map_err(|err| IrcError::ConnectionError(err))?
                    {
                        0 => return Err(IrcError::EOF),
                        n => self.decoder.update_length(n),
                    };
                }
            };
        }
    }

    pub fn write(&mut self, msg: Message) -> Result<(), IrcError> {
        self.encoder.load(msg);
        while self.encoder.has_remaining() {
            match self
                .stream
                .write(self.encoder.encode())
                .map_err(|err| IrcError::ConnectionError(err))?
            {
                0 => return Err(IrcError::EOF),
                n => self.encoder.update_cursor(n),
            }
        }
        Ok(())
    }

    pub fn close(self) {
        drop(self.stream)
    }
}

#[cfg(feature = "tokio-stream")]
impl<S: AsyncReadExt + AsyncWriteExt + Unpin> Transport<S> {
    pub async fn read(&mut self) -> Result<Message, IrcError> {
        if !self.decoder.has_remaining() {
            match self
                .stream
                .read(self.decoder.buffer_mut())
                .await
                .map_err(IrcError::ConnectionError)?
            {
                0 => return Err(IrcError::EOF),
                n => self.decoder.set_length(n),
            };
        }

        loop {
            match self.decoder.decode().map_err(IrcError::ParseError)? {
                Some(message) => return Ok(message),
                None => {
                    match self
                        .stream
                        .read(self.decoder.buffer_mut())
                        .await
                        .map_err(IrcError::ConnectionError)?
                    {
                        0 => return Err(IrcError::EOF),
                        n => self.decoder.update_length(n),
                    };
                }
            };
        }
    }

    pub async fn write(&mut self, msg: Message) -> Result<(), IrcError> {
        self.encoder.load(msg);
        while self.encoder.has_remaining() {
            match self
                .stream
                .write(self.encoder.encode())
                .await
                .map_err(IrcError::ConnectionError)?
            {
                0 => return Err(IrcError::EOF),
                n => self.encoder.update_cursor(n),
            }
        }
        Ok(())
    }

    pub async fn close(&mut self) -> Result<(), IrcError> {
        self.stream
            .shutdown()
            .await
            .map_err(IrcError::ConnectionError)?;
        Ok(())
    }
}

#[cfg(feature = "tokio-stream")]
#[derive(Debug)]
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

        let mut transport = Transport::new(stream);
        let handle = tokio::spawn(async move {
            loop {
                tokio::select! {
                    msg = transport.read() => {
                        match msg {
                            Ok(msg) => {
                                sink_tx.send(IrcEvent::Message(msg)).await.ok();
                            }
                            Err(err) => {
                                sink_tx.send(IrcEvent::Error(err)).await.ok();
                            },
                        }
                    }

                    msg = sink_rx.recv() => {
                        match msg {
                            Some(msg) => {
                                if let Err(err) = transport.write(msg).await {
                                    sink_tx.send(IrcEvent::Error(err)).await.ok();
                                }
                            }
                            None => {
                                sink_tx.send(IrcEvent::Closed).await.ok();
                            },
                        }
                    }
                    _ = cancel_rx.recv() => break,
                }
            }
            sink_rx.close();
            transport.close().await.ok();
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

    pub async fn send(&self, msg: Message) -> Result<(), mpsc::error::SendError<Message>> {
        self.tx.send(msg).await
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
        enable_logging,
        message::{Command, MessageBuilder},
    };

    // fn start_listen() -> (TcpListener, SocketAddr) {
    //     let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    //     let addr = listener.local_addr().unwrap();
    //     return (listener, addr);
    // }
    //
    // #[test]
    // fn test_transport_write1() {
    //     let (listener, _) = start_listen();
    //     let stream = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
    //     let (mut server, _) = listener.accept().unwrap();
    //     let mut client = Transport::new(stream);
    //
    //     let message = MessageBuilder::with_command(Command::PRIVMSG {
    //         targets: "#chan",
    //         text: "Hello",
    //     })
    //     .build()
    //     .unwrap();
    //     client.write(message).unwrap();
    //
    //     let mut buf = String::new();
    //     server.read_to_string(&mut buf);
    //
    //     assert_eq!("PRIVMSG #chan Hello\r\n", buf);
    // }

    fn start_listen() -> (TcpListener, SocketAddr) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        log::info!("Server listening on {}", addr);
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

        let mut res = String::new();
        server.read_to_string(&mut res).unwrap();

        assert_eq!("PRIVMSG #chan Hello\r\n", res);
        server.shutdown(std::net::Shutdown::Both).unwrap();
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
        server.read_to_string(&mut buf).unwrap();

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

        server.write_all(b"PRIVMSG #chan Hello1\r\n").await.unwrap();
        server.write_all(b"PRIVMSG #chan Hello2\r\n").await.unwrap();

        let mut msg = String::new();
        msg.push_str(client.read().await.unwrap().contents());
        msg.push_str(client.read().await.unwrap().contents());

        assert_eq!("PRIVMSG #chan Hello1\r\nPRIVMSG #chan Hello2\r\n", msg);
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
            server.flush().await.unwrap();
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
    async fn test_transport_read4() {
        let (listener, _) = start_listen().await;
        let stream = TcpStream::connect(listener.local_addr().unwrap())
            .await
            .unwrap();
        let (mut server, _) = listener.accept().await.unwrap();
        let mut client = Transport::new(stream);

        server
            .write_all(b"PRIVMSG #chan Hello\r\nINVALID\r\nPING token\r\n")
            .await
            .unwrap();

        assert_eq!(
            "PRIVMSG #chan Hello\r\n",
            client.read().await.unwrap().contents(),
        );
        assert!(client.read().await.is_err());
        assert_eq!("PING token\r\n", client.read().await.unwrap().contents(),);
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
