
use std::io::{Read,Write};
use mio::tcp::TcpStream;
use mio::{Token,EventLoop,EventSet,PollOpt,Sender};
use handler::EventHandler;
use event_message::EventMessage;

#[derive(Debug,PartialEq,Eq)]
pub enum ClientState {
    New,
    WaitingOrReadingChatMessage,
    WritingChatMessage,
}

pub struct Client {
    socket: TcpStream,
    token: Token,
    sender: Sender<EventMessage>,
    state: ClientState,
    incoming: Vec<u8>,
    outgoing: Vec<u8>,
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
        }
    }

    pub fn register(&mut self, event_loop: &mut EventLoop<EventHandler>)
    {
        if self.state == ClientState::New {
            self.state = ClientState::WaitingOrReadingChatMessage;
            event_loop.register(&self.socket,
                                self.token,
                                EventSet::readable() | EventSet::hup(),
                                PollOpt::edge() | PollOpt::oneshot()).unwrap();
            return;
        }

        let event_set = EventSet::hup() | match self.state {
            ClientState::New => unreachable!("Handled above"),
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
            ClientState::New =>
                unreachable!("State should have moved passed new prior to first register"),
            ClientState::WaitingOrReadingChatMessage => {
                // Read and echo to the console
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
            ClientState::WritingChatMessage =>
                unreachable!("We are not waiting for read (WritingChatMessage)"),
        }
    }

    pub fn handle_writable(&mut self)
    {
        match self.state {
            ClientState::New =>
                unreachable!("State should have moved passed new prior to first register"),
            ClientState::WaitingOrReadingChatMessage =>
                unreachable!("We are not waiting for write (WaitingOrReadingChatMessage)"),
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

    pub fn handle_message(&mut self, message: Vec<u8>)
    {
        self.outgoing.push_all(&message[..]);
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
                println!("{}", String::from_utf8_lossy(&message));
                self.sender.send(EventMessage::Message(message.clone())).unwrap();
                continue;
            }
            break;
        }
    }
}
