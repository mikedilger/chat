
use mio::{Handler,EventLoop};
use server::Server;

pub struct EventHandler {
    server: Server,
}

impl Handler for EventHandler {
    type Timeout = ();
    type Message = ();
}

impl EventHandler {
    pub fn new(server: Server) -> EventHandler {
        EventHandler {
            server: server
        }
    }

    pub fn register_server(&mut self, event_loop: &mut EventLoop<EventHandler>) {
        self.server.register(event_loop);
    }
}
