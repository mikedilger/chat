
use mio::Token;

pub enum EventMessage {
    /// Client has finished processing, and needs to be re-armed
    ReArm(Token),

    /// Client has detected the remote has closed or errored, and should be
    /// shut down
    Close(Token),

    /// Client received this message
    Message(Token, String),
}
