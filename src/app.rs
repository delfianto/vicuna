use crate::api::types::{Model, ModelName, SessionId};
use crate::config::Config;
use crate::tui::components::popup::Popup;
use crate::tui::components::toast::Toast;
use crossterm::event::{KeyCode, KeyEvent};
use std::cell::Cell;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    FetchModels,
    Quit,
    DeleteModel(ModelName),
    PullModel(ModelName),
    ShowModelInfo(ModelName),
    Generate(String, ModelName),
    FetchSessions,
    LoadSession(SessionId),
    DeleteSession(SessionId),
    CreateSession(SessionId, String, ModelName),
    SaveMessage(SessionId, String, String),
    InitImage(u16, u16),
}

#[derive(PartialEq, Debug)]
pub enum CurrentTab {
    Models,
    Chat,
}

#[derive(PartialEq, Debug)]
pub enum ChatFocus {
    Input,
    Sessions,
}

#[derive(PartialEq, Debug)]
pub enum ModelsFocus {
    List,
    Info,
}

#[derive(PartialEq, Debug, Clone, Copy)]
pub enum SortColumn {
    Name,
    Size,
    Modified,
}

pub struct App {
    pub _config: Config,
    pub current_tab: CurrentTab,
    pub chat_focus: ChatFocus,
    pub models_focus: ModelsFocus,
    pub should_quit: bool,

    pub models: Vec<Model>,
    pub selected_model_index: usize,
    pub sort_column: SortColumn,

    pub show_info: bool,
    pub model_info: Option<crate::api::types::ShowModelResponse>,
    pub info_scroll: Cell<u16>,

    pub input: tui_textarea::TextArea<'static>,
    pub messages: Vec<crate::db::repo::Message>,
    pub is_generating: bool,
    pub toasts: Vec<Toast>,

    pub sessions: Vec<crate::db::repo::Session>,
    pub selected_session_index: usize,
    pub current_session_id: Option<String>,
    pub chat_scroll: u16,

    pub show_popup: bool,
    pub popup: Popup<'static>,
    pub logo: Option<Box<dyn ratatui_image::protocol::Protocol>>,
}

impl App {
    pub fn new(config: Config) -> Self {
        let mut input = tui_textarea::TextArea::default();
        input.set_placeholder_text("Type a message...");
        input.set_cursor_line_style(ratatui::style::Style::default());
        input.set_block(
            ratatui::widgets::Block::default()
                .borders(ratatui::widgets::Borders::ALL)
                .title("Input"),
        );

        let popup = Popup::new("Pull Model".to_string());

        Self {
            _config: config,
            current_tab: CurrentTab::Models,
            chat_focus: ChatFocus::Input,
            models_focus: ModelsFocus::List,
            should_quit: false,
            models: Vec::new(),
            selected_model_index: 0,
            sort_column: SortColumn::Name,
            show_info: false,
            model_info: None,
            info_scroll: Cell::new(0),
            input,
            messages: Vec::new(),
            is_generating: false,
            toasts: Vec::new(),
            sessions: Vec::new(),
            selected_session_index: 0,
            current_session_id: None,
            chat_scroll: 0,
            show_popup: false,
            popup,
            logo: None,
        }
    }

