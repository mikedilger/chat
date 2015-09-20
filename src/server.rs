
use std::net::SocketAddr;
use mio::tcp::TcpListener;
use mio::{EventLoop,EventSet,PollOpt,Token};
use handler::EventHandler;

const LISTENER_FD: Token = Token(0);

pub struct Server {
    listener: TcpListener,
}

impl Server {
    pub fn new(address: &SocketAddr) -> Server
    {
        // See net2::TcpBuilder if finer-grained control is required
        // (e.g. ipv6 or setting the listen backlog)
        let listener = TcpListener::bind(address).unwrap();
        Server {
            listener: listener,
        }
    }

    pub fn register(&mut self, event_loop: &mut EventLoop<EventHandler>) {
        event_loop.register(&self.listener,
                            LISTENER_FD,
                            EventSet::readable(),
                            PollOpt::edge()).unwrap();
    }
}
