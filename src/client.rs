
use std::io::Read;
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
}

impl Client {
    pub fn new(socket: TcpStream, token: Token, sender: Sender<EventMessage>) -> Client
    {
        Client {
            socket: socket,
            token: token,
            sender: sender,
            state: ClientState::New,
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
                        // Don't re-register --- FIXME, this just HANGS this client.
                    },
                    Ok(0) => {
                        // We read zero bytes.  This means the peer has closed the connection.
                        self.sender.send(EventMessage::Close(self.token)).unwrap();
                    },
                    Ok(_size) => {
                        let output = String::from_utf8_lossy(&buf);
                        print!("{}", output);
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
                // FIXME: currently we don't write anything, and as we stay in the same
                // state, this will trigger over and over every tick.
                self.sender.send(EventMessage::ReArm(self.token)).unwrap();
            },
        }
    }
}
