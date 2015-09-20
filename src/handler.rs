
use mio::{Handler,EventLoop,Token,EventSet};
use server::{Server,LISTENER_FD};
use message::Message;

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
    type Message = Message;

    fn ready(&mut self, event_loop: &mut EventLoop<EventHandler>,
             token: Token, _events: EventSet)
    {
        match token {
            LISTENER_FD => {
                self.server.accept(event_loop);
            },
            // All other tokens must be clients
            client_token => {
                self.server.handle_client_read(event_loop, client_token);
            }
        }
    }
}
