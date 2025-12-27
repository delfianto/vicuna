use crate::api::modelfile;
use crate::app::App;
use crate::tui::styles;
use crate::utils::vram;
use ratatui::{
    Frame,
    layout::{Constraint, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, Cell, Row, Table, TableState},
};

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let header_cells = [
        "Name", "Family", "Size", "Quant", "Params", "VRAM", "Modified",
    ]
    .iter()
    .map(|h| Cell::from(*h).style(Style::default().add_modifier(Modifier::BOLD)));
    let header = Row::new(header_cells).height(1).bottom_margin(1);

    let rows = app.models.iter().map(|model| {
        let details = model.details.as_ref();
        let family = details.map(|d| d.family.as_str()).unwrap_or("?");
        let size_str = vram::format_size(model.size);
        let quant_str = details
            .map(|d| d.quantization_level.as_str())
            .unwrap_or("?");
        let params_str = details.map(|d| d.parameter_size.as_str()).unwrap_or("?");

        let vram_est = if let (Some(p), Some(q)) = (
            modelfile::parse_parameter_size(params_str),
            modelfile::parse_quantization_bits(quant_str),
        ) {
            vram::estimate_vram_usage(p, q)
        } else {
            0
        };
        let vram_str = if vram_est > 0 {
            vram::format_size(vram_est)
        } else {
            "?".to_string()
        };

        let cells = vec![
            Cell::from(model.name.as_str()),
            Cell::from(family),
            Cell::from(size_str),
            Cell::from(quant_str),
            Cell::from(params_str),
            Cell::from(vram_str),
            Cell::from(model.modified_at.as_str()),
        ];
        Row::new(cells).height(1)
    });

    let t = Table::new(
        rows,
        [
            Constraint::Percentage(20),
            Constraint::Percentage(10),
            Constraint::Percentage(10),
            Constraint::Percentage(10),
            Constraint::Percentage(10),
            Constraint::Percentage(10),
            Constraint::Percentage(30),
        ],
    )
    .header(header)
    .block(Block::default().borders(Borders::ALL).title("Models"))
    .row_highlight_style(styles::HIGHLIGHT_STYLE);

    let mut state = TableState::default();
    state.select(Some(app.selected_model_index));

    f.render_stateful_widget(t, area, &mut state);
}
