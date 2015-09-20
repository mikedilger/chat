
use mio::Token;

pub enum Message {
    /// Client has finished processing
    ClientDone(Token),

    /// Client has detected the remote has closed the connection (read 0 bytes)
    ClientHup(Token),
}
