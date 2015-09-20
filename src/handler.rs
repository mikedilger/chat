
use mio::Handler;
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
}
