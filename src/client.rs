
use std::io::Read;
use mio::tcp::TcpStream;
use mio::{Token,EventLoop,EventSet,PollOpt,Sender};
use handler::EventHandler;
use message::Message;

pub struct Client {
    socket: TcpStream,
    token: Token,
    registered: bool,
    sender: Sender<Message>,
}

impl Client {
    pub fn new(socket: TcpStream, token: Token, sender: Sender<Message>) -> Client
    {
        Client {
            socket: socket,
            token: token,
            registered: false,
            sender: sender,
        }
    }

    pub fn register(&mut self, event_loop: &mut EventLoop<EventHandler>)
    {
        if ! self.registered {
            event_loop.register(&self.socket,
                                self.token,
                                EventSet::readable(),
                                PollOpt::edge() | PollOpt::oneshot()).unwrap();
            self.registered = true;
        } else {
            event_loop.reregister(&self.socket,
                                  self.token,
                                  EventSet::readable(),
                                  PollOpt::edge() | PollOpt::oneshot()).unwrap();
        }
    }

    pub fn handle_readable(&mut self)
    {
        // Read and echo to the console
        let mut buf: [u8; 1024] = [0; 1024];
        match self.socket.read(&mut buf[..]) {
            Err(e) => {
                println!("Read error: {:?}",e);
                // Don't re-register --- FIXME, this just HANGS this client.
            },
            Ok(_size) => {
                let output = String::from_utf8_lossy(&buf);
                print!("{}", output);
                self.sender.send(Message::ClientDone(self.token)).unwrap();
            }
        }
    }
}
