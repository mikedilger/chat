
use mio::Token;
use websocket_frame::WebSocketFrame;

pub enum EventMessage {
    /// Client has finished processing, and needs to be re-armed
    ReArm(Token),

    /// Client has detected the remote has closed or errored, and should be
    /// shut down
    Close(Token),

    /// Client received ping frame
    Ping(Token, WebSocketFrame),

    /// Client received pong frame
    Pong(Token, Vec<u8>),

    /// Client received this String
    TextFrame(Token, String),

    /// Client received these Bytes
    BinaryFrame(Token, Vec<u8>),
}
