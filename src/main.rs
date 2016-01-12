
extern crate mio;
extern crate threadpool;
extern crate num_cpus;
extern crate rustc_serialize;
extern crate http_muncher;
extern crate sha1;
extern crate byteorder;

mod handler;
mod server;
mod client;
mod event_message;
mod http_parser;
mod websocket_frame;

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
