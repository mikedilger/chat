
use mio::{Handler,EventLoop,Token,EventSet};
use server::{Server,LISTENER_FD};
use event_message::EventMessage;

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
    type Message = EventMessage;

    fn ready(&mut self, event_loop: &mut EventLoop<EventHandler>,
             token: Token, events: EventSet)
    {
        match token {
            LISTENER_FD => {
                self.server.accept(event_loop);
            },
            // All other tokens must be clients
            client_token => {
                if events.is_hup() {
                    self.server.handle_client_close(client_token);
                }
                if events.is_writable() {
                    self.server.handle_client_write(event_loop, client_token);
                }
                else if events.is_readable() {
                    self.server.handle_client_read(event_loop, client_token);
                }
            }
        }
    }

    fn notify(&mut self, event_loop: &mut EventLoop<EventHandler>,
              message: EventMessage)
    {
        match message {
            EventMessage::ReArm(client_token) => {
                self.server.handle_client_rearm(event_loop, client_token);
            },
            EventMessage::Close(client_token) => {
                self.server.handle_client_close(client_token);
            },
            EventMessage::Message(message) => {
                self.server.handle_client_message(message);
            }
        }
    }
}
