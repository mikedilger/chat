
#![feature(vec_push_all)]

extern crate mio;
extern crate threadpool;
extern crate num_cpus;
extern crate http_muncher; // FIXME: try to move to httparse, it should be faster
extern crate sha1; // FIXME: why not rust-crypto here?
extern crate rustc_serialize;
extern crate websocket;

mod handler;
mod server;
mod client;
mod event_message;
mod http;

use std::net::SocketAddr;
use mio::EventLoop;
use handler::EventHandler;
use server::Server;

fn main() {
    // Create the server
    let server = {
        let address = "192.168.66.62:10000".parse::<SocketAddr>().unwrap();
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
