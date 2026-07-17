use crate::tui::styles;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Paragraph, StatefulWidget, Widget, Wrap};
use tui_markdown::{Options, StyleSheet, from_str_with_options};

/// Vicuna-tuned markdown theme — readable body, clear code, soft role headers.
#[derive(Debug, Clone, Copy, Default)]
pub struct VicunaMarkdownStyle;

impl StyleSheet for VicunaMarkdownStyle {
    fn heading(&self, level: u8) -> Style {
        match level {
            1 => Style::new()
                .fg(styles::ACCENT)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            2 => Style::new()
                .fg(styles::ACCENT)
                .add_modifier(Modifier::BOLD),
            3 => Style::new()
                .fg(styles::ROLE_ASSISTANT)
                .add_modifier(Modifier::BOLD),
            _ => Style::new()
                .fg(styles::TEXT_MUTED)
                .add_modifier(Modifier::BOLD | Modifier::ITALIC),
        }
    }

    fn code(&self) -> Style {
        Style::new()
            .fg(Color::Rgb(220, 220, 210))
            .bg(Color::Rgb(36, 36, 42))
    }

    fn link(&self) -> Style {
        Style::new()
            .fg(styles::ROLE_USER)
            .add_modifier(Modifier::UNDERLINED)
    }

    fn blockquote(&self) -> Style {
        Style::new()
            .fg(styles::TEXT_MUTED)
            .add_modifier(Modifier::ITALIC)
    }

    fn heading_meta(&self) -> Style {
        Style::new().fg(styles::TEXT_DIM)
    }

    fn metadata_block(&self) -> Style {
        Style::new().fg(styles::WARN)
    }
}

pub struct MarkdownViewer<'a> {
    content: &'a str,
    scroll: u16,
    follow: bool,
    block: Option<Block<'a>>,
}

impl<'a> MarkdownViewer<'a> {
    pub fn new(content: &'a str) -> Self {
        Self {
            content,
            scroll: 0,
            follow: false,
            block: None,
        }
    }

    pub fn scroll(mut self, offset: u16) -> Self {
        self.scroll = offset;
        self
    }

    pub fn follow(mut self, follow: bool) -> Self {
        self.follow = follow;
        self
    }

    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }

    /// Max logical-line scroll for a given content width/height (mirrors render math).
    pub fn max_scroll_for(content: &str, _inner_width: u16, inner_height: u16) -> u16 {
        let text = render_markdown(content);
        let line_count = text.lines.len() as u16;
        line_count.saturating_sub(inner_height.max(1))
    }
}

fn render_markdown(content: &str) -> ratatui::text::Text<'_> {
    let options = Options::new(VicunaMarkdownStyle);
    from_str_with_options(content, &options)
}

impl<'a> Widget for MarkdownViewer<'a> {
    fn render(self, area: Rect, buf: &mut ratatui::buffer::Buffer) {
        let text = render_markdown(self.content);

        let inner_area = if let Some(ref block) = self.block {
            block.inner(area)
        } else {
            area
        };

        if inner_area.width == 0 || inner_area.height == 0 {
            return;
        }

        // Paragraph::scroll is a *logical* line offset.
        let line_count = text.lines.len() as u16;
        let max_scroll = line_count.saturating_sub(inner_area.height.max(1));
        let scroll = if self.follow {
            max_scroll
        } else {
            self.scroll.min(max_scroll)
        };

        let mut p = Paragraph::new(text)
            .style(styles::text())
            .wrap(Wrap { trim: false })
            .scroll((scroll, 0));

        if let Some(block) = self.block {
            p = p.block(block);
        }

        p.render(area, buf);

        // Always draw a scrollbar track when content can scroll (or is focused overflow).
        if max_scroll > 0 || line_count > inner_area.height {
            let bar_area = Rect {
                x: area.x + area.width.saturating_sub(1),
                y: inner_area.y,
                width: 1,
                height: inner_area.height,
            };
            if bar_area.width > 0 && bar_area.height > 0 {
                let scrollbar = ratatui::widgets::Scrollbar::new(
                    ratatui::widgets::ScrollbarOrientation::VerticalRight,
                )
                .begin_symbol(Some("▲"))
                .end_symbol(Some("▼"))
                .style(Style::default().fg(styles::TEXT_DIM))
                .thumb_style(Style::default().fg(styles::ACCENT));

                let content_len = (max_scroll as usize).saturating_add(1).max(line_count as usize);
                let mut scrollbar_state = ratatui::widgets::ScrollbarState::new(content_len)
                    .position(scroll as usize);

                StatefulWidget::render(scrollbar, bar_area, buf, &mut scrollbar_state);
            }
        }
    }
}
