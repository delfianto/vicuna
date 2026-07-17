//! Shared visual language for Vicuna.
//!
//! Design intent: one warm accent, quiet chrome, loud focus.
//! Personality without rainbow chaos.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Padding},
};

/// Gap between sibling panes (sessions | chat, table | details, etc.).
pub const GAP: u16 = 1;
/// Outer inset on top/sides only — bottom flush with the control bars.
pub const OUTER_PAD: u16 = 1;

// ── Palette ──────────────────────────────────────────────────────────────────

/// Deep background used on bars and overlays.
pub const BG_DEEP: Color = Color::Rgb(18, 18, 22);
/// Slightly lifted surface (panels / composer idle).
pub const BG_SURFACE: Color = Color::Rgb(28, 28, 34);
/// Selection / highlight wash.
pub const BG_SELECT: Color = Color::Rgb(48, 42, 36);
/// Control-bar fill — a step brighter so the chrome reads as intentional.
pub const BG_BAR: Color = Color::Rgb(32, 30, 36);

/// Primary accent — warm amber (focus, active, key caps).
pub const ACCENT: Color = Color::Rgb(232, 168, 84);
/// Soft accent for secondary emphasis.
#[allow(dead_code)]
pub const ACCENT_DIM: Color = Color::Rgb(160, 120, 70);

pub const TEXT: Color = Color::Rgb(230, 228, 224);
pub const TEXT_MUTED: Color = Color::Rgb(140, 138, 134);
pub const TEXT_DIM: Color = Color::Rgb(80, 78, 76);

pub const BORDER_IDLE: Color = Color::Rgb(55, 55, 62);
pub const BORDER_FOCUS: Color = ACCENT;

pub const OK: Color = Color::Rgb(120, 190, 140);
#[allow(dead_code)]
pub const WARN: Color = Color::Rgb(230, 180, 80);
pub const ERR: Color = Color::Rgb(220, 100, 100);

pub const ROLE_USER: Color = Color::Rgb(130, 180, 230);
pub const ROLE_ASSISTANT: Color = Color::Rgb(180, 150, 220);
#[allow(dead_code)]
pub const ROLE_SYSTEM: Color = Color::Rgb(140, 160, 140);

// ── Styles ───────────────────────────────────────────────────────────────────

pub const HIGHLIGHT_STYLE: Style = Style::new()
    .fg(ACCENT)
    .bg(BG_SELECT)
    .add_modifier(Modifier::BOLD);

pub fn text() -> Style {
    Style::default().fg(TEXT)
}

pub fn muted() -> Style {
    Style::default().fg(TEXT_MUTED)
}

pub fn dim() -> Style {
    Style::default().fg(TEXT_DIM)
}

pub fn accent() -> Style {
    Style::default().fg(ACCENT)
}

pub fn accent_bold() -> Style {
    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
}

pub fn title_style(focused: bool) -> Style {
    if focused {
        Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(TEXT_MUTED)
    }
}

/// Pane chrome: rounded always; thick + amber when focused, plain gray when not.
pub fn pane_block<'a>(title: impl Into<String>, focused: bool) -> Block<'a> {
    let title = title.into();
    let (border_style, border_type) = if focused {
        (
            Style::default()
                .fg(BORDER_FOCUS)
                .add_modifier(Modifier::BOLD),
            BorderType::Thick,
        )
    } else {
        (Style::default().fg(BORDER_IDLE), BorderType::Rounded)
    };

    Block::default()
        .borders(Borders::ALL)
        .border_type(border_type)
        .border_style(border_style)
        .padding(Padding::horizontal(1))
        .title(Span::styled(format!(" {title} "), title_style(focused)))
}

/// Inset top + sides only; bottom stays flush so control bars sit on the edge.
pub fn outer_area(area: Rect) -> Rect {
    let pad = OUTER_PAD;
    if area.width <= pad * 2 || area.height <= pad {
        return area;
    }
    Rect {
        x: area.x + pad,
        y: area.y + pad,
        width: area.width - pad * 2,
        height: area.height - pad,
    }
}

/// Horizontal split with a gutter between panes.
pub fn split_horizontal(area: Rect, left: Constraint, right: Constraint) -> (Rect, Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .spacing(GAP)
        .constraints([left, right])
        .split(area);
    (chunks[0], chunks[1])
}

/// Vertical stack with a gutter between panes.
pub fn split_vertical(area: Rect, top: Constraint, bottom: Constraint) -> (Rect, Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .spacing(GAP)
        .constraints([top, bottom])
        .split(area);
    (chunks[0], chunks[1])
}

