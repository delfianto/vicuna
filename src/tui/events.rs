use crate::api::types::{Model, ShowModelResponse};
use crate::db::repo::{Message, Session};
use crossterm::event::KeyEvent;

pub enum Event {
    Input(KeyEvent),
    Tick,
    ModelsFetched(Vec<Model>),
    SessionsFetched(Vec<Session>),
    MessagesLoaded(Vec<Message>),
    ModelInfoFetched(ShowModelResponse),
    TokenReceived(String),
    GenerationDone,
    Error(String),
    ImageLoaded(image::DynamicImage),
    ImageInitialized(Box<dyn ratatui_image::protocol::Protocol>),
}
