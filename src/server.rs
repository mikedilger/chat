
use std::net::SocketAddr;
use std::collections::HashMap;
use mio::tcp::{TcpListener,TcpStream};
use mio::{EventLoop,EventSet,PollOpt,Token};
use handler::EventHandler;

pub const LISTENER_FD: Token = Token(0);

pub struct Server {
    listener: TcpListener,
    clients: HashMap<Token, TcpStream>,
    next_free_token: usize,
}

impl Server {
    pub fn new(address: &SocketAddr) -> Server
    {
        // See net2::TcpBuilder if finer-grained control is required
        // (e.g. ipv6 or setting the listen backlog)
        let listener = TcpListener::bind(address).unwrap();
        Server {
            listener: listener,
            clients: HashMap::new(),
            next_free_token: 1,
        }
    }

    pub fn register(&mut self, event_loop: &mut EventLoop<EventHandler>) {
        event_loop.register(&self.listener,
                            LISTENER_FD,
                            EventSet::readable(),
                            PollOpt::edge()).unwrap();
    }

    pub fn accept(&mut self, event_loop: &mut EventLoop<EventHandler>) {

        // Accept the client
        let client_socket = match self.listener.accept() {
            Err(e) => {
                println!("Accept error: {}", e);
                return;
            },
            Ok(None) => unreachable!("Accept has returned 'None'"),
            Ok(Some((socket, _peer_address))) => socket,
        };

        // Allocate a token
        let new_token = Token(self.next_free_token);
        self.next_free_token += 1;

        // And remember that this token maps to this client
        self.clients.insert(new_token, client_socket);

        // Register for readable events on the client socket
        event_loop.register(&self.clients[&new_token],
                            new_token,
                            EventSet::readable(),
                            PollOpt::edge() | PollOpt::oneshot()).unwrap();
    }

    pub fn handle_client_read(&mut self, event_loop: &mut EventLoop<EventHandler>,
                              client_token: Token)
    {
        let client = match self.clients.get(&client_token) {
            None => return,
            Some(client) => client,
        };

        // TBD: do something

        // Re-register for readable events on the client socket
        event_loop.reregister(client,
                              client_token,
                              EventSet::readable(),
                              PollOpt::edge() | PollOpt::oneshot()).unwrap();
    }
}
