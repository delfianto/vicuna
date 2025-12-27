use ratatui::widgets::{Block, Paragraph, StatefulWidget, Widget, Wrap};
use tui_markdown::from_str;

pub struct MarkdownViewer<'a> {
    content: &'a str,
    scroll: u16,
    block: Option<Block<'a>>,
}

impl<'a> MarkdownViewer<'a> {
    pub fn new(content: &'a str) -> Self {
        Self {
            content,
            scroll: 0,
            block: None,
        }
    }

    pub fn scroll(mut self, offset: u16) -> Self {
        self.scroll = offset;
        self
    }

    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }
}

impl<'a> Widget for MarkdownViewer<'a> {
    fn render(self, area: ratatui::layout::Rect, buf: &mut ratatui::buffer::Buffer) {
        let text = from_str(self.content);

        let inner_area = if let Some(ref block) = self.block {
            block.inner(area)
        } else {
            area
        };

        if inner_area.width == 0 || inner_area.height == 0 {
            return;
        }

        let mut visual_lines = 0;
        for line in &text.lines {
            let w = line.width() as u16;
            if w == 0 {
                visual_lines += 1;
            } else {
                visual_lines += w.div_ceil(inner_area.width);
            }
        }

        let max_scroll = visual_lines.saturating_sub(inner_area.height);
        let scroll = self.scroll.min(max_scroll);

        let mut p = Paragraph::new(text.clone())
            .wrap(Wrap { trim: false })
            .scroll((scroll, 0));

        if let Some(block) = self.block {
            p = p.block(block);
        }

        p.render(area, buf);

        if usize::from(visual_lines) > usize::from(inner_area.height) {
            let scrollbar = ratatui::widgets::Scrollbar::new(
                ratatui::widgets::ScrollbarOrientation::VerticalRight,
            )
            .symbols(ratatui::symbols::scrollbar::VERTICAL)
            .begin_symbol(Some("▲"))
            .end_symbol(Some("▼"));

            let mut scrollbar_state =
                ratatui::widgets::ScrollbarState::new(usize::from(visual_lines))
                    .position(usize::from(scroll));

            StatefulWidget::render(scrollbar, area, buf, &mut scrollbar_state);
        }
    }
}
