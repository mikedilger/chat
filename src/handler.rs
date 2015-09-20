
use mio::{Handler,EventLoop,Token,EventSet};
use server::{Server,LISTENER_FD};

pub struct EventHandler {
    server: Server,
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

impl Handler for EventHandler {
    type Timeout = ();
    type Message = ();

    fn ready(&mut self, event_loop: &mut EventLoop<EventHandler>,
             token: Token, _events: EventSet)
    {
        match token {
            LISTENER_FD => {
                self.server.accept(event_loop);
            },
            _ => unreachable!("Ready event for unknown token"),
        }
    }
}
