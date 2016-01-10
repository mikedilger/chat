
use std::io::{Read,Write,ErrorKind};
use mio::tcp::TcpStream;
use mio::{Token,EventLoop,EventSet,PollOpt,Sender};
use handler::EventHandler;
use event_message::EventMessage;
use http_parser::HttpParser;
use http_muncher::Parser;
use sha1;
use rustc_serialize::base64::{ToBase64, STANDARD};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::fmt;


#[derive(Debug,PartialEq,Eq,PartialOrd,Ord)]
pub enum ClientState {
    New,
    AwaitingHandshake,
    HandshakeResponse,
    Running,
    RunningAndWriting,
}

impl ClientState {
    pub fn next(&self) -> ClientState {
        match *self {
            ClientState::New => ClientState::AwaitingHandshake,
            ClientState::AwaitingHandshake => ClientState::HandshakeResponse,
            ClientState::HandshakeResponse => ClientState::Running,
            ClientState::Running => ClientState::Running,
            ClientState::RunningAndWriting => ClientState::Running,
        }
    }
}

pub struct Client {
    pub token: Token,
    socket: TcpStream,
    sender: Sender<EventMessage>,
    state: ClientState,
    incoming: Vec<u8>,
    outgoing: Vec<u8>,
    name: String,
    headers: Arc<Mutex<HashMap<String, String>>>,
    http_parser: Parser<HttpParser>,
}

impl Client {
    pub fn new(socket: TcpStream, token: Token, sender: Sender<EventMessage>) -> Client
    {
        let headers = Arc::new(Mutex::new(HashMap::new()));

        Client {
            socket: socket,
            token: token,
            sender: sender,
            state: ClientState::New,
            incoming: Vec::with_capacity(1024),
            outgoing: Vec::with_capacity(1024),
            name: "Guest".to_string(),
            headers: headers.clone(),
            http_parser: Parser::request(HttpParser{
                current_key: None,
                headers: headers.clone(),
            }),
        }
    }

    /// (re)register interest in events, chosen based upon the current state
    pub fn register(&mut self, event_loop: &mut EventLoop<EventHandler>)
    {
        if self.state == ClientState::New {
            self.state = self.state.next();

            event_loop.register(&self.socket,
                                self.token,
                                EventSet::readable() | EventSet::hup(),
                                PollOpt::edge() | PollOpt::oneshot()).unwrap();

            return;
        }

        let event_set = EventSet::hup() | match self.state {
            ClientState::New => unreachable!("Handled above"),
            ClientState::HandshakeResponse => EventSet::writable(),
            ClientState::AwaitingHandshake => EventSet::readable(),
            ClientState::Running => EventSet::readable(),
            ClientState::RunningAndWriting => EventSet::writable() | EventSet::readable(),
        };

        event_loop.reregister(&self.socket,
                              self.token,
                              event_set,
                              PollOpt::edge() | PollOpt::oneshot()).unwrap();
    }

    pub fn handle_readable(&mut self)
    {
        match self.state {
            ClientState::New | ClientState::HandshakeResponse => {
                println!("Event out of step: Readable, but {:?}", self.state);
                self.sender.send(EventMessage::ReArm(self.token)).unwrap();
            },
            ClientState::AwaitingHandshake | ClientState::Running
                | ClientState::RunningAndWriting =>
            {
                let mut buf: [u8; 1024] = [0; 1024];
                loop {
                    match self.socket.read(&mut buf[..]) {
                        Err(e) => {
                            match e.kind() {
                                ErrorKind::WouldBlock => {
                                    // Remain in the current state, and re-arm for further reading
                                    self.sender.send(EventMessage::ReArm(self.token)).unwrap();
                                    break;
                                },
                                ErrorKind::Interrupted => {
                                    continue; // in case there is more to read
                                },
                                _other => {
                                    println!("Read error: {:?}",e);
                                    self.sender.send(EventMessage::Close(self.token)).unwrap();
                                    break;
                                }
                            }
                        },
                        Ok(0) => {
                            // We are done.  Re-Arm for the next event.
                            self.sender.send(EventMessage::ReArm(self.token)).unwrap();
                            break;
                        },
                        Ok(size) => {
                            //self.incoming.extend_from_slice(&buf[..size]);
                            match self.state {
                                ClientState::AwaitingHandshake => {

                                    self.http_parser.parse(&buf[..size]);

                                    if self.http_parser.is_upgrade() {
                                        self.process_upgrade();
                                        break;
                                    }
                                    continue; // in case there is more to read
                                },
                                _ => {
                                    self.process_incoming();
                                    continue; // in case there is more to read
                                }
                            }
                        }
                    }
                }
            },
        }
    }

