use crate::api::types::Model;
use crate::config::Config;
use crate::tui::components::toast::Toast;
use crossterm::event::{KeyCode, KeyEvent};

#[derive(Clone, Debug)]
pub enum Action {
    FetchModels,
    Quit,
    DeleteModel(String),
    PullModel(String),
    Generate(String, String),
}

#[derive(Clone, Debug)]
pub struct Message {
    pub role: String,
    pub content: String,
}

#[derive(PartialEq)]
pub enum CurrentTab {
    Models,
    Chat,
}

pub struct App {
    #[allow(dead_code)]
    pub config: Config,
    pub current_tab: CurrentTab,
    pub should_quit: bool,

    pub models: Vec<Model>,
    pub selected_model_index: usize,

    pub input: tui_textarea::TextArea<'static>,
    pub messages: Vec<Message>,
    pub is_generating: bool,
    pub toasts: Vec<Toast>,
}

impl App {
    pub fn new(config: Config) -> Self {
        let mut input = tui_textarea::TextArea::default();
        input.set_placeholder_text("Type a message...");
        input.set_block(
            ratatui::widgets::Block::default()
                .borders(ratatui::widgets::Borders::ALL)
                .title("Input"),
        );

        Self {
            config,
            current_tab: CurrentTab::Models,
            should_quit: false,
            models: Vec::new(),
            selected_model_index: 0,
            input,
            messages: Vec::new(),
            is_generating: false,
            toasts: Vec::new(),
        }
    }

    pub fn on_tick(&mut self) {
        self.toasts.retain_mut(|t| {
            if t.duration > 0 {
                t.duration -= 1;
                true
            } else {
                false
            }
        });
    }

    pub fn show_error(&mut self, msg: &str) {
        self.toasts.push(Toast {
            message: msg.to_string(),
            duration: 20,
            color: ratatui::style::Color::Red,
        });
    }

    pub fn on_key(&mut self, key: KeyEvent) -> Option<Action> {
        match key.code {
            KeyCode::Char('q') => {
                self.should_quit = true;
                Some(Action::Quit)
            }
            KeyCode::Tab => {
                self.current_tab = match self.current_tab {
                    CurrentTab::Models => CurrentTab::Chat,
                    CurrentTab::Chat => CurrentTab::Models,
                };
                None
            }
            _ => match self.current_tab {
                CurrentTab::Models => self.on_key_models(key),
                CurrentTab::Chat => self.on_key_chat(key),
            },
        }
    }

    fn on_key_chat(&mut self, key: KeyEvent) -> Option<Action> {
        if self.is_generating {
            return None;
        }

        match key.code {
            KeyCode::Enter => {
                let text = self.input.lines().join("\n");
                if !text.trim().is_empty() {
                    let prompt = text.clone();
                    self.messages.push(Message {
                        role: "user".into(),
                        content: text,
                    });

                    self.input = tui_textarea::TextArea::default();
                    self.input.set_placeholder_text("Type a message...");
                    self.input.set_block(
                        ratatui::widgets::Block::default()
                            .borders(ratatui::widgets::Borders::ALL)
                            .title("Input"),
                    );

                    self.is_generating = true;

                    let model_name = self
                        .models
                        .get(self.selected_model_index)
                        .map(|m| m.name.clone())
                        .unwrap_or_else(|| "llama2:latest".to_string());

                    return Some(Action::Generate(prompt, model_name));
                }
            }
            _ => {
                self.input.input(key);
            }
        }
        None
    }

    fn on_key_models(&mut self, key: KeyEvent) -> Option<Action> {
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                if !self.models.is_empty() && self.selected_model_index < self.models.len() - 1 {
                    self.selected_model_index += 1;
                }

                None
            }

            KeyCode::Char('k') | KeyCode::Up => {
                if self.selected_model_index > 0 {
                    self.selected_model_index -= 1;
                }

                None
            }

            KeyCode::Char('d') => {
                if let Some(model) = self.models.get(self.selected_model_index) {
                    Some(Action::DeleteModel(model.name.clone()))
                } else {
                    None
                }
            }

            KeyCode::Char('p') => Some(Action::PullModel("llama2:latest".to_string())),

            _ => None,
        }
    }
}
