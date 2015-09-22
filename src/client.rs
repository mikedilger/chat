
use std::io::{Read,Write};
use std::sync::Arc;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;

use mio::tcp::TcpStream;
use mio::{Token,EventLoop,EventSet,PollOpt,Sender};
use http_muncher::{Parser, ParserHandler};

use handler::EventHandler;
use event_message::EventMessage;
use http::HttpParser;

#[derive(Debug,PartialEq,Eq,PartialOrd,Ord)]
pub enum ClientState {
    New,
    AwaitingHandshake,
    HandshakeResponse,
    Running,
    RunningAndWriting,
}

pub struct Client {
    pub token: Token,
    socket: TcpStream,
    headers: Arc<RefCell<HashMap<String, String>>>, // FIXME try linear_map
    http_parser: Parser<HttpParser>,
    sender: Sender<EventMessage>,
    state: ClientState,
    incoming: Vec<u8>,
    outgoing: Vec<u8>,
    name: String,
}

impl Client {
    pub fn new(socket: TcpStream, token: Token, sender: Sender<EventMessage>) -> Client
    {
        let headers = Arc::new(RefCell::new(HashMap::new()));
        Client {
            socket: socket,
            headers: headers.clone(),
            http_parser: Parser::request(HttpParser {
                current_key: None,
                headers: headers.clone()
            }),
            token: token,
            sender: sender,
            state: ClientState::AwaitingHandshake,
            incoming: Vec::with_capacity(1024),
            outgoing: Vec::with_capacity(1024),
            name: "Guest".to_string(),
        }
    }

    /// (re)register interest in events, chosen based on the current state
    pub fn register(&mut self, event_loop: &mut EventLoop<EventHandler>)
    {
        if self.state == ClientState::New {
            self.state = ClientState::AwaitingHandshake;
            event_loop.register(&self.socket,
                                self.token,
                                EventSet::readable() | EventSet::hup(),
                                PollOpt::edge() | PollOpt::oneshot()).unwrap();
            return;
        }

        let event_set = EventSet::hup() | match self.state {
            ClientState::New => unreachable!("Handled above"),
            ClientState::AwaitingHandshake => EventSet::readable(),
            ClientState::HandshakeResponse => EventSet::writable(),
            ClientState::Running => EventSet::readable(),
            ClientState::RunningAndWriting => EventSet::writable() | EventSet::readable(),
        };
        event_loop.reregister(&self.socket,
                              self.token,
                              event_set,
                              PollOpt::edge() | PollOpt::oneshot()).unwrap();
    }

    /// Handle a readable event
    pub fn handle_readable(&mut self)
    {
        match self.state {
            ClientState::New => {
                println!("Event out of step: Readable, but ClientState::New");
                self.sender.send(EventMessage::ReArm(self.token)).unwrap();
            },
            ClientState::AwaitingHandshake => {
                // FIXME: read in a loop
                let mut buf = [0; 2048];
                match self.socket.read(&mut buf) {
                    Err(e) => {
                        println!("Read error: {:?}",e);
                        self.sender.send(EventMessage::Close(self.token)).unwrap();
                    },
                    Ok(0) => {
                        // We read zero bytes.  This means the peer has closed the connection.
                        self.sender.send(EventMessage::Close(self.token)).unwrap();
                    },
                    Ok(size) => {
                        self.http_parser.parse(&buf);
                        if self.http_parser.is_upgrade() {
                            let headers = self.headers.borrow();
                            let response_key = ::http::gen_key(
                                &headers.get("Sec-WebSocket-Key").unwrap());
                            self.outgoing = fmt::format(
                                format_args!("HTTP/1.1 101 Switching Protocols\r\n\
                                              Connection: Upgrade\r\n\
                                              Sec-WebSocket-Accept: {}\r\n\
                                              Upgrade: websocket\r\n\r\n",
                                             response_key)).as_bytes();
                            self.state = ClientState::HandshakeResponse;
                        }
                        else {
                            panic!("FIXME: we need to keep reading into a buffer somewhere");
                            // and loop around.
                        }
                    }
                }
            },
            ClientState::HandshakeResponse => {
                println!("Event out of step: Readable, but ClientState::WaitingToGreet");
                self.sender.send(EventMessage::ReArm(self.token)).unwrap();
            },
            ClientState::Running | ClientState::RunningAndWriting => {
                // FIXME: read in a loop
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
        }
    }

    pub fn handle_writable(&mut self)
    {
        match self.state {
            ClientState::New => {
                println!("Event out of step: Writable, but ClientState::New");
                self.sender.send(EventMessage::ReArm(self.token)).unwrap();
            },
            ClientState::AwaitingHandshake => {
                println!("Event out of step: Writable, but ClientState::AwaitingHandshake");
                self.sender.send(EventMessage::ReArm(self.token)).unwrap();
            },
            ClientState::HandshakeResponse | ClientState::RunningAndWriting => {
                // FIXME: write in a loop
                match self.socket.write(&mut self.outgoing) {
                    Err(e) => {
                        println!("Write error: {:?}",e);
                        self.sender.send(EventMessage::Close(self.token)).unwrap();
                    },
                    Ok(0) => {
                        // Write is finished.  Revert state
                        self.state = ClientState::Running;
                        self.sender.send(EventMessage::ReArm(self.token)).unwrap();
                    },
                    Ok(size) if size==self.outgoing.len() => {
                        self.outgoing.truncate(0);
                        self.state = ClientState::Running;
                        self.sender.send(EventMessage::ReArm(self.token)).unwrap();
                    }
                    Ok(size) => {
                        // FIXME - just take 'size' bytes off the front
                        self.outgoing.truncate(0);
                        self.sender.send(EventMessage::ReArm(self.token)).unwrap();
                    }
                }
            },
            ClientState::Running => {
                println!("Event out of step: Writable, but ClientState::Running");
                self.sender.send(EventMessage::ReArm(self.token)).unwrap();
            },
        }
    }

    pub fn handle_message(&mut self, message: String)
    {
        if self.state < ClientState::Running {
            // Do nothing if not yet setup
            return;
        }

        self.outgoing.push_all(message.as_bytes());
        self.outgoing.push(0x0A);
        self.state = ClientState::RunningAndWriting;
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
