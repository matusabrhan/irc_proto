#[cfg(feature = "std-stream")]
use std::io::{Read, Write};

#[cfg(feature = "tokio-stream")]
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[derive(Debug)]
pub enum ConnectionError {
    IOError,
    ParsingError,
}

use crate::message::Message;

#[derive(Debug)]
pub struct Connection {
    #[cfg(feature = "std-stream")]
    stream: std::net::TcpStream,
    #[cfg(feature = "tokio-stream")]
    stream: tokio::net::TcpStream,
    buffer: [u8; Self::BUFFER_SIZE],
    length: usize,
    cursor: usize,
}

#[cfg(feature = "std-stream")]
impl Connection {
    const BUFFER_SIZE: usize = 1024 * 2;

    pub fn new(stream: std::net::TcpStream) -> Self {
        Self {
            stream,
            buffer: [0; Self::BUFFER_SIZE],
            length: 0,
            cursor: 0,
        }
    }

    pub fn read(&mut self) -> Result<Message, ConnectionError> {
        if self.cursor >= self.length {
            self.length = self
                .stream
                .read(&mut self.buffer)
                .map_err(|_| ConnectionError::IOError)?;
            self.cursor = 0;
        }

        match Message::new(&self.buffer[self.cursor..self.length]) {
            Ok(message) => {
                self.cursor += message.contents().len();
                Ok(message)
            }
            Err(end) => {
                self.cursor += end;
                Err(ConnectionError::ParsingError)
            }
        }
    }

    pub fn write(&mut self, msg: Message) -> Result<(), ConnectionError> {
        self.stream
            .write(msg.contents().as_bytes())
            .map_err(|_| ConnectionError::IOError)?;
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
    const BUFFER_SIZE: usize = 1024 * 2;

    pub fn new(stream: tokio::net::TcpStream) -> Self {
        Self {
            stream,
            buffer: [0; Self::BUFFER_SIZE],
            length: 0,
            cursor: 0,
        }
    }

    pub async fn read(&mut self) -> Result<Message, ConnectionError> {
        if self.cursor >= self.length {
            self.length = self
                .stream
                .read(&mut self.buffer)
                .await
                .map_err(|_| ConnectionError::IOError)?;
            self.cursor = 0;
        }

        match Message::new(&self.buffer[self.cursor..self.length]) {
            Ok(message) => {
                self.cursor += message.contents().len();
                Ok(message)
            }
            Err(end) => {
                self.cursor += end;
                Err(ConnectionError::ParsingError)
            }
        }
    }

    pub async fn write(&mut self, msg: Message) -> Result<(), ConnectionError> {
        self.stream
            .write(msg.contents().as_bytes())
            .await
            .map_err(|_| ConnectionError::IOError)?;

        Ok(())
    }

    pub async fn close(&mut self) -> Result<(), ()> {
        self.stream.shutdown().await.map_err(|_| ())?;
        Ok(())
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
