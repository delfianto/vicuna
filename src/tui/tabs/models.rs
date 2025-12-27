use crate::api::modelfile;
use crate::app::App;
use crate::tui::styles;
use crate::utils::vram;
use chrono::DateTime;
use ratatui::{
    Frame,
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Row, Table, TableState},
};

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let header_style = Style::default()
        .fg(Color::LightCyan)
        .add_modifier(Modifier::BOLD);
    let header_cells = [
        "#", "Name", "Family", "Size", "Quant", "Params", "VRAM", "Modified",
    ]
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

        let date_str = if let Ok(dt) = DateTime::parse_from_rfc3339(&model.modified_at) {
            dt.format("%Y-%m-%d %H:%M:%S").to_string()
        } else {
            model.modified_at.clone()
        };

        let display_name = modelfile::sanitize_model_name(&model.name).to_lowercase();

        let cells = vec![
            Cell::from((i + 1).to_string()).style(Style::default().fg(Color::DarkGray)),
            Cell::from(display_name).style(Style::default().fg(Color::Magenta)),
            Cell::from(family).style(Style::default().fg(Color::Cyan)),
            Cell::from(size_str).style(Style::default().fg(Color::Green)),
            Cell::from(quant_str).style(Style::default().fg(Color::Yellow)),
            Cell::from(params_str).style(Style::default().fg(Color::LightRed)),
            Cell::from(vram_str).style(Style::default().fg(Color::LightBlue)),
            Cell::from(date_str).style(Style::default().fg(Color::DarkGray)),
        ];
        Row::new(cells).height(1)
    });

    let t = Table::new(
        rows,
        [
            Constraint::Length(4),
            Constraint::Percentage(26),
            Constraint::Percentage(10),
            Constraint::Percentage(10),
            Constraint::Percentage(10),
            Constraint::Percentage(10),
            Constraint::Percentage(10),
            Constraint::Percentage(20),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" Models (Sorted by {:?}) ", app.sort_column))
            .border_style(Style::default().fg(Color::LightMagenta)),
    )
    .row_highlight_style(styles::HIGHLIGHT_STYLE);

    let mut state = TableState::default();
    state.select(Some(app.selected_model_index));

    f.render_stateful_widget(t, area, &mut state);
}