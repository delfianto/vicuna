use crate::api::types::Model;

pub enum Event {
    Input(crossterm::event::KeyEvent),
    Tick,
    ModelsFetched(Vec<Model>),
    TokenReceived(String),
    GenerationDone,
    Error(String),
    SessionsFetched(Vec<(String, String, String, String)>), // id, title, model, date
    MessagesLoaded(Vec<(String, String)>), // role, content
}