#[allow(dead_code)]
pub fn panel_block<'a>(title: impl Into<String>) -> Block<'a> {
    pane_block(title, false)
}

#[allow(dead_code)]
pub fn role_style(role: &str) -> Style {
    let fg = match role.to_ascii_lowercase().as_str() {
        "user" | "you" => ROLE_USER,
        "assistant" | "model" => ROLE_ASSISTANT,
        "system" => ROLE_SYSTEM,
        _ => TEXT_MUTED,
    };
    Style::default().fg(fg).add_modifier(Modifier::BOLD)
}

pub fn role_label(role: &str) -> &'static str {
    match role.to_ascii_lowercase().as_str() {
        "user" => "you",
        "assistant" => "assistant",
        "system" => "system",
        _ => "message",
    }
}

/// Chrome bar background (single bottom line).
pub fn bar_bg() -> Style {
    Style::default().bg(BG_BAR).fg(TEXT)
}

/// Active tab pill on the status bar.
pub fn tab_active() -> Style {
    Style::default()
        .fg(BG_DEEP)
        .bg(ACCENT)
        .add_modifier(Modifier::BOLD)
}

/// Inactive tab pill.
pub fn tab_idle() -> Style {
    Style::default().fg(TEXT_MUTED).bg(BG_SURFACE)
}

/// Compact key cap — amber text, no heavy fill (less visual noise).
pub fn key_cap() -> Style {
    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
}

pub fn key_desc() -> Style {
    Style::default().fg(TEXT_MUTED)
}

/// Build a help line that fits `max_width` cells (drops trailing bindings if needed).
pub fn help_line<'a>(keys: &[(&'a str, &'a str)], max_width: u16) -> Line<'a> {
    let max = max_width as usize;
    let mut spans = Vec::new();
    let mut used = 0usize;

    for (i, (key, desc)) in keys.iter().enumerate() {
        // "  " sep + "<key> " + desc
        let sep = if i == 0 { 0 } else { 2 };
        let piece = format!("<{key}> {desc}");
        let piece_w = piece.chars().count();
        if used + sep + piece_w > max {
            if spans.is_empty() {
                // Always show at least one binding, truncated.
                let keep = max.saturating_sub(1);
                let mut s: String = piece.chars().take(keep).collect();
                if s.chars().count() < piece_w {
                    s.push('…');
                }
                spans.push(Span::styled(s, key_cap()));
            }
            break;
        }
        if i > 0 {
            spans.push(Span::styled("  ", dim()));
            used += 2;
        }
        spans.push(Span::styled(format!("<{key}>"), key_cap()));
        spans.push(Span::styled(format!(" {desc}"), key_desc()));
        used += piece_w;
    }
    Line::from(spans)
}

/// Shorten a model id for the status bar: basename, keep `:tag` when possible.
pub fn short_model(name: &str, max: usize) -> String {
    if max == 0 {
        return String::new();
    }
    let base = name.rsplit('/').next().unwrap_or(name);
    let n = base.chars().count();
    if n <= max {
        return base.to_string();
    }
    if let Some((head, tag)) = base.rsplit_once(':') {
        let tag_part = format!(":{tag}");
        let tag_len = tag_part.chars().count();
        if tag_len + 2 >= max {
            return ellipsize(base, max);
        }
        let head_budget = max.saturating_sub(tag_len + 1); // …
        let head_short = ellipsize(head, head_budget);
        // ellipsize may already end with …; avoid double
        if head_short.ends_with('…') {
            format!("{head_short}{tag_part}")
        } else {
            format!("{head_short}…{tag_part}")
        }
    } else {
        ellipsize(base, max)
    }
}

fn ellipsize(s: &str, max: usize) -> String {
    let n = s.chars().count();
    if n <= max {
        return s.to_string();
    }
    if max <= 1 {
        return "…".to_string();
    }
    let mut out: String = s.chars().take(max - 1).collect();
    out.push('…');
    out
}

/// Sort indicator for table headers.
pub fn sort_mark(active: bool, _asc: bool) -> &'static str {
    if active { "▾" } else { " " }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_model_keeps_tag() {
        let long = "hf.co/HauhauCS/Gemma4-26B-A4B-QAT-Uncensored-HauhauCS-Balanced-MTP:Q4_K_M";
        let s = short_model(long, 28);
        assert!(s.ends_with(":Q4_K_M"), "{s}");
        assert!(s.chars().count() <= 28, "{s}");
        assert!(!s.contains("hf.co/"));
    }

    #[test]
    fn short_model_short_names_unchanged() {
        assert_eq!(short_model("llama3.2:latest", 40), "llama3.2:latest");
    }
}
