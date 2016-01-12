
use mio::Token;

pub enum EventMessage {
    /// Client has finished processing, and needs to be re-armed
    ReArm(Token),

    /// Client has detected the remote has closed or errored, and should be
    /// shut down
    Close(Token),

    Ping(Token, Vec<u8>),
    Pong(Token, Vec<u8>),

    /// Client received this String
    TextMessage(Token, String),

    /// Client received these Bytes
    BinaryMessage(Token, Vec<u8>),
}
