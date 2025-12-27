use ratatui::{
    widgets::{Block, Paragraph, Widget, Wrap},
};
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
        let mut p = Paragraph::new(text)
            .wrap(Wrap { trim: false })
            .scroll((self.scroll, 0));

        if let Some(block) = self.block {
            p = p.block(block);
        }

        p.render(area, buf);
    }
}