    pub fn handle_writable(&mut self)
    {
        match self.state {
            ClientState::New | ClientState::AwaitingHandshake
                | ClientState::Running =>
            {
                println!("Event out of step: Writable, but {:?}", self.state);
                self.sender.send(EventMessage::ReArm(self.token)).unwrap();
            },
            ClientState::HandshakeResponse | ClientState::RunningAndWriting =>
            {
                loop {
                    match self.socket.write(&mut self.outgoing) {
                        Err(e) => {
                            match e.kind() {
                                ErrorKind::WouldBlock => {
                                    // Remain in the current state, and re-arm for further writing
                                    self.sender.send(EventMessage::ReArm(self.token)).unwrap();
                                    break;
                                },
                                ErrorKind::Interrupted => {
                                    continue; // in case there is more to write
                                },
                                _other => {
                                    println!("Write error: {:?}",e);
                                    self.sender.send(EventMessage::Close(self.token)).unwrap();
                                    break;
                                }
                            }
                        },
                        Ok(0) => {
                            // We are done.  Re-Arm for the next event.
                            self.sender.send(EventMessage::ReArm(self.token)).unwrap();
                            break;
                        },
                        Ok(size) if size == self.outgoing.len() => {
                            self.outgoing.truncate(0);
                            self.state = self.state.next();
                            self.sender.send(EventMessage::ReArm(self.token)).unwrap();
                            break;
                        },
                        Ok(size) => {
                            // Only partially written.  Shorten the buffer.
                            let remaining = self.outgoing.split_off(size);
                            self.outgoing = remaining;
                            continue; // in case we can write more
                        }
                    }
                }
            },
        }
    }

    pub fn handle_message(&mut self, message: String)
    {
        if self.state < ClientState::Running {
            // Do nothing if not yet setup
            return;
        }

        self.outgoing.extend_from_slice(message.as_bytes());
        self.outgoing.push(0x0A);
        self.state = ClientState::RunningAndWriting;
        self.sender.send(EventMessage::ReArm(self.token)).unwrap();
    }

    // Process incoming chat messages from the incoming buffer
    pub fn process_incoming(&mut self) {
        loop {
            if let Some(lf) = self.incoming.iter().position(|c| *c==0x0A) {
                let mut message = self.incoming.split_off(lf);
                message.remove(0); // Drop the leading LF
                ::std::mem::swap(&mut self.incoming, &mut message);
                let message = format!("{}: {}", self.name, String::from_utf8_lossy(&message));
                self.sender.send(EventMessage::Message(self.token, message)).unwrap();
                continue;
            }
            break;
        }
    }

    // Process upgrade for the incoming buffer
    pub fn process_upgrade(&mut self) {
        let headers = self.headers.lock().unwrap();

        let mut m = sha1::Sha1::new();
        let mut rbuf = [0u8; 20];

        m.update(&headers.get("Sec-WebSocket-Key").unwrap().as_bytes());
        m.update("258EAFA5-E914-47DA-95CA-C5AB0DC85B11".as_bytes());

        m.output(&mut rbuf);

        let response = fmt::format(format_args!("HTTP/1.1 101 Switching Protocols\r\n\
                                                 Connection: Upgrade\r\n\
                                                 Sec-WebSocket-Accept: {}\r\n\
                                                 Upgrade: websocket\r\n\r\n", rbuf.to_base64(STANDARD)));

        self.outgoing.extend_from_slice(response.as_bytes());

        self.state = ClientState::RunningAndWriting;

        self.sender.send(EventMessage::ReArm(self.token)).unwrap();
    }
}
