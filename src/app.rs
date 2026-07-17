use crate::api::types::{Model, ModelName, SessionId};
use crate::config::Config;
use crate::tui::components::confirm::{ConfirmAction, ConfirmPrompt};
use crate::tui::components::model_picker::ModelPicker;
use crate::tui::components::popup::Popup;
use crate::tui::components::toast::Toast;
use crossterm::event::{
    KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::layout::Rect;
use uuid::Uuid;

/// Last-frame hit regions for mouse (filled during draw).
#[derive(Debug, Clone, Default)]
pub struct UiHits {
    pub sessions: Option<Rect>,
    pub messages: Option<Rect>,
    pub composer: Option<Rect>,
    pub models_list: Option<Rect>,
    pub models_info: Option<Rect>,
}

impl UiHits {
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    fn contains(r: Option<Rect>, col: u16, row: u16) -> bool {
        r.is_some_and(|r| {
            col >= r.x && col < r.x.saturating_add(r.width) && row >= r.y && row < r.y.saturating_add(r.height)
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    FetchModels,
    Quit,
    DeleteModel(ModelName),
    PullModel(ModelName),
    ShowModelInfo(ModelName),
    Generate(String, ModelName),
    /// Abort the in-flight token stream (if any).
    CancelGeneration,
    /// Drop the last assistant row for a session (regen cleanup).
    DeleteLastAssistant(SessionId),
    FetchSessions,
    LoadSession(SessionId),
    DeleteSession(SessionId),
    CreateSession(SessionId, String, ModelName),
    /// Persist a refined title after the first turn finishes.
    RenameSession(SessionId, String),
    SaveMessage(SessionId, String, String),
}

#[derive(PartialEq, Debug)]
pub enum CurrentTab {
    Models,
    Chat,
}

/// Chat pane focus.
#[derive(PartialEq, Debug, Clone, Copy)]
pub enum ChatFocus {
    /// Side panel — session list.
    Sessions,
    /// Main transcript — scroll with ↑↓ / j k / PgUp/Dn.
    Conversation,
    /// Composer — type / send.
    Input,
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

    /// Right-side inspect pane open in the models library.
    pub show_info: bool,
    pub model_info: Option<crate::api::types::ShowModelResponse>,
    /// Name the inspect payload belongs to (detect stale cache on selection change).
    pub model_info_for: Option<String>,
    pub info_scroll: u16,
    pub info_max_scroll: u16,

    pub input: tui_textarea::TextArea<'static>,
    pub messages: Vec<crate::db::repo::Message>,
    pub is_generating: bool,
    /// Set when the user aborts a stream so `GenerationDone` does not persist it.
    pub generation_cancelled: bool,
    pub toasts: Vec<Toast>,

    pub sessions: Vec<crate::db::repo::Session>,
    pub selected_session_index: usize,
    pub current_session_id: Option<String>,
    /// Logical line offset into the transcript (`0` = top).
    pub chat_scroll: u16,
    /// When true, keep the transcript pinned to the latest lines.
    pub chat_follow: bool,
    /// Max useful `chat_scroll` from the last frame (for un-sticking from bottom).
    pub chat_max_scroll: u16,
    /// Spinner phase while `is_generating` (advanced on Tick).
    pub spinner_frame: usize,

    pub show_popup: bool,
    pub popup: Popup<'static>,
    /// Destructive-action confirmation overlay.
    pub confirm: Option<ConfirmPrompt>,
    /// In-chat model picker overlay (`m` from normal mode).
    pub model_picker: Option<ModelPicker>,
    /// Mouse hit-test regions from the last draw.
    pub hits: UiHits,
    /// First Ctrl+C arms quit; second within a short window exits (LLM-harness style).
    pub quit_armed: bool,
}

impl App {
    pub fn new(config: Config) -> Self {
        let mut input = tui_textarea::TextArea::default();
        input.set_placeholder_text("message…");
        input.set_cursor_line_style(ratatui::style::Style::default());

        let popup = Popup::new("Pull Model".to_string());

        Self {
            _config: config,
            current_tab: CurrentTab::Chat,
            // Start in insert so chat-first means “just type”.
            chat_focus: ChatFocus::Input,
            models_focus: ModelsFocus::List,
            should_quit: false,
            models: Vec::new(),
            selected_model_index: 0,
            sort_column: SortColumn::Name,
            show_info: false,
            model_info: None,
            model_info_for: None,
            info_scroll: 0,
            info_max_scroll: 0,
            input,
            messages: Vec::new(),
            is_generating: false,
            generation_cancelled: false,
            toasts: Vec::new(),
            sessions: Vec::new(),
            selected_session_index: 0,
            current_session_id: None,
            chat_scroll: 0,
            chat_follow: true,
            chat_max_scroll: 0,
            spinner_frame: 0,
            show_popup: false,
            popup,
            confirm: None,
            model_picker: None,
            hits: UiHits::default(),
            quit_armed: false,
        }
    }

    pub fn open_model_picker(&mut self) {
        self.model_picker = Some(ModelPicker::new(self.selected_model_index));
    }

    fn enter_insert(&mut self) {
        self.chat_focus = ChatFocus::Input;
    }

    fn enter_sessions(&mut self) {
        self.chat_focus = ChatFocus::Sessions;
    }

    fn enter_conversation(&mut self) {
        self.chat_focus = ChatFocus::Conversation;
    }

    pub fn scroll_chat_up(&mut self, step: u16) {
        // Leaving follow: start from the real bottom offset, not a sentinel.
        if self.chat_follow {
            self.chat_follow = false;
            self.chat_scroll = self.chat_max_scroll;
        }
        self.chat_scroll = self.chat_scroll.saturating_sub(step);
    }

    pub fn scroll_chat_down(&mut self, step: u16) {
        if self.chat_follow {
            return; // already pinned
        }
        let next = self.chat_scroll.saturating_add(step);
        if next >= self.chat_max_scroll {
            self.stick_chat_bottom();
        } else {
            self.chat_scroll = next;
        }
    }

    pub fn stick_chat_bottom(&mut self) {
        self.chat_follow = true;
        // Keep scroll in sync with last known max so UI state stays sane.
        self.chat_scroll = self.chat_max_scroll;
    }

    fn active_model_name(&self) -> String {
        self.models
            .get(self.selected_model_index)
            .map(|m| m.name.clone())
            .unwrap_or_else(|| "llama2:latest".to_string())
    }

    /// Stop streaming. Returns `CancelGeneration` when something was in flight.
    fn cancel_generation(&mut self) -> Vec<Action> {
        if !self.is_generating {
            return vec![];
        }
        self.is_generating = false;
        self.generation_cancelled = true;
        vec![Action::CancelGeneration]
    }

    /// Re-run the last user turn: strip trailing assistant reply, stream again.
    fn regenerate(&mut self) -> Vec<Action> {
        let mut actions = self.cancel_generation();

        while self
            .messages
            .last()
            .is_some_and(|m| m.role == "assistant")
        {
            self.messages.pop();
        }

        let Some(prompt) = self
            .messages
            .iter()
            .rev()
            .find(|m| m.role == "user")
            .map(|m| m.content.clone())
        else {
            return actions;
        };

        if let Some(sid) = &self.current_session_id {
            actions.push(Action::DeleteLastAssistant(SessionId(sid.clone())));
        }

        self.is_generating = true;
        self.generation_cancelled = false;
        self.stick_chat_bottom();
        actions.push(Action::Generate(
            prompt,
            ModelName(self.active_model_name()),
        ));
        actions
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
        if self.is_generating {
            self.spinner_frame = self.spinner_frame.wrapping_add(1);
        }
        self.toasts.retain_mut(|t| {
            if t.duration > 0 {
                t.duration -= 1;
                true
            } else {
                false
            }
        });
    }

    pub fn spinner_glyph(&self) -> &'static str {
        const FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
        FRAMES[self.spinner_frame % FRAMES.len()]
    }

    /// Short provisional title while generating (first line / words of the prompt).
    pub fn provisional_session_title(prompt: &str) -> String {
        session_title_from_text(prompt, 28)
    }

    /// Refined title after the turn completes (a bit longer, cleaned).
    pub fn final_session_title(prompt: &str) -> String {
        session_title_from_text(prompt, 48)
    }

    /// Insert/update a session at the top of the left list and select it.
    pub fn upsert_session_local(&mut self, id: String, title: String, model: String) {
        if let Some(existing) = self.sessions.iter_mut().find(|s| s.id.0 == id) {
            existing.title = title;
            if let Some(i) = self.sessions.iter().position(|s| s.id.0 == id) {
                self.selected_session_index = i;
            }
            return;
        }
        self.sessions.insert(
            0,
            crate::db::repo::Session {
                id: SessionId(id),
                title,
                model: ModelName(model),
                created_at: String::new(),
            },
        );
        self.selected_session_index = 0;
    }

    pub fn rename_session_local(&mut self, id: &str, title: String) {
        if let Some(s) = self.sessions.iter_mut().find(|s| s.id.0 == id) {
            s.title = title;
        }
    }

    /// Keep selection on `current_session_id` after a full list replace.
    pub fn select_current_session(&mut self) {
        if let Some(id) = &self.current_session_id {
            if let Some(i) = self.sessions.iter().position(|s| s.id.0 == *id) {
                self.selected_session_index = i;
            }
        }
    }

    pub fn show_error(&mut self, msg: &str) {
        self.toasts.push(Toast {
            message: msg.to_string(),
            duration: 20,
            color: crate::tui::styles::ERR,
        });
    }

    pub fn show_hint(&mut self, msg: &str) {
        self.toasts.push(Toast {
            message: msg.to_string(),
            duration: 12,
            color: crate::tui::styles::ACCENT,
        });
    }

    pub fn close_inspect(&mut self) {
        self.show_info = false;
        self.models_focus = ModelsFocus::List;
        self.info_scroll = 0;
    }

    /// Open/close the right-hand model inspect pane (F3).
    pub fn toggle_inspect(&mut self) -> Vec<Action> {
        if self.current_tab != CurrentTab::Models {
            self.current_tab = CurrentTab::Models;
        }
        if self.show_info {
            self.close_inspect();
            return vec![];
        }
        self.open_inspect()
    }

    pub fn open_inspect(&mut self) -> Vec<Action> {
        let Some(model) = self.models.get(self.selected_model_index) else {
            return vec![];
        };
        let name = model.name.clone();
        self.show_info = true;
        self.info_scroll = 0;
        self.models_focus = ModelsFocus::Info;
        if self.model_info_for.as_deref() != Some(name.as_str()) {
            self.model_info = None;
            self.model_info_for = Some(name.clone());
        }
        vec![Action::ShowModelInfo(ModelName(name))]
    }

    fn refresh_inspect_if_open(&mut self) -> Vec<Action> {
        if !self.show_info {
            return vec![];
        }
        let Some(model) = self.models.get(self.selected_model_index) else {
            return vec![];
        };
        let name = model.name.clone();
        self.info_scroll = 0;
        if self.model_info_for.as_deref() != Some(name.as_str()) {
            self.model_info = None;
            self.model_info_for = Some(name.clone());
        }
        self.models_focus = ModelsFocus::Info;
        vec![Action::ShowModelInfo(ModelName(name))]
    }

    pub fn scroll_info_up(&mut self, step: u16) {
        self.info_scroll = self.info_scroll.saturating_sub(step);
    }

    pub fn scroll_info_down(&mut self, step: u16) {
        let next = self.info_scroll.saturating_add(step);
        self.info_scroll = next.min(self.info_max_scroll);
    }

    /// Esc “pop” navigation: overlay → close inspect → leave library → leave insert.
    fn navigate_back(&mut self) -> Vec<Action> {
        self.quit_armed = false;
        if self.confirm.is_some() {
            self.confirm = None;
            return vec![];
        }
        if self.model_picker.is_some() {
            self.model_picker = None;
            return vec![];
        }
        if self.show_popup {
            self.show_popup = false;
            return vec![];
        }
        match self.current_tab {
            CurrentTab::Models => {
                if self.show_info {
                    self.close_inspect();
                } else {
                    // Leave library → chat composer.
                    self.current_tab = CurrentTab::Chat;
                    self.enter_insert();
                }
                vec![]
            }
            CurrentTab::Chat => {
                // INSERT → conversation → sessions (back out of the chat UI stack).
                self.chat_focus = match self.chat_focus {
                    ChatFocus::Input => ChatFocus::Conversation,
                    ChatFocus::Conversation => ChatFocus::Sessions,
                    ChatFocus::Sessions => ChatFocus::Sessions,
                };
                vec![]
            }
        }
    }

    /// Double Ctrl+C to quit; first Ctrl+C cancels a stream if one is running.
    fn handle_ctrl_c(&mut self) -> Vec<Action> {
        if self.is_generating {
            let actions = self.cancel_generation();
            self.quit_armed = true;
            self.show_hint("Ctrl+C again to quit");
            return actions;
        }
        if self.quit_armed {
            self.should_quit = true;
            self.quit_armed = false;
            return vec![Action::Quit];
        }
        self.quit_armed = true;
        self.show_hint("Ctrl+C again to quit");
        vec![]
    }

    /// Basic mouse: wheel scroll, click-to-focus panes, click session/model row.
    pub fn on_mouse(&mut self, mouse: MouseEvent) -> Vec<Action> {
        // Overlays eat mouse until dismissed with keyboard (keep it simple).
        if self.confirm.is_some() || self.model_picker.is_some() || self.show_popup {
            return vec![];
        }

        let col = mouse.column;
        let row = mouse.row;

        match mouse.kind {
            MouseEventKind::ScrollUp | MouseEventKind::ScrollDown => {
                let up = matches!(mouse.kind, MouseEventKind::ScrollUp);
                let step: u16 = 3;
                // Prefer the pane under the cursor; fall back to focused chat region.
                if UiHits::contains(self.hits.messages, col, row)
                    || (self.current_tab == CurrentTab::Chat
                        && !UiHits::contains(self.hits.sessions, col, row)
                        && !UiHits::contains(self.hits.composer, col, row)
                        && !UiHits::contains(self.hits.models_list, col, row)
                        && !UiHits::contains(self.hits.models_info, col, row))
                {
                    if up {
                        self.scroll_chat_up(step);
                    } else {
                        self.scroll_chat_down(step);
                    }
                } else if UiHits::contains(self.hits.models_info, col, row) {
                    if up {
                        self.scroll_info_up(step);
                    } else {
                        self.scroll_info_down(step);
                    }
                } else if UiHits::contains(self.hits.sessions, col, row) && !self.sessions.is_empty()
                {
                    if up {
                        self.selected_session_index = self.selected_session_index.saturating_sub(1);
                    } else if self.selected_session_index + 1 < self.sessions.len() {
                        self.selected_session_index += 1;
                    }
                } else if UiHits::contains(self.hits.models_list, col, row) && !self.models.is_empty()
                {
                    if up {
                        self.selected_model_index = self.selected_model_index.saturating_sub(1);
                    } else if self.selected_model_index + 1 < self.models.len() {
                        self.selected_model_index += 1;
                    }
                } else if self.current_tab == CurrentTab::Chat {
                    if up {
                        self.scroll_chat_up(step);
                    } else {
                        self.scroll_chat_down(step);
                    }
                }
                vec![]
            }
            MouseEventKind::Down(MouseButton::Left) => self.on_mouse_click(col, row),
            _ => vec![],
        }
    }

    fn on_mouse_click(&mut self, col: u16, row: u16) -> Vec<Action> {
        match self.current_tab {
            CurrentTab::Chat => {
                if UiHits::contains(self.hits.composer, col, row) {
                    self.enter_insert();
                    return vec![];
                }
                if UiHits::contains(self.hits.sessions, col, row) {
                    self.enter_sessions();
                    if let Some(idx) = row_index_in_list(self.hits.sessions, row) {
                        if idx < self.sessions.len() {
                            self.selected_session_index = idx;
                        }
                    }
                    return vec![];
                }
                if UiHits::contains(self.hits.messages, col, row) {
                    self.enter_conversation();
                    return vec![];
                }
            }
            CurrentTab::Models => {
                if UiHits::contains(self.hits.models_info, col, row) {
                    self.models_focus = ModelsFocus::Info;
                    return vec![];
                }
                if UiHits::contains(self.hits.models_list, col, row) {
                    self.models_focus = ModelsFocus::List;
                    // Header row lives above data; row 0 is first model.
                    if let Some(idx) = row_index_in_table(self.hits.models_list, row) {
                        if idx < self.models.len() {
                            self.selected_model_index = idx;
                            if self.show_info
                                && let Some(model) = self.models.get(self.selected_model_index)
                            {
                                return vec![Action::ShowModelInfo(ModelName(model.name.clone()))];
                            }
                        }
                    }
                    return vec![];
                }
            }
        }
        vec![]
    }

    pub fn on_key(&mut self, key: KeyEvent) -> Vec<Action> {
        // Ignore key release / media events — modern terminals (kitty protocol)
        // emit Press+Release for Tab, which would toggle focus twice and look like a no-op.
        if !matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
            return vec![];
        }

        // Global Ctrl+C — cancel stream / double-tap to quit (LLM harness style).
        if matches!(key.code, KeyCode::Char('c')) && key.modifiers.contains(KeyModifiers::CONTROL)
        {
            return self.handle_ctrl_c();
        }

        // Any other key cancels a pending quit arm.
        self.quit_armed = false;

        // Esc is always “back”: overlay → details → library → insert→normal.
        if matches!(key.code, KeyCode::Esc) {
            return self.navigate_back();
        }

        // Overlay stack: confirm > model picker > pull popup > page keys
        if self.confirm.is_some() {
            return self.on_key_confirm(key);
        }
        if self.model_picker.is_some() {
            return self.on_key_model_picker(key);
        }
        if self.show_popup {
            return self.on_key_popup(key);
        }

        // Tab / Shift+Tab / literal '\t' — cycle panel focus (never insert into textarea).
        let is_tab = matches!(key.code, KeyCode::Tab | KeyCode::BackTab)
            || matches!(key.code, KeyCode::Char('\t'));
        if is_tab {
            let reverse = matches!(key.code, KeyCode::BackTab)
                || key.modifiers.contains(KeyModifiers::SHIFT);
            return self.cycle_panel_focus(reverse);
        }

        match key.code {
            // F-keys: free of readline/emacs conflicts (Ctrl+A/E/B/F/D/…).
            KeyCode::F(2) => {
                self.current_tab = CurrentTab::Models;
                vec![]
            }
            KeyCode::F(1) => {
                self.current_tab = CurrentTab::Chat;
                self.enter_insert();
                vec![]
            }
            // Inspect selected model (library pane) — toggle open/close.
            KeyCode::F(3) => self.toggle_inspect(),
            _ => match self.current_tab {
                CurrentTab::Models => self.on_key_models(key),
                CurrentTab::Chat => self.on_key_chat(key),
            },
        }
    }

    /// Cycle focus between panes on the current screen.
    fn cycle_panel_focus(&mut self, _reverse: bool) -> Vec<Action> {
        match self.current_tab {
            CurrentTab::Chat => {
                // sessions → conversation → composer → sessions
                self.chat_focus = match self.chat_focus {
                    ChatFocus::Sessions => ChatFocus::Conversation,
                    ChatFocus::Conversation => ChatFocus::Input,
                    ChatFocus::Input => ChatFocus::Sessions,
                };
                vec![]
            }
            CurrentTab::Models => {
                if self.show_info {
                    self.models_focus = match self.models_focus {
                        ModelsFocus::List => ModelsFocus::Info,
                        ModelsFocus::Info => ModelsFocus::List,
                    };
                    vec![]
                } else {
                    // Open inspect so Tab has a second pane.
                    self.open_inspect()
                }
            }
        }
    }

    fn on_key_confirm(&mut self, key: KeyEvent) -> Vec<Action> {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                let Some(prompt) = self.confirm.take() else {
                    return vec![];
                };
                match prompt.action {
                    ConfirmAction::DeleteSession(id) => {
                        if let Some(current) = &self.current_session_id
                            && current == &id.0
                        {
                            self.messages.clear();
                            self.current_session_id = None;
                        }
                        // Keep selection valid after delete.
                        if self.selected_session_index > 0
                            && self.selected_session_index >= self.sessions.len().saturating_sub(1)
                        {
                            self.selected_session_index =
                                self.selected_session_index.saturating_sub(1);
                        }
                        vec![Action::DeleteSession(id)]
                    }
                    ConfirmAction::DeleteModel(name) => {
                        vec![Action::DeleteModel(name)]
                    }
                }
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                self.confirm = None;
                vec![]
            }
            _ => vec![],
        }
    }

    fn on_key_model_picker(&mut self, key: KeyEvent) -> Vec<Action> {
        let Some(picker) = self.model_picker.as_mut() else {
            return vec![];
        };

        let filtered_len = picker.filtered(&self.models).len();
        picker.clamp_selection(filtered_len);

        match key.code {
            KeyCode::Esc => {
                self.model_picker = None;
                vec![]
            }
            KeyCode::Enter => {
                let filtered = picker.filtered(&self.models);
                if let Some((orig_idx, _)) = filtered.get(picker.selected) {
                    self.selected_model_index = *orig_idx;
                    self.model_picker = None;
                    // Stay in chat, jump to composer with the new model.
                    self.current_tab = CurrentTab::Chat;
                    self.enter_insert();
                }
                vec![]
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if filtered_len > 0 && picker.selected + 1 < filtered_len {
                    picker.selected += 1;
                }
                vec![]
            }
            KeyCode::Char('k') | KeyCode::Up => {
                picker.selected = picker.selected.saturating_sub(1);
                vec![]
            }
            KeyCode::Backspace => {
                picker.query.pop();
                let len = picker.filtered(&self.models).len();
                picker.clamp_selection(len);
                vec![]
            }
            KeyCode::Char(c)
                if !key.modifiers.contains(KeyModifiers::CONTROL)
                    && !key.modifiers.contains(KeyModifiers::ALT) =>
            {
                // Allow typing filter; j/k already handled above for nav when alone —
                // if query non-empty, j/k still navigate (not typed). Fine for v1.
                if c != 'j' && c != 'k' {
                    picker.query.push(c);
                    let len = picker.filtered(&self.models).len();
                    picker.clamp_selection(len);
                } else if !picker.query.is_empty() {
                    // When filtering, prefer nav; users can type J/K rarely needed in names.
                }
                vec![]
            }
            _ => vec![],
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
                    self.popup = Popup::new("Pull Model".into());
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
            KeyCode::Left if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.enter_sessions();
                return vec![];
            }
            KeyCode::Right if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.enter_insert();
                return vec![];
            }
            _ => {}
        }

        // While streaming in insert, only allow scroll / regen / new — typing is blocked.
        if self.is_generating && self.chat_focus == ChatFocus::Input {
            match key.code {
                KeyCode::Up => {
                    self.scroll_chat_up(3);
                    return vec![];
                }
                KeyCode::Down => {
                    self.scroll_chat_down(3);
                    return vec![];
                }
                KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {}
                KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {}
                _ => return vec![],
            }
        }

        match self.chat_focus {
            ChatFocus::Sessions => self.on_key_chat_sessions(key),
            ChatFocus::Conversation => self.on_key_chat_conversation(key),
            ChatFocus::Input => self.on_key_chat_insert(key),
        }
    }

    /// Sessions side panel.
    fn on_key_chat_sessions(&mut self, key: KeyEvent) -> Vec<Action> {
        match key.code {
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
            KeyCode::Char('i') | KeyCode::Char('a') => {
                self.enter_insert();
                vec![]
            }
            KeyCode::Char('l') => {
                // Vim-ish: open conversation pane.
                self.enter_conversation();
                vec![]
            }
            KeyCode::Char('m') => {
                self.open_model_picker();
                vec![]
            }
            KeyCode::Char('n') => {
                self.messages.clear();
                self.current_session_id = None;
                self.stick_chat_bottom();
                self.enter_insert();
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
                    self.enter_insert();
                    vec![Action::LoadSession(SessionId(id))]
                } else {
                    self.enter_insert();
                    vec![]
                }
            }
            KeyCode::Char('d') => {
                if let Some(session) = self.sessions.get(self.selected_session_index) {
                    self.confirm = Some(ConfirmPrompt::delete_session(
                        session.id.clone(),
                        &session.title,
                    ));
                }
                vec![]
            }
            KeyCode::Char('r') => self.regenerate(),
            KeyCode::Char('c') if self.is_generating => self.cancel_generation(),
            _ => vec![],
        }
    }

    /// Main transcript pane — ↑↓ / j k / PgUp PgDn scroll.
    fn on_key_chat_conversation(&mut self, key: KeyEvent) -> Vec<Action> {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.scroll_chat_up(3);
                vec![]
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.scroll_chat_down(3);
                vec![]
            }
            KeyCode::PageUp => {
                self.scroll_chat_up(15);
                vec![]
            }
            KeyCode::PageDown => {
                self.scroll_chat_down(15);
                vec![]
            }
            KeyCode::Char('g') => {
                // Top of transcript.
                self.chat_follow = false;
                self.chat_scroll = 0;
                vec![]
            }
            KeyCode::Char('G') => {
                self.stick_chat_bottom();
                vec![]
            }
            KeyCode::Char('i') | KeyCode::Char('a') | KeyCode::Enter => {
                self.enter_insert();
                vec![]
            }
            KeyCode::Char('r') => self.regenerate(),
            KeyCode::Char('c') if self.is_generating => self.cancel_generation(),
            KeyCode::Char('n') => {
                self.messages.clear();
                self.current_session_id = None;
                self.stick_chat_bottom();
                self.enter_insert();
                vec![]
            }
            _ => vec![],
        }
    }

    /// Insert mode: composer owns the keyboard.
    fn on_key_chat_insert(&mut self, key: KeyEvent) -> Vec<Action> {
        match key.code {
            KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.regenerate()
            }
            KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                let actions = self.cancel_generation();
                self.messages.clear();
                self.current_session_id = None;
                self.stick_chat_bottom();
                actions
            }
            KeyCode::Char('m') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.open_model_picker();
                vec![]
            }
            KeyCode::Up => {
                self.scroll_chat_up(3);
                vec![]
            }
            KeyCode::Down => {
                self.scroll_chat_down(3);
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
                    self.input.set_placeholder_text("message…");
                    self.input.set_cursor_line_style(ratatui::style::Style::default());

                    self.is_generating = true;
                    self.generation_cancelled = false;
                    self.stick_chat_bottom();

                    let model_name = self.active_model_name();
                    let mut actions = Vec::new();

                    let session_id = if let Some(id) = &self.current_session_id {
                        id.clone()
                    } else {
                        // New session: show it on the left immediately (provisional title).
                        let new_id = Uuid::new_v4().to_string();
                        let title = Self::provisional_session_title(&prompt);
                        self.current_session_id = Some(new_id.clone());
                        self.upsert_session_local(
                            new_id.clone(),
                            title.clone(),
                            model_name.clone(),
                        );
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
        }
    }

    fn on_key_models(&mut self, key: KeyEvent) -> Vec<Action> {
        // When inspect pane is focused, arrows/pg scroll the inspector.
        if self.show_info && self.models_focus == ModelsFocus::Info {
            match key.code {
                KeyCode::Char('i') | KeyCode::F(3) => {
                    return self.toggle_inspect();
                }
                KeyCode::PageUp => {
                    self.scroll_info_up(15);
                    return vec![];
                }
                KeyCode::PageDown => {
                    self.scroll_info_down(15);
                    return vec![];
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    self.scroll_info_up(3);
                    return vec![];
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.scroll_info_down(3);
                    return vec![];
                }
                // Still allow list nav with capital J/K? Keep simple: only scroll here.
                _ => {}
            }
        }

        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                if !self.models.is_empty() && self.selected_model_index < self.models.len() - 1 {
                    self.selected_model_index += 1;
                    return self.refresh_inspect_if_open();
                }
                vec![]
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.selected_model_index > 0 {
                    self.selected_model_index -= 1;
                    return self.refresh_inspect_if_open();
                }
                vec![]
            }
            KeyCode::Char('d') => {
                if let Some(model) = self.models.get(self.selected_model_index) {
                    self.confirm = Some(ConfirmPrompt::delete_model(ModelName(model.name.clone())));
                }
                vec![]
            }
            KeyCode::Char('p') => {
                self.show_popup = true;
                vec![]
            }
            KeyCode::Char('m') => {
                self.open_model_picker();
                vec![]
            }
            // `i` / F3 — inspect toggle (open focuses the pane).
            KeyCode::Char('i') => self.toggle_inspect(),
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
                self.close_inspect();
                self.current_tab = CurrentTab::Chat;
                self.messages.clear();
                self.current_session_id = None;
                self.stick_chat_bottom();
                self.enter_insert();
                vec![]
            }
            _ => vec![],
        }
    }
}

