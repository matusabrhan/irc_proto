use crate::error::ConnectionError;
use crate::message::{IrcCursor, IrcSerializable, Message, MESSAGE_MAX_LENGTH};
use bytes::{Buf, BytesMut};
use std::{io::Cursor, net::SocketAddr};
use tokio::{
    self,
    io::{self, AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

pub const BUFFER_SIZE: usize = 1024 * 2;

#[derive(Debug)]
pub struct Connection {
    stream: TcpStream,
    buffer: BytesMut,
}

impl Connection {
    pub fn new(stream: TcpStream) -> Self {
        return Connection {
            stream: stream,
            buffer: BytesMut::with_capacity(BUFFER_SIZE),
        };
    }

    pub async fn read(&mut self) -> Result<Message, ConnectionError> {
        loop {
            if let Some(msg) = self.parse_frame() {
                return Ok(msg);
            }

            match self.stream.read_buf(&mut self.buffer).await {
                Ok(0) | Err(_) => {
                    return Err(ConnectionError::new("client exited".to_string()));
                }
                Ok(_) => {}
            }
        }
    }

    pub async fn write(&mut self, msg: Message) -> Result<(), ConnectionError> {
        let msg_bytes = msg.to_vec_u8();
        let mut cursor: Cursor<&[u8]> = Cursor::new(&msg_bytes);
        while cursor.has_remaining() {
            match self
                .stream
                .write(&cursor.get_ref()[cursor.position() as usize..])
                .await
            {
                Ok(0) | Err(_) => {
                    return Err(ConnectionError::new("client exited".to_string()));
                }
                Ok(n) => {
                    cursor.advance(n);
                }
            }
        }
        Ok(())
    }

    pub fn peer_address(&self) -> io::Result<SocketAddr> {
        self.stream.peer_addr()
    }

    pub fn local_address(&self) -> io::Result<SocketAddr> {
        self.stream.local_addr()
    }

    pub async fn shutdown(&mut self) {
        self.stream
            .shutdown()
            .await
            .expect("could not shutdown connection")
    }

    fn parse_frame(&mut self) -> Option<Message> {
        let mut cursor = Cursor::new(self.buffer.chunk());
        match cursor.split_frame() {
            Some(frame) => match frame.len() {
                n if n > MESSAGE_MAX_LENGTH => {
                    self.buffer.advance(n);
                    None
                }
                _ => {
                    let message = Message::from_u8(frame);
                    self.buffer.advance(cursor.position() as usize);
                    message.ok()
                }
            },
            None => {
                let n = cursor.get_ref().len();
                if n > MESSAGE_MAX_LENGTH {
                    self.buffer.advance(n);
                }
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Connection;
    use crate::message::{Command, IrcSerializable, Message};
    use log::info;
    use std::net::SocketAddr;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::{TcpListener, TcpStream},
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
        info!("Client connected from {}", client.local_address().unwrap());

        let message = Message::new(
            None,
            None,
            Command::PRIVMSG {
                targets: vec!["#chan".to_string()],
                text: "Hello".to_string(),
            },
        );
        client.write(message).await.unwrap();
        client.shutdown().await;

        let mut res = String::new();
        server.read_to_string(&mut res).await.unwrap();

        assert_eq!("PRIVMSG #chan Hello\r\n", res);
        client.shutdown().await;
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
        info!("Client connected from {}", client.local_address().unwrap());

        let message = Message::new(
            None,
            None,
            Command::PRIVMSG {
                targets: vec!["#chan".to_string()],
                text: "Hello".to_string(),
            },
        );
        client.write(message.clone()).await.unwrap();
        client.write(message).await.unwrap();
        client.shutdown().await;

        let mut res = String::new();
        server.read_to_string(&mut res).await.unwrap();

        assert_eq!("PRIVMSG #chan Hello\r\nPRIVMSG #chan Hello\r\n", res);
        client.shutdown().await;
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
        info!("Client connected from {}", client.local_address().unwrap());

        server.write_all(b"PRIVMSG #chan Hello\r\n").await.unwrap();

        assert_eq!(
            String::from("PRIVMSG #chan Hello\r\n"),
            String::from_utf8(client.read().await.unwrap().to_vec_u8()).unwrap()
        );
        client.shutdown().await;
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
        info!("Client connected from {}", client.local_address().unwrap());

        server.write_all(b"PRIVMSG #chan Hello\r\n").await.unwrap();
        server.write_all(b"PRIVMSG #chan Hello\r\n").await.unwrap();

        let mut msg = String::new();
        msg.push_str(
            String::from_utf8(client.read().await.unwrap().to_vec_u8())
                .unwrap()
                .as_str(),
        );
        msg.push_str(
            String::from_utf8(client.read().await.unwrap().to_vec_u8())
                .unwrap()
                .as_str(),
        );

        assert_eq!(
            String::from("PRIVMSG #chan Hello\r\nPRIVMSG #chan Hello\r\n"),
            msg
        );
        client.shutdown().await;
        server.shutdown().await.unwrap();
    }
}
