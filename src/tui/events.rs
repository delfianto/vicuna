use crate::api::types::{Model, ShowModelResponse};
use crate::db::repo::{Message, Session};
use crossterm::event::{KeyEvent, MouseEvent};

pub enum Event {
    Input(KeyEvent),
    Mouse(MouseEvent),
    Tick,
    ModelsFetched(Vec<Model>),
    SessionsFetched(Vec<Session>),
    MessagesLoaded(Vec<Message>),
    ModelInfoFetched(ShowModelResponse),
    TokenReceived(String),
    GenerationDone,
    Error(String),
}
