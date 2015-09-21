
use std::net::SocketAddr;
use std::collections::HashMap;
use std::sync::{Arc,Mutex};
use threadpool::ThreadPool;
use mio::tcp::TcpListener;
use mio::{EventLoop,EventSet,PollOpt,Token};
use handler::EventHandler;
use client::Client;

pub const LISTENER_FD: Token = Token(0);

pub struct Server {
    listener: TcpListener,
    clients: HashMap<Token, Arc<Mutex<Client>>>,
    next_free_token: usize,
    pool: ThreadPool,
}

impl Server {
    pub fn new(address: &SocketAddr) -> Server
    {
        // Create the thread pool
        let pool = ThreadPool::new( ::num_cpus::get() );

        // See net2::TcpBuilder if finer-grained control is required
        // (e.g. ipv6 or setting the listen backlog)
        let listener = TcpListener::bind(address).unwrap();
        Server {
            listener: listener,
            clients: HashMap::new(),
            next_free_token: 1,
            pool: pool,
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

        // Build a channel to the event loop for use from the client thread
        let sender = event_loop.channel();

        // Build a new client
        let client = Arc::new(Mutex::new(Client::new(client_socket, new_token, sender)));

        // And remember that this token maps to this client
        self.clients.insert(new_token, client.clone());

        // Register the client's readable events.  This must be after inserting
        // into the map, to be sure the server is actually ready
        client.lock().unwrap().register(event_loop);
    }

    pub fn handle_client_read(&mut self, _event_loop: &mut EventLoop<EventHandler>,
                              client_token: Token)
    {
        let client = match self.clients.get_mut(&client_token) {
            None => return,
            Some(client) => client.clone(),
        };

        self.pool.execute(move || {
            client.lock().unwrap().handle_readable();
        });
    }

    pub fn handle_client_write(&mut self, _event_loop: &mut EventLoop<EventHandler>,
                               client_token: Token)
    {
        let client = match self.clients.get_mut(&client_token) {
            None => return,
            Some(client) => client.clone(),
        };

        self.pool.execute(move || {
            client.lock().unwrap().handle_writable();
        });
    }

    pub fn handle_client_rearm(&mut self, event_loop: &mut EventLoop<EventHandler>,
                              client_token: Token)
    {
        let client = match self.clients.get_mut(&client_token) {
            None => return,
            Some(client) => client.clone(),
        };

        // Re-register for client readable events
        client.lock().unwrap().register(event_loop);
    }

    pub fn handle_client_close(&mut self, client_token: Token) {
        let _ = self.clients.remove(&client_token);
    }

    pub fn handle_client_message(&mut self, client_token: Token, message: String) {
        for (_,client) in self.clients.iter() {
            // FIXME: send clients an arc around a singular message in memory,
            //  dont clone the message for every client

            let mut client = client.lock().unwrap();
            if client.token == client_token {
                continue;
            }
            client.handle_message(message.clone());
        }
    }
}
