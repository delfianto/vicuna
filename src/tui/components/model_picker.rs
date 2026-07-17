use crate::api::modelfile;
use crate::api::types::Model;
use crate::tui::styles;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, ListState, Paragraph},
};

#[derive(Clone, Debug)]
pub struct ModelPicker {
    pub selected: usize,
    pub query: String,
}

impl ModelPicker {
    pub fn new(selected_model_index: usize) -> Self {
        Self {
            selected: selected_model_index,
            query: String::new(),
        }
    }

    /// Models matching the filter query (case-insensitive substring).
    pub fn filtered<'a>(&self, models: &'a [Model]) -> Vec<(usize, &'a Model)> {
        let q = self.query.to_lowercase();
        models
            .iter()
            .enumerate()
            .filter(|(_, m)| {
                if q.is_empty() {
                    true
                } else {
                    m.name.to_lowercase().contains(&q)
                        || modelfile::sanitize_model_name(&m.name)
                            .to_lowercase()
                            .contains(&q)
                }
            })
            .collect()
    }

    pub fn clamp_selection(&mut self, filtered_len: usize) {
        if filtered_len == 0 {
            self.selected = 0;
        } else if self.selected >= filtered_len {
            self.selected = filtered_len - 1;
        }
    }

    pub fn draw(&self, f: &mut Frame, area: Rect, models: &[Model]) {
        let box_area = centered_rect(56, 60, area);
        let filtered = self.filtered(models);

        let plate = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(styles::ACCENT))
            .style(Style::default().bg(styles::BG_SURFACE))
            .title(Span::styled(" pick model ", styles::accent_bold()));

        f.render_widget(Clear, box_area);
        f.render_widget(plate, box_area);

        let inner = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .spacing(1)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Min(3),
                Constraint::Length(1),
            ])
            .split(box_area);

        let filter_line = if self.query.is_empty() {
            Line::from(Span::styled(
                "type to filter…",
                styles::dim().add_modifier(Modifier::ITALIC),
            ))
        } else {
            Line::from(vec![
                Span::styled("filter ", styles::muted()),
                Span::styled(self.query.clone(), styles::accent_bold()),
                Span::styled("▌", styles::accent()),
            ])
        };
        f.render_widget(Paragraph::new(filter_line), inner[0]);

        let count = Paragraph::new(Line::from(Span::styled(
            format!("{} models", filtered.len()),
            styles::dim(),
        )));
        f.render_widget(count, inner[1]);

        let items: Vec<ListItem> = if filtered.is_empty() {
            vec![ListItem::new(Line::from(Span::styled(
                "  no matches",
                styles::muted().add_modifier(Modifier::ITALIC),
            )))]
        } else {
            filtered
                .iter()
                .map(|(_, m)| {
                    let name = modelfile::sanitize_model_name(&m.name).to_lowercase();
                    ListItem::new(Line::from(vec![
                        Span::styled(format!("{name}  "), styles::text()),
                        Span::styled(m.name.clone(), styles::dim()),
                    ]))
                })
                .collect()
        };

        let list = List::new(items)
            .highlight_style(styles::HIGHLIGHT_STYLE)
            .highlight_symbol("▎ ");

        let mut state = ListState::default();
        if !filtered.is_empty() {
            state.select(Some(self.selected.min(filtered.len() - 1)));
        }
        f.render_stateful_widget(list, inner[2], &mut state);

        f.render_widget(
            Paragraph::new(styles::help_line(
                &[
                    ("j/k", "nav"),
                    ("enter", "use"),
                    ("type", "filter"),
                    ("esc", "close"),
                ],
                inner[3].width.saturating_sub(1),
            )),
            inner[3],
        );
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