/// Collapse whitespace and truncate with an ellipsis for session list titles.
fn session_title_from_text(text: &str, max: usize) -> String {
    let flat: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.is_empty() {
        return "new chat".into();
    }
    let chars: Vec<char> = flat.chars().collect();
    if chars.len() <= max {
        return flat;
    }
    if max <= 1 {
        return "…".into();
    }
    let mut s: String = chars.into_iter().take(max.saturating_sub(1)).collect();
    s.push('…');
    s
}

/// Content row under the cursor for a bordered pane (`pane_block` chrome).
fn content_inner(area: Rect) -> Rect {
    // Matches styles::pane_block: borders all sides + horizontal padding 1.
    crate::tui::styles::pane_block("", false).inner(area)
}

fn row_index_in_list(area: Option<Rect>, row: u16) -> Option<usize> {
    let area = area?;
    let inner = content_inner(area);
    if row < inner.y || row >= inner.y.saturating_add(inner.height) {
        return None;
    }
    Some((row - inner.y) as usize)
}

fn row_index_in_table(area: Option<Rect>, row: u16) -> Option<usize> {
    let area = area?;
    let inner = content_inner(area);
    if row < inner.y || row >= inner.y.saturating_add(inner.height) {
        return None;
    }
    // Skip header row (first line of table content).
    let rel = (row - inner.y) as usize;
    rel.checked_sub(1)
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

    fn mock_key_release(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Release,
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
        assert_eq!(app.current_tab, CurrentTab::Chat);

        app.on_key(mock_key(KeyCode::F(2)));
        assert_eq!(app.current_tab, CurrentTab::Models);

        app.on_key(mock_key(KeyCode::F(1)));
        assert_eq!(app.current_tab, CurrentTab::Chat);
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

        app.current_tab = CurrentTab::Chat;
        app.chat_focus = ChatFocus::Input;
        app.on_key(mock_key(KeyCode::Tab));
        assert_eq!(app.chat_focus, ChatFocus::Sessions);
        app.on_key(mock_key(KeyCode::Tab));
        assert_eq!(app.chat_focus, ChatFocus::Conversation);
        app.on_key(mock_key(KeyCode::Tab));
        assert_eq!(app.chat_focus, ChatFocus::Input);

        // Release events must not toggle again (would cancel the Press).
        app.on_key(mock_key_release(KeyCode::Tab));
        assert_eq!(app.chat_focus, ChatFocus::Input);

        // Char('\t') also cycles (some terminals send this).
        app.on_key(mock_key(KeyCode::Char('\t')));
        assert_eq!(app.chat_focus, ChatFocus::Sessions);

        app.show_info = true;
        app.current_tab = CurrentTab::Models;
        app.models_focus = ModelsFocus::List;
        app.on_key(mock_key(KeyCode::Tab));
        assert_eq!(app.models_focus, ModelsFocus::Info);
        app.on_key(mock_key(KeyCode::Tab));
        assert_eq!(app.models_focus, ModelsFocus::List);
    }

    #[test]
    fn test_f3_toggles_inspect() {
        let config = Config {
            _config_dir: "/tmp".into(),
            _data_dir: "/tmp".into(),
            log_dir: "/tmp".into(),
            db_path: "/tmp/test.db".into(),
            ollama_url: "http://localhost:11434".into(),
        };
        let mut app = App::new(config);
        app.models = vec![Model {
            name: "llama3".into(),
            modified_at: String::new(),
            size: 1,
            digest: String::new(),
            details: None,
        }];
        app.current_tab = CurrentTab::Models;
        let actions = app.on_key(mock_key(KeyCode::F(3)));
        assert!(app.show_info);
        assert_eq!(app.models_focus, ModelsFocus::Info);
        assert!(matches!(actions.first(), Some(Action::ShowModelInfo(_))));

        app.on_key(mock_key(KeyCode::F(3)));
        assert!(!app.show_info);
        assert_eq!(app.models_focus, ModelsFocus::List);

        app.on_key(mock_key(KeyCode::F(3)));
        assert!(app.show_info);
        app.on_key(mock_key(KeyCode::Esc));
        assert!(!app.show_info);
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
    fn test_double_ctrl_c_quits() {
        let config = Config {
            _config_dir: "/tmp".into(),
            _data_dir: "/tmp".into(),
            log_dir: "/tmp".into(),
            db_path: "/tmp/test.db".into(),
            ollama_url: "http://localhost:11434".into(),
        };
        let mut app = App::new(config);
        assert!(!app.should_quit);

        let a1 = app.on_key(mock_key_ctrl(KeyCode::Char('c')));
        assert!(a1.is_empty());
        assert!(app.quit_armed);
        assert!(!app.should_quit);

        let a2 = app.on_key(mock_key_ctrl(KeyCode::Char('c')));
        assert_eq!(a2, vec![Action::Quit]);
        assert!(app.should_quit);
    }

    #[test]
    fn test_esc_leaves_models_for_chat() {
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
        app.on_key(mock_key(KeyCode::Esc));
        assert_eq!(app.current_tab, CurrentTab::Chat);
        assert_eq!(app.chat_focus, ChatFocus::Input);

        app.current_tab = CurrentTab::Models;
        app.show_info = true;
        app.on_key(mock_key(KeyCode::Esc));
        assert_eq!(app.current_tab, CurrentTab::Models);
        assert!(!app.show_info);
        app.on_key(mock_key(KeyCode::Esc));
        assert_eq!(app.current_tab, CurrentTab::Chat);
    }

    #[test]
    fn test_esc_enters_normal_from_insert() {
        let config = Config {
            _config_dir: "/tmp".into(),
            _data_dir: "/tmp".into(),
            log_dir: "/tmp".into(),
            db_path: "/tmp/test.db".into(),
            ollama_url: "http://localhost:11434".into(),
        };
        let mut app = App::new(config);
        assert_eq!(app.chat_focus, ChatFocus::Input);
        app.on_key(mock_key(KeyCode::Esc));
        assert_eq!(app.chat_focus, ChatFocus::Conversation);
        app.on_key(mock_key(KeyCode::Esc));
        assert_eq!(app.chat_focus, ChatFocus::Sessions);
        app.on_key(mock_key(KeyCode::Char('i')));
        assert_eq!(app.chat_focus, ChatFocus::Input);
    }

    #[test]
    fn test_delete_requires_confirm() {
        let config = Config {
            _config_dir: "/tmp".into(),
            _data_dir: "/tmp".into(),
            log_dir: "/tmp".into(),
            db_path: "/tmp/test.db".into(),
            ollama_url: "http://localhost:11434".into(),
        };
        let mut app = App::new(config);
        app.sessions.push(crate::db::repo::Session {
            id: SessionId("s1".into()),
            title: "hello".into(),
            model: ModelName("m".into()),
            created_at: "now".into(),
        });
        app.enter_sessions();
        let actions = app.on_key(mock_key(KeyCode::Char('d')));
        assert!(actions.is_empty());
        assert!(app.confirm.is_some());

        let actions = app.on_key(mock_key(KeyCode::Char('n')));
        assert!(actions.is_empty());
        assert!(app.confirm.is_none());

        app.enter_sessions();
        app.on_key(mock_key(KeyCode::Char('d')));
        let actions = app.on_key(mock_key(KeyCode::Char('y')));
        assert_eq!(actions, vec![Action::DeleteSession(SessionId("s1".into()))]);
        assert!(app.confirm.is_none());
    }

    #[test]
    fn test_cancel_generation_action() {
        let config = Config {
            _config_dir: "/tmp".into(),
            _data_dir: "/tmp".into(),
            log_dir: "/tmp".into(),
            db_path: "/tmp/test.db".into(),
            ollama_url: "http://localhost:11434".into(),
        };
        let mut app = App::new(config);
        assert!(app.cancel_generation().is_empty());
        app.is_generating = true;
        assert_eq!(app.cancel_generation(), vec![Action::CancelGeneration]);
        assert!(!app.is_generating);
    }

    #[test]
    fn test_regenerate_strips_assistant_and_replays_user() {
        let config = Config {
            _config_dir: "/tmp".into(),
            _data_dir: "/tmp".into(),
            log_dir: "/tmp".into(),
            db_path: "/tmp/test.db".into(),
            ollama_url: "http://localhost:11434".into(),
        };
        let mut app = App::new(config);
        app.current_session_id = Some("s1".into());
        app.models = vec![Model {
            name: "m1".into(),
            modified_at: String::new(),
            size: 1,
            digest: String::new(),
            details: None,
        }];
        app.messages = vec![
            crate::db::repo::Message {
                role: "user".into(),
                content: "hello".into(),
            },
            crate::db::repo::Message {
                role: "assistant".into(),
                content: "hi there".into(),
            },
        ];

        let actions = app.regenerate();
        assert_eq!(app.messages.len(), 1);
        assert_eq!(app.messages[0].role, "user");
        assert!(app.is_generating);
        assert!(actions.contains(&Action::DeleteLastAssistant(SessionId("s1".into()))));
        assert!(actions.contains(&Action::Generate(
            "hello".into(),
            ModelName("m1".into())
        )));
    }

    #[test]
    fn test_model_picker_opens_and_selects() {
        let config = Config {
            _config_dir: "/tmp".into(),
            _data_dir: "/tmp".into(),
            log_dir: "/tmp".into(),
            db_path: "/tmp/test.db".into(),
            ollama_url: "http://localhost:11434".into(),
        };
        let mut app = App::new(config);
        app.models = vec![
            Model {
                name: "alpha".into(),
                modified_at: String::new(),
                size: 1,
                digest: String::new(),
                details: None,
            },
            Model {
                name: "beta".into(),
                modified_at: String::new(),
                size: 2,
                digest: String::new(),
                details: None,
            },
        ];
        app.enter_sessions();
        app.on_key(mock_key(KeyCode::Char('m')));
        assert!(app.model_picker.is_some());
        app.on_key(mock_key(KeyCode::Char('j')));
        app.on_key(mock_key(KeyCode::Enter));
        assert!(app.model_picker.is_none());
        assert_eq!(app.selected_model_index, 1);
        assert_eq!(app.chat_focus, ChatFocus::Input);
    }

    #[test]
    fn test_session_appears_immediately_and_title_refines() {
        let config = Config {
            _config_dir: "/tmp".into(),
            _data_dir: "/tmp".into(),
            log_dir: "/tmp".into(),
            db_path: "/tmp/test.db".into(),
            ollama_url: "http://localhost:11434".into(),
        };
        let mut app = App::new(config);
        let prompt = "hello world this is a fairly long first message for the chat";
        let provisional = App::provisional_session_title(prompt);
        app.upsert_session_local("sid1".into(), provisional.clone(), "m1".into());
        assert_eq!(app.sessions.len(), 1);
        assert_eq!(app.sessions[0].title, provisional);
        assert_eq!(app.selected_session_index, 0);

        let final_t = App::final_session_title(prompt);
        app.rename_session_local("sid1", final_t.clone());
        assert_eq!(app.sessions[0].title, final_t);
        assert!(final_t.chars().count() >= provisional.chars().count() || final_t.ends_with('…'));
    }

    #[test]
    fn test_scroll_up_unsticks_from_bottom() {
        let config = Config {
            _config_dir: "/tmp".into(),
            _data_dir: "/tmp".into(),
            log_dir: "/tmp".into(),
            db_path: "/tmp/test.db".into(),
            ollama_url: "http://localhost:11434".into(),
        };
        let mut app = App::new(config);
        app.chat_max_scroll = 40;
        app.stick_chat_bottom();
        assert!(app.chat_follow);
        assert_eq!(app.chat_scroll, 40);

        app.scroll_chat_up(3);
        assert!(!app.chat_follow);
        assert_eq!(app.chat_scroll, 37);

        app.scroll_chat_down(100);
        assert!(app.chat_follow);
        assert_eq!(app.chat_scroll, 40);
    }

    #[test]
    fn test_tab_cycles_three_chat_panes() {
        let config = Config {
            _config_dir: "/tmp".into(),
            _data_dir: "/tmp".into(),
            log_dir: "/tmp".into(),
            db_path: "/tmp/test.db".into(),
            ollama_url: "http://localhost:11434".into(),
        };
        let mut app = App::new(config);
        app.chat_focus = ChatFocus::Sessions;
        app.on_key(mock_key(KeyCode::Tab));
        assert_eq!(app.chat_focus, ChatFocus::Conversation);
        app.on_key(mock_key(KeyCode::Tab));
        assert_eq!(app.chat_focus, ChatFocus::Input);
        app.on_key(mock_key(KeyCode::Tab));
        assert_eq!(app.chat_focus, ChatFocus::Sessions);
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