    pub fn sort_models(&mut self) {
        match self.sort_column {
            SortColumn::Name => {
                self.models.sort_by(|a, b| {
                    let name_a = crate::api::modelfile::sanitize_model_name(&a.name).to_lowercase();
                    let name_b = crate::api::modelfile::sanitize_model_name(&b.name).to_lowercase();
                    name_a.cmp(&name_b)
                });
            }
            SortColumn::Size => {
                self.models.sort_by_key(|a| a.size);
            }
            SortColumn::Modified => {
                self.models
                    .sort_by(|a, b| a.modified_at.cmp(&b.modified_at));
            }
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

    pub fn on_key(&mut self, key: KeyEvent) -> Vec<Action> {
        if self.show_popup {
            return self.on_key_popup(key);
        }

        match key.code {
            KeyCode::Char('q')
                if key
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::CONTROL) =>
            {
                self.should_quit = true;
                vec![Action::Quit]
            }
            KeyCode::Char('q') => match self.current_tab {
                CurrentTab::Models => {
                    if self.show_info {
                        self.on_key_models(key)
                    } else {
                        self.should_quit = true;
                        vec![Action::Quit]
                    }
                }
                CurrentTab::Chat => self.on_key_chat(key),
            },
            KeyCode::Char('a')
                if key
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::CONTROL) =>
            {
                self.current_tab = CurrentTab::Models;
                vec![]
            }
            KeyCode::Char('d')
                if key
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::CONTROL) =>
            {
                self.current_tab = CurrentTab::Chat;
                vec![]
            }
            KeyCode::Tab => {
                match self.current_tab {
                    CurrentTab::Models => {
                        if self.show_info {
                            self.models_focus = match self.models_focus {
                                ModelsFocus::List => ModelsFocus::Info,
                                ModelsFocus::Info => ModelsFocus::List,
                            };
                        }
                    }
                    CurrentTab::Chat => {
                        self.chat_focus = match self.chat_focus {
                            ChatFocus::Input => ChatFocus::Sessions,
                            ChatFocus::Sessions => ChatFocus::Input,
                        };
                    }
                }
                vec![]
            }
            _ => match self.current_tab {
                CurrentTab::Models => self.on_key_models(key),
                CurrentTab::Chat => self.on_key_chat(key),
            },
        }
    }

    fn on_key_popup(&mut self, key: KeyEvent) -> Vec<Action> {
        match key.code {
            KeyCode::Esc => {
                self.show_popup = false;
                vec![]
            }
            KeyCode::Enter => {
                let model_name = self.popup.textarea.lines().join("").trim().to_string();
                if !model_name.is_empty() {
                    self.show_popup = false;
                    self.popup.textarea = tui_textarea::TextArea::default();
                    self.popup.textarea.set_block(
                        ratatui::widgets::Block::default()
                            .borders(ratatui::widgets::Borders::ALL)
                            .title("Input"),
                    );
                    vec![Action::PullModel(ModelName(model_name))]
                } else {
                    vec![]
                }
            }
            _ => {
                self.popup.textarea.input(key);
                vec![]
            }
        }
    }

    fn on_key_chat(&mut self, key: KeyEvent) -> Vec<Action> {
        match key.code {
            KeyCode::PageUp => {
                self.chat_scroll = self.chat_scroll.saturating_sub(15);
                return vec![];
            }
            KeyCode::PageDown => {
                self.chat_scroll = self.chat_scroll.saturating_add(15);
                return vec![];
            }
            KeyCode::Up if self.chat_focus != ChatFocus::Sessions => {
                self.chat_scroll = self.chat_scroll.saturating_sub(3);
                return vec![];
            }
            KeyCode::Down if self.chat_focus != ChatFocus::Sessions => {
                self.chat_scroll = self.chat_scroll.saturating_add(3);
                return vec![];
            }
            KeyCode::Left
                if key
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::CONTROL) =>
            {
                self.chat_focus = ChatFocus::Sessions;
                return vec![];
            }
            KeyCode::Right
                if key
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::CONTROL) =>
            {
                self.chat_focus = ChatFocus::Input;
                return vec![];
            }
            _ => {}
        }

        if self.is_generating {
            return vec![];
        }

        match self.chat_focus {
            ChatFocus::Sessions => match key.code {
                KeyCode::Char('j') | KeyCode::Down => {
                    if !self.sessions.is_empty()
                        && self.selected_session_index < self.sessions.len() - 1
                    {
                        self.selected_session_index += 1;
                    }
                    vec![]
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    if self.selected_session_index > 0 {
                        self.selected_session_index -= 1;
                    }
                    vec![]
                }
                KeyCode::Enter => {
                    if let Some(session) = self.sessions.get(self.selected_session_index) {
                        let id = session.id.0.clone();
                        let model_name = session.model.0.clone();

                        if let Some(idx) = self.models.iter().position(|m| m.name == model_name) {
                            self.selected_model_index = idx;
                        }

                        self.current_session_id = Some(id.clone());
                        self.chat_focus = ChatFocus::Input;
                        vec![Action::LoadSession(SessionId(id))]
                    } else {
                        vec![]
                    }
                }
                KeyCode::Char('n')
                    if key
                        .modifiers
                        .contains(crossterm::event::KeyModifiers::CONTROL) =>
                {
                    self.messages.clear();
                    self.current_session_id = None;
                    self.chat_scroll = 0;
                    self.chat_focus = ChatFocus::Input;
                    vec![]
                }
                KeyCode::Char('d') => {
                    if let Some(session) = self.sessions.get(self.selected_session_index) {
                        let id = session.id.clone();
                        if let Some(current) = &self.current_session_id
                            && current == &id.0
                        {
                            self.messages.clear();
                            self.current_session_id = None;
                        }
                        vec![Action::DeleteSession(id)]
                    } else {
                        vec![]
                    }
                }
                _ => vec![],
            },
            ChatFocus::Input => match key.code {
                KeyCode::Char('n')
                    if key
                        .modifiers
                        .contains(crossterm::event::KeyModifiers::CONTROL) =>
                {
                    self.messages.clear();
                    self.current_session_id = None;
                    self.chat_scroll = 0;
                    vec![]
                }
                KeyCode::Enter => {
                    let text = self.input.lines().join("\n");
                    if !text.trim().is_empty() {
                        let prompt = text.clone();
                        self.messages.push(crate::db::repo::Message {
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

                        let mut actions = Vec::new();

                        let session_id = if let Some(id) = &self.current_session_id {
                            id.clone()
                        } else {
                            let new_id = Uuid::new_v4().to_string();
                            let title = prompt.chars().take(20).collect::<String>();
                            self.current_session_id = Some(new_id.clone());
                            actions.push(Action::CreateSession(
                                SessionId(new_id.clone()),
                                title,
                                ModelName(model_name.clone()),
                            ));
                            new_id
                        };

                        actions.push(Action::SaveMessage(
                            SessionId(session_id),
                            "user".to_string(),
                            prompt.clone(),
                        ));

                        actions.push(Action::Generate(prompt, ModelName(model_name)));

                        return actions;
                    }
                    vec![]
                }
                _ => {
                    self.input.input(key);
                    vec![]
                }
            },
        }
    }

    fn on_key_models(&mut self, key: KeyEvent) -> Vec<Action> {
        if self.show_info {
            match key.code {
                KeyCode::Char('i') | KeyCode::Esc => {
                    self.show_info = false;
                    self.models_focus = ModelsFocus::List;
                    return vec![];
                }
                KeyCode::PageUp => {
                    self.info_scroll
                        .set(self.info_scroll.get().saturating_sub(15));
                    return vec![];
                }
                KeyCode::PageDown => {
                    self.info_scroll
                        .set(self.info_scroll.get().saturating_add(15));
                    return vec![];
                }
                KeyCode::Up if self.models_focus == ModelsFocus::Info => {
                    self.info_scroll
                        .set(self.info_scroll.get().saturating_sub(3));
                    return vec![];
                }
                KeyCode::Down if self.models_focus == ModelsFocus::Info => {
                    self.info_scroll
                        .set(self.info_scroll.get().saturating_add(3));
                    return vec![];
                }
                _ => {}
            }
        }

        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                if !self.models.is_empty() && self.selected_model_index < self.models.len() - 1 {
                    self.selected_model_index += 1;
                    if self.show_info
                        && let Some(model) = self.models.get(self.selected_model_index)
                    {
                        return vec![Action::ShowModelInfo(ModelName(model.name.clone()))];
                    }
                }
                vec![]
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.selected_model_index > 0 {
                    self.selected_model_index -= 1;
                    if self.show_info
                        && let Some(model) = self.models.get(self.selected_model_index)
                    {
                        return vec![Action::ShowModelInfo(ModelName(model.name.clone()))];
                    }
                }
                vec![]
            }
            KeyCode::Char('d') => {
                if let Some(model) = self.models.get(self.selected_model_index) {
                    vec![Action::DeleteModel(ModelName(model.name.clone()))]
                } else {
                    vec![]
                }
            }
            KeyCode::Char('p') => {
                self.show_popup = true;
                vec![]
            }
            KeyCode::Char('i') => {
                if let Some(model) = self.models.get(self.selected_model_index) {
                    self.show_info = true;
                    self.info_scroll.set(0);
                    self.models_focus = ModelsFocus::Info;
                    vec![Action::ShowModelInfo(ModelName(model.name.clone()))]
                } else {
                    vec![]
                }
            }
            KeyCode::Char('s') => {
                self.sort_column = match self.sort_column {
                    SortColumn::Name => SortColumn::Size,
                    SortColumn::Size => SortColumn::Modified,
                    SortColumn::Modified => SortColumn::Name,
                };
                self.sort_models();
                vec![]
            }
            KeyCode::Enter => {
                self.current_tab = CurrentTab::Chat;
                self.messages.clear();
                self.current_session_id = None;
                self.chat_scroll = 0;
                self.models_focus = ModelsFocus::List;
                vec![]
            }
            _ => vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

    fn mock_key(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        }
    }

    fn mock_key_ctrl(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::CONTROL,
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        }
    }

    #[test]
    fn test_app_tab_switching() {
        let config = Config {
            _config_dir: "/tmp".into(),
            _data_dir: "/tmp".into(),
            log_dir: "/tmp".into(),
            db_path: "/tmp/test.db".into(),
            ollama_url: "http://localhost:11434".into(),
        };
        let mut app = App::new(config);
        assert_eq!(app.current_tab, CurrentTab::Models);

        app.on_key(mock_key_ctrl(KeyCode::Char('d')));
        assert_eq!(app.current_tab, CurrentTab::Chat);

        app.on_key(mock_key_ctrl(KeyCode::Char('a')));
        assert_eq!(app.current_tab, CurrentTab::Models);
    }

    #[test]
    fn test_app_pane_switching() {
        let config = Config {
            _config_dir: "/tmp".into(),
            _data_dir: "/tmp".into(),
            log_dir: "/tmp".into(),
            db_path: "/tmp/test.db".into(),
            ollama_url: "http://localhost:11434".into(),
        };
        let mut app = App::new(config);

        app.current_tab = CurrentTab::Models;
        app.show_info = false;
        app.on_key(mock_key(KeyCode::Tab));
        assert_eq!(app.models_focus, ModelsFocus::List);

        app.show_info = true;
        app.models_focus = ModelsFocus::List;
        app.on_key(mock_key(KeyCode::Tab));
        assert_eq!(app.models_focus, ModelsFocus::Info);
        app.on_key(mock_key(KeyCode::Tab));
        assert_eq!(app.models_focus, ModelsFocus::List);

        app.show_info = true;
        app.models_focus = ModelsFocus::Info;
        app.on_key(mock_key(KeyCode::Enter));
        assert_eq!(app.current_tab, CurrentTab::Chat);
        assert_eq!(app.models_focus, ModelsFocus::List);

        app.current_tab = CurrentTab::Chat;
        app.chat_focus = ChatFocus::Input;
        app.on_key(mock_key(KeyCode::Tab));
        assert_eq!(app.chat_focus, ChatFocus::Sessions);
        app.on_key(mock_key(KeyCode::Tab));
        assert_eq!(app.chat_focus, ChatFocus::Input);
    }

    #[test]
    fn test_chat_focus_switching() {
        let config = Config {
            _config_dir: "/tmp".into(),
            _data_dir: "/tmp".into(),
            log_dir: "/tmp".into(),
            db_path: "/tmp/test.db".into(),
            ollama_url: "http://localhost:11434".into(),
        };
        let mut app = App::new(config);
        app.current_tab = CurrentTab::Chat;
        assert_eq!(app.chat_focus, ChatFocus::Input);

        app.on_key(mock_key_ctrl(KeyCode::Left));
        assert_eq!(app.chat_focus, ChatFocus::Sessions);

        app.on_key(mock_key_ctrl(KeyCode::Right));
        assert_eq!(app.chat_focus, ChatFocus::Input);
    }

    #[test]
    fn test_quit_handling() {
        let config = Config {
            _config_dir: "/tmp".into(),
            _data_dir: "/tmp".into(),
            log_dir: "/tmp".into(),
            db_path: "/tmp/test.db".into(),
            ollama_url: "http://localhost:11434".into(),
        };
        let mut app = App::new(config);
        assert!(!app.should_quit);

        app.on_key(mock_key_ctrl(KeyCode::Char('q')));
        assert!(app.should_quit);
    }

    #[test]
    fn test_sanitized_name_sorting() {
        let config = Config {
            _config_dir: "/tmp".into(),
            _data_dir: "/tmp".into(),
            log_dir: "/tmp".into(),
            db_path: "/tmp/test.db".into(),
            ollama_url: "http://localhost:11434".into(),
        };
        let mut app = App::new(config);

        use crate::api::types::Model;
        app.models = vec![
            Model {
                name: "z-org/alpha:latest".into(),
                modified_at: "".into(),
                size: 0,
                digest: "".into(),
                details: None,
            },
            Model {
                name: "a-org/zebra:latest".into(),
                modified_at: "".into(),
                size: 0,
                digest: "".into(),
                details: None,
            },
        ];

        app.sort_column = SortColumn::Name;
        app.sort_models();

        assert_eq!(app.models[0].name, "z-org/alpha:latest");
        assert_eq!(app.models[1].name, "a-org/zebra:latest");
    }
}
