
extern crate mio;

mod handler;
mod server;

use std::net::SocketAddr;
use mio::EventLoop;
use handler::EventHandler;
use server::Server;

fn main() {
    // Create the server
    let server = {
        let address = "127.0.0.1:10000".parse::<SocketAddr>().unwrap();
        Server::new(&address)
    };

    // Create the event handler
    let mut event_handler = EventHandler::new(server);

    // Create the event loop
    let mut event_loop: EventLoop<EventHandler> = EventLoop::new().unwrap();

    // Register the server in the event loop
    event_handler.register_server(&mut event_loop);

    // Run the event loop
    event_loop.run(&mut event_handler).unwrap();
}
