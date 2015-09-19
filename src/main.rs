
extern crate mio;

mod handler;

use mio::EventLoop;
use handler::EventHandler;

fn main() {
    // Create the event loop
    let mut event_loop: EventLoop<EventHandler> = EventLoop::new().unwrap();

    // Create the event handler
    let mut event_handler = EventHandler;

    // Run the event loop
    event_loop.run(&mut event_handler).unwrap();
}
