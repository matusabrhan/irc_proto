use std::{
    io::{Read, Write},
    net::{Shutdown, TcpStream},
};

use crate::message::Message;

#[derive(Debug)]
pub struct Connection {
    stream: TcpStream,
    buffer: [u8; Self::BUFFER_SIZE],
}

impl Connection {
    const BUFFER_SIZE: usize = 1024 * 2;

    pub fn new(stream: TcpStream) -> Self {
        return Connection {
            stream: stream,
            buffer: [0; Self::BUFFER_SIZE],
        };
    }

    pub fn read(&mut self) -> Result<Message, ()> {
        let n = self.stream.read(&mut self.buffer).map_err(|_| ())?;
        let x = Message::new(&self.buffer[0..n]);
        x.ok_or(())
    }

    pub fn write(&mut self, msg: Message) -> Result<(), ()> {
        self.stream
            .write(msg.contents().as_bytes())
            .map_err(|_| ())?;
        Ok(())
    }

    pub fn close(&mut self) -> Result<(), ()> {
        self.stream.shutdown(Shutdown::Both).map_err(|_| ())
    }
}

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
}
