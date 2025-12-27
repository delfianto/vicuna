use crate::api::modelfile;
use crate::app::{App, ModelsFocus};
use crate::tui::styles;
use crate::utils::vram;
use chrono::DateTime;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, BorderType, Borders, Cell, Paragraph, Row, Table, TableState, Wrap},
    Frame,
};

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let main_area = if app.show_info {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);
        draw_info_pane(f, app, chunks[1]);
        chunks[0]
    } else {
        area
    };

    let header_style = Style::default()
        .fg(Color::LightCyan)
        .add_modifier(Modifier::BOLD);

    let mut header_titles = vec!["#", "Name", "Family", "Size", "Quant", "Params", "VRAM"];
    if !app.show_info {
        header_titles.push("Modified");
    }

    let header_cells = header_titles
        .iter()
        .map(|h| Cell::from(*h).style(header_style));
    let header = Row::new(header_cells).height(1).bottom_margin(1);

    let rows = app.models.iter().enumerate().map(|(i, model)| {
        let details = model.details.as_ref();
        let family = details.map(|d| d.family.as_str()).unwrap_or("?");
        let size_str = vram::format_size(model.size);

        let mut quant_str = details
            .map(|d| d.quantization_level.clone())
            .unwrap_or_else(|| "?".to_string());

        let params_str = details.map(|d| d.parameter_size.as_str()).unwrap_or("?");

        let mut q_bits = modelfile::parse_quantization_bits(&quant_str);

        if (q_bits.is_none() || quant_str == "unknown")
            && let Some(bits) = modelfile::parse_quantization_bits(&model.name)
        {
            q_bits = Some(bits);
            let upper_name = model.name.to_uppercase();
            if let Some(pos) = upper_name.find(':') {
                let tag = &upper_name[pos + 1..];
                if tag.contains('Q') || tag.contains("MXFP") || tag.contains("IQ") {
                    quant_str = tag.to_string();
                } else if let Some(q_pos) = upper_name.find("-Q") {
                    quant_str = model.name[q_pos + 1..].to_string();
                }
            } else if upper_name.contains("Q4_K_M") {
                quant_str = "Q4_K_M".to_string();
            }
        }

        let vram_est =
            if let (Some(p), Some(q)) = (modelfile::parse_parameter_size(params_str), q_bits) {
                vram::estimate_vram_usage(p, q)
            } else {
                0
            };
        let vram_str = if vram_est > 0 {
            vram::format_size(vram_est)
        } else {
            "?".to_string()
        };

        let display_name = modelfile::sanitize_model_name(&model.name).to_lowercase();

        let mut cells = vec![
            Cell::from((i + 1).to_string()).style(Style::default().fg(Color::DarkGray)),
            Cell::from(display_name).style(Style::default().fg(Color::Magenta)),
            Cell::from(family).style(Style::default().fg(Color::Cyan)),
            Cell::from(size_str).style(Style::default().fg(Color::Green)),
            Cell::from(quant_str).style(Style::default().fg(Color::Yellow)),
            Cell::from(params_str).style(Style::default().fg(Color::LightRed)),
            Cell::from(vram_str).style(Style::default().fg(Color::LightBlue)),
        ];

        if !app.show_info {
            let date_str = if let Ok(dt) = DateTime::parse_from_rfc3339(&model.modified_at) {
                dt.format("%Y-%m-%d %H:%M:%S").to_string()
            } else {
                model.modified_at.clone()
            };
            cells.push(Cell::from(date_str).style(Style::default().fg(Color::DarkGray)));
        }

        Row::new(cells).height(1)
    });

    let mut constraints = vec![
        Constraint::Length(4),
        Constraint::Percentage(26),
        Constraint::Percentage(10),
        Constraint::Percentage(10),
        Constraint::Percentage(10),
        Constraint::Percentage(10),
        Constraint::Percentage(10),
    ];
    if !app.show_info {
        constraints.push(Constraint::Percentage(20));
    }

    let (border_style, border_type) = if app.models_focus == ModelsFocus::List {
        (
            Style::default()
                .fg(Color::LightYellow)
                .add_modifier(Modifier::BOLD),
            BorderType::Thick,
        )
    } else {
        (Style::default().fg(Color::LightMagenta), BorderType::Plain)
    };

    let t = Table::new(rows, constraints)
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(border_type)
                .title(format!(" Models (Sorted by {:?}) ", app.sort_column))
                .border_style(border_style),
        )
        .row_highlight_style(styles::HIGHLIGHT_STYLE);

    let mut state = TableState::default();
    state.select(Some(app.selected_model_index));

    f.render_stateful_widget(t, main_area, &mut state);
}

fn draw_info_pane(f: &mut Frame, app: &App, area: Rect) {
    let (border_style, border_type) = if app.models_focus == ModelsFocus::Info {
        (
            Style::default()
                .fg(Color::LightYellow)
                .add_modifier(Modifier::BOLD),
            BorderType::Thick,
        )
    } else {
        let border_color = styles::RAINBOW[app.selected_model_index % styles::RAINBOW.len()];
        (Style::default().fg(border_color), BorderType::Rounded)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(border_type)
        .border_style(border_style)
        .title(" Model Details ");

    if let Some(ref info) = app.model_info {
        let mut details = String::new();

        if let Some(ref template) = info.template {
            details.push_str("# Template\n");
            details.push_str(template);
            details.push_str("\n\n");
        }

        if let Some(ref parameters) = info.parameters {
            details.push_str("# Parameters\n");
            details.push_str(parameters);
            details.push_str("\n\n");
        }

        if let Some(ref license) = info.license {
            details.push_str("# License\n");
            details.push_str(license);
        }

        let inner_area = block.inner(area);
        
        // Simple height estimation for clamping (wrapped lines)
        let mut visual_lines = 0;
        for line in details.lines() {
            let len = line.chars().count() as u16;
            if len == 0 {
                visual_lines += 1;
            } else {
                visual_lines += len.div_ceil(inner_area.width);
            }
        }

        let max_scroll = visual_lines.saturating_sub(inner_area.height);
        let scroll = app.info_scroll.min(max_scroll);

        let p = Paragraph::new(details)
            .block(block)
            .wrap(Wrap { trim: false })
            .scroll((scroll, 0));

        f.render_widget(p, area);

        // Render scrollbar
        if visual_lines > inner_area.height {
            let scrollbar = ratatui::widgets::Scrollbar::new(ratatui::widgets::ScrollbarOrientation::VerticalRight)
                .symbols(ratatui::symbols::scrollbar::VERTICAL)
                .begin_symbol(Some("▲"))
                .end_symbol(Some("▼"));
            
            let mut scrollbar_state = ratatui::widgets::ScrollbarState::new(usize::from(visual_lines))
                .position(usize::from(scroll));
            
            f.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
        }
    } else {
        f.render_widget(Paragraph::new("Loading...").block(block), area);
    }
}