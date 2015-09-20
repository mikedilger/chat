
use std::net::SocketAddr;
use mio::tcp::TcpListener;

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
}
