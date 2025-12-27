use crate::api::types::{Model, ShowModelResponse};
use crossterm::event::KeyEvent;

pub enum Event {
    Input(KeyEvent),
    Tick,
    ModelsFetched(Vec<Model>),
    SessionsFetched(Vec<(String, String, String, String)>),
    MessagesLoaded(Vec<(String, String)>),
    ModelInfoFetched(ShowModelResponse),
    TokenReceived(String),
    GenerationDone,
    Error(String),
    ImageLoaded(image::DynamicImage),
    ImageInitialized(Box<dyn ratatui_image::protocol::Protocol>),
}
