use crate::tui::styles;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};
use tui_textarea::TextArea;

pub struct Popup<'a> {
    pub title: String,
    pub textarea: TextArea<'a>,
}

impl<'a> Popup<'a> {
    pub fn new(title: String) -> Self {
        let mut textarea = TextArea::default();
        textarea.set_placeholder_text("e.g. llama3.2  or  mistral:7b");
        textarea.set_cursor_line_style(Style::default());
        textarea.set_block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(styles::BORDER_FOCUS))
                .title(Span::styled(" model ", styles::accent_bold())),
        );
        Self { title, textarea }
    }

    pub fn draw(&self, f: &mut Frame, area: Rect) {
        let area = centered_rect(50, 28, area);

        // Dim backdrop plate
        let plate = Block::default()
            .borders(Borders::ALL)
            .border_type(ratatui::widgets::BorderType::Rounded)
            .border_style(Style::default().fg(styles::ACCENT))
            .style(Style::default().bg(styles::BG_SURFACE))
            .title(Span::styled(
                format!(" {} ", self.title.to_lowercase()),
                styles::accent_bold(),
            ));

        f.render_widget(Clear, area);
        f.render_widget(plate, area);

        let inner = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .spacing(1)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(3),
                Constraint::Length(1),
                Constraint::Min(1),
            ])
            .split(area);

        let hint = Paragraph::new(Line::from(Span::styled(
            "name from the ollama library",
            styles::muted(),
        )));
        f.render_widget(hint, inner[0]);

        f.render_widget(&self.textarea, inner[1]);

        let help = Paragraph::new(styles::help_line(
            &[("enter", "pull"), ("esc", "cancel")],
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
