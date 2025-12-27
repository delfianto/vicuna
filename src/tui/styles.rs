use ratatui::style::{Color, Modifier, Style};

pub const HIGHLIGHT_STYLE: Style = Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD);

pub const RAINBOW: [Color; 6] = [
    Color::Red,
    Color::LightYellow,
    Color::Green,
    Color::Cyan,
    Color::Blue,
    Color::Magenta,
];

pub fn get_rainbow_style(index: usize) -> Style {
    Style::default().fg(RAINBOW[index % RAINBOW.len()])
}