
use mio::Token;

pub enum Message {
    ClientDone(Token),
}
