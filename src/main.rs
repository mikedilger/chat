
extern crate mio;

use mio::{EventLoop,Handler};

struct EventHandler;

impl Handler for EventHandler {
    type Timeout = ();
    type Message = ();
}

fn main() {
    // Create the event loop
    let mut event_loop: EventLoop<EventHandler> = EventLoop::new().unwrap();

    // Create the event handler
    let mut event_handler = EventHandler;

    // Run the event loop
    event_loop.run(&mut event_handler).unwrap();
}
