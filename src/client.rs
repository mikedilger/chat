
use std::io::{Read,Write};
use mio::tcp::TcpStream;
use mio::{Token,EventLoop,EventSet,PollOpt,Sender};
use handler::EventHandler;
use event_message::EventMessage;

#[derive(Debug,PartialEq,Eq,PartialOrd,Ord)]
pub enum ClientState {
    New,
    WaitingToGreet,
    WaitingForName,
    WaitingOrReadingChatMessage,
    WritingChatMessage,
}

pub struct Client {
    pub token: Token,
    socket: TcpStream,
    sender: Sender<EventMessage>,
    state: ClientState,
    incoming: Vec<u8>,
    outgoing: Vec<u8>,
    name: String,
}

impl Client {
    pub fn new(socket: TcpStream, token: Token, sender: Sender<EventMessage>) -> Client
    {
        Client {
            socket: socket,
            token: token,
            sender: sender,
            state: ClientState::New,
            incoming: Vec::with_capacity(1024),
            outgoing: Vec::with_capacity(1024),
            name: "Guest".to_string(),
        }
    }

    pub fn register(&mut self, event_loop: &mut EventLoop<EventHandler>)
    {
        if self.state == ClientState::New {
            self.state = ClientState::WaitingToGreet;
            event_loop.register(&self.socket,
                                self.token,
                                EventSet::writable() | EventSet::hup(),
                                PollOpt::edge() | PollOpt::oneshot()).unwrap();
            return;
        }

        let event_set = EventSet::hup() | match self.state {
            ClientState::New => unreachable!("Handled above"),
            ClientState::WaitingToGreet => EventSet::writable(),
            ClientState::WaitingForName => EventSet::readable(),
            ClientState::WaitingOrReadingChatMessage => EventSet::readable(),
            ClientState::WritingChatMessage => EventSet::writable(),
        };
        event_loop.reregister(&self.socket,
                              self.token,
                              event_set,
                              PollOpt::edge() | PollOpt::oneshot()).unwrap();
    }

    pub fn handle_readable(&mut self)
    {
        match self.state {
            ClientState::New => {
                println!("Event out of step: Readable, but ClientState::New");
                self.sender.send(EventMessage::ReArm(self.token)).unwrap();
            },
            ClientState::WaitingToGreet => {
                println!("Event out of step: Readable, but ClientState::WaitingToGreet");
                self.sender.send(EventMessage::ReArm(self.token)).unwrap();
            },
            ClientState::WaitingForName => {
                let mut buf: [u8; 1024] = [0; 1024];
                match self.socket.read(&mut buf[..]) {
                    Err(e) => {
                        println!("Read error: {:?}",e);
                        self.sender.send(EventMessage::Close(self.token)).unwrap();
                    },
                    Ok(0) => {
                        // We read zero bytes.  This means the peer has closed the connection.
                        self.sender.send(EventMessage::Close(self.token)).unwrap();
                    },
                    Ok(_size) => {
                        let name = String::from_utf8_lossy(&buf).into_owned();
                        self.name = name.trim_matches(|c: char| ! c.is_alphanumeric()).to_owned();
                        self.state = ClientState::WaitingOrReadingChatMessage;
                        self.sender.send(EventMessage::ReArm(self.token)).unwrap();
                    }
                }
            },
            ClientState::WaitingOrReadingChatMessage => {
                let mut buf: [u8; 1024] = [0; 1024];
                match self.socket.read(&mut buf[..]) {
                    Err(e) => {
                        println!("Read error: {:?}",e);
                        self.sender.send(EventMessage::Close(self.token)).unwrap();
                    },
                    Ok(0) => {
                        // We read zero bytes.  This means the peer has closed the connection.
                        self.sender.send(EventMessage::Close(self.token)).unwrap();
                    },
                    Ok(size) => {
                        self.process_incoming(&buf[..size]);
                        self.sender.send(EventMessage::ReArm(self.token)).unwrap();
                    }
                }
            },
            ClientState::WritingChatMessage => {
                println!("Event out of step: Readable, but ClientState::WritingChatMessage");
                self.sender.send(EventMessage::ReArm(self.token)).unwrap();
            }
        }
    }

    pub fn handle_writable(&mut self)
    {
        match self.state {
            ClientState::New => {
                println!("Event out of step: Writable, but ClientState::New");
                self.sender.send(EventMessage::ReArm(self.token)).unwrap();
            },
            ClientState::WaitingToGreet => {
                let greeting = b"Enter a chat handle for yourself: ";
                match self.socket.write(greeting) {
                    Ok(size) if size == greeting.len() => {
                        self.state = ClientState::WaitingForName;
                        self.sender.send(EventMessage::ReArm(self.token)).unwrap();
                    },
                    Ok(size) if size < greeting.len() => {
                        // FIXME: remember how much we wrote so we can write the
                        // rest later.  FOR NOW we will just move to the next
                        // state anyhow and never send the remainder.
                        self.state = ClientState::WaitingForName;
                        self.sender.send(EventMessage::ReArm(self.token)).unwrap();
                    },
                    Ok(_) => unreachable!("Read broke its promise"),
                    Err(e) => {
                        println!("Write error: {:?}",e);
                        self.sender.send(EventMessage::Close(self.token)).unwrap();
                    }
                }
            },
            ClientState::WaitingForName => {
                println!("Event out of step: Writable, but ClientState::WaitingForName");
                self.sender.send(EventMessage::ReArm(self.token)).unwrap();
            },
            ClientState::WaitingOrReadingChatMessage => {
                println!("Event out of step: Writable, but ClientState::WaitingOrReadingChatMessage");
                self.sender.send(EventMessage::ReArm(self.token)).unwrap();
            },
            ClientState::WritingChatMessage => {
                match self.socket.write(&mut self.outgoing) {
                    Err(e) => {
                        println!("Write error: {:?}",e);
                        self.sender.send(EventMessage::Close(self.token)).unwrap();
                    },
                    Ok(0) => {
                        // Write is finished.  Revert state
                        self.state = ClientState::WaitingOrReadingChatMessage;
                        self.sender.send(EventMessage::ReArm(self.token)).unwrap();
                    },
                    Ok(_size) => {
                        // FIXME - check for size and only take that much off the front
                        self.outgoing.truncate(0);
                    }
                }

                self.sender.send(EventMessage::ReArm(self.token)).unwrap();
            },
        }
    }

    pub fn handle_message(&mut self, message: String)
    {
        if self.state < ClientState::WaitingOrReadingChatMessage {
            // Do nothing if not yet setup
            return;
        }

        self.outgoing.push_all(message.as_bytes());
        self.outgoing.push(0x0A);
        self.state = ClientState::WritingChatMessage;
        self.sender.send(EventMessage::ReArm(self.token)).unwrap();
    }

    pub fn process_incoming(&mut self, buf: &[u8]) {
        self.incoming.push_all(&buf);

        loop {
            if let Some(cr) = self.incoming.iter().position(|c| *c==0x0A) {
                let mut message = self.incoming.split_off(cr);
                message.remove(0); // Drop the leading LF
                ::std::mem::swap(&mut self.incoming, &mut message);
                let message = format!("{}: {}", self.name, String::from_utf8_lossy(&message));
                self.sender.send(EventMessage::Message(self.token, message)).unwrap();
                continue;
            }
            break;
        }
    }
}
