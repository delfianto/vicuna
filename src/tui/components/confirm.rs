use crate::api::types::{ModelName, SessionId};
use crate::tui::styles;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
};

#[derive(Clone, Debug)]
pub enum ConfirmAction {
    DeleteSession(SessionId),
    DeleteModel(ModelName),
}

#[derive(Clone, Debug)]
pub struct ConfirmPrompt {
    pub title: String,
    pub detail: String,
    pub action: ConfirmAction,
}

impl ConfirmPrompt {
    pub fn delete_session(id: SessionId, title: &str) -> Self {
        let label = if title.is_empty() { "untitled" } else { title };
        Self {
            title: "delete session".into(),
            detail: format!("“{label}” will be removed permanently."),
            action: ConfirmAction::DeleteSession(id),
        }
    }

    pub fn delete_model(name: ModelName) -> Self {
        let label = name.0.clone();
        Self {
            title: "delete model".into(),
            detail: format!("“{label}” and its chats will be removed."),
            action: ConfirmAction::DeleteModel(name),
        }
    }

    pub fn draw(&self, f: &mut Frame, area: Rect) {
        let box_area = centered_rect(52, 32, area);

        let plate = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(styles::ERR))
            .style(Style::default().bg(styles::BG_SURFACE))
            .title(Span::styled(
                format!(" {} ", self.title),
                Style::default()
                    .fg(styles::ERR)
                    .add_modifier(Modifier::BOLD),
            ));

        f.render_widget(Clear, box_area);
        f.render_widget(plate, box_area);

        let inner = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .spacing(1)
            .constraints([
                Constraint::Length(2),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Min(1),
            ])
            .split(box_area);

        let detail = Paragraph::new(Line::from(Span::styled(
            self.detail.clone(),
            styles::text(),
        )));
        f.render_widget(detail, inner[0]);

        let warn = Paragraph::new(Line::from(Span::styled(
            "this cannot be undone",
            styles::muted().add_modifier(Modifier::ITALIC),
        )));
        f.render_widget(warn, inner[1]);

        let help = Paragraph::new(styles::help_line(
            &[("y", "confirm"), ("n/esc", "cancel")],
            inner[2].width.saturating_sub(1),
        ));
        f.render_widget(help, inner[2]);
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
