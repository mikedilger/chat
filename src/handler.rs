
use mio::Handler;

pub struct EventHandler;

impl Handler for EventHandler {
    type Timeout = ();
    type Message = ();
}
