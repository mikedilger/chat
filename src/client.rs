
use mio::tcp::TcpStream;
use mio::{Token,EventLoop,EventSet,PollOpt};
use handler::EventHandler;

pub struct Client {
    socket: TcpStream,
    token: Token,
    registered: bool,
}

impl Client {
    pub fn new(socket: TcpStream, token: Token) -> Client
    {
        Client {
            socket: socket,
            token: token,
            registered: false,
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
        // TBD: do something
    }
}
