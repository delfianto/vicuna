use crate::api::modelfile;
use crate::app::{App, ModelsFocus, SortColumn};
use crate::tui::styles;
use crate::utils::vram;
use chrono::DateTime;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Cell, Paragraph, Row, Table, TableState, Wrap},
};

pub fn draw(f: &mut Frame, app: &mut App, area: Rect) {
    let table_area = if app.show_info {
        let (table_area, info_area) = styles::split_horizontal(
            area,
            Constraint::Percentage(55),
            Constraint::Percentage(45),
        );
        app.hits.models_info = Some(info_area);
        draw_info_pane(f, app, info_area);
        table_area
    } else {
        app.hits.models_info = None;
        area
    };

    app.hits.models_list = Some(table_area);
    app.hits.sessions = None;
    app.hits.messages = None;
    app.hits.composer = None;

    draw_table(f, app, table_area);
}

fn header_cell(label: &str, sort: SortColumn, active: SortColumn) -> Cell<'_> {
    let mark = styles::sort_mark(sort == active, true);
    let text = format!("{label}{mark}");
    let style = if sort == active {
        Style::default()
            .fg(styles::ACCENT)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .fg(styles::TEXT_MUTED)
            .add_modifier(Modifier::BOLD)
    };
    Cell::from(text).style(style)
}

fn draw_table(f: &mut Frame, app: &App, table_area: Rect) {
    let list_focused = !app.show_info || app.models_focus == ModelsFocus::List;

    let mut header_cells = vec![
        Cell::from("#").style(styles::muted().add_modifier(Modifier::BOLD)),
        header_cell("name", SortColumn::Name, app.sort_column),
        Cell::from("family").style(styles::muted().add_modifier(Modifier::BOLD)),
        header_cell("size", SortColumn::Size, app.sort_column),
        Cell::from("quant").style(styles::muted().add_modifier(Modifier::BOLD)),
        Cell::from("params").style(styles::muted().add_modifier(Modifier::BOLD)),
        Cell::from("vram≈").style(styles::muted().add_modifier(Modifier::BOLD)),
    ];
    if !app.show_info {
        header_cells.push(header_cell("modified", SortColumn::Modified, app.sort_column));
    }
    let header = Row::new(header_cells).height(1).bottom_margin(0);

    let rows = app.models.iter().enumerate().map(|(i, model)| {
        let details = model.details.as_ref();
        let family = details.map(|d| d.family.as_str()).unwrap_or("—");
        let size_str = vram::format_size(model.size);

        let mut quant_str = details
            .map(|d| d.quantization_level.clone())
            .unwrap_or_else(|| "—".to_string());

        let params_str = details.map(|d| d.parameter_size.as_str()).unwrap_or("—");

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
            "—".to_string()
        };

        let display_name = modelfile::sanitize_model_name(&model.name).to_lowercase();

        let mut cells = vec![
            Cell::from((i + 1).to_string()).style(styles::dim()),
            Cell::from(display_name).style(styles::text()),
            Cell::from(family).style(styles::muted()),
            Cell::from(size_str).style(Style::default().fg(styles::OK)),
            Cell::from(quant_str).style(styles::accent()),
            Cell::from(params_str).style(styles::muted()),
            Cell::from(vram_str).style(styles::muted()),
        ];

        if !app.show_info {
            let date_str = if let Ok(dt) = DateTime::parse_from_rfc3339(&model.modified_at) {
                dt.format("%Y-%m-%d %H:%M").to_string()
            } else {
                model.modified_at.clone()
            };
            cells.push(Cell::from(date_str).style(styles::dim()));
        }

        Row::new(cells).height(1)
    });

    let mut constraints = vec![
        Constraint::Length(3),
        Constraint::Percentage(28),
        Constraint::Percentage(12),
        Constraint::Percentage(10),
        Constraint::Percentage(12),
        Constraint::Percentage(10),
        Constraint::Percentage(10),
    ];
    if !app.show_info {
        constraints.push(Constraint::Percentage(14));
    }

    let n = app.models.len();
    let title = if n == 0 {
        "models".to_string()
    } else {
        format!("models · {n}")
    };

    let t = Table::new(rows, constraints)
        .header(header)
        .block(styles::pane_block(title, list_focused))
        .row_highlight_style(styles::HIGHLIGHT_STYLE)
        .column_spacing(2);

    let mut state = TableState::default();
    if !app.models.is_empty() {
        state.select(Some(app.selected_model_index));
    }

    f.render_stateful_widget(t, table_area, &mut state);

    // Empty library hint
    if app.models.is_empty() {
        let hint = Paragraph::new(vec![
            Line::from(Span::styled("no models yet", styles::accent_bold())),
            Line::from(""),
            Line::from(vec![
                Span::styled("[p] ", styles::key_cap()),
                Span::styled("pull something like llama3.2", styles::key_desc()),
            ]),
        ])
        .alignment(ratatui::layout::HorizontalAlignment::Center);
        let inner = styles::pane_block("", false).inner(table_area);
        let mid = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(1),
                Constraint::Length(3),
                Constraint::Min(1),
            ])
            .split(inner);
        f.render_widget(hint, mid[1]);
    }
}

fn draw_info_pane(f: &mut Frame, app: &App, area: Rect) {
    let focused = app.models_focus == ModelsFocus::Info;
    let model_name = app
        .models
        .get(app.selected_model_index)
        .map(|m| m.name.as_str())
        .unwrap_or("details");

    let block = styles::pane_block(format!("details · {model_name}"), focused);

    if let Some(ref info) = app.model_info {
        let mut details = String::new();

        if let Some(ref template) = info.template {
            details.push_str("## template\n\n");
            details.push_str(template);
            details.push_str("\n\n");
        }

        if let Some(ref parameters) = info.parameters {
            details.push_str("## parameters\n\n");
            details.push_str(parameters);
            details.push_str("\n\n");
        }

        if let Some(ref license) = info.license {
            details.push_str("## license\n\n");
            details.push_str(license);
        }

        if details.trim().is_empty() {
            details = "_no extra metadata from ollama_".into();
        }

        let inner_area = block.inner(area);

        let mut visual_lines = 0u16;
        for line in details.lines() {
            let len = line.chars().count() as u16;
            if len == 0 {
                visual_lines += 1;
            } else {
                visual_lines += len.div_ceil(inner_area.width.max(1));
            }
        }

        let max_scroll = visual_lines.saturating_sub(inner_area.height);
        let scroll = app.info_scroll.get().min(max_scroll);
        app.info_scroll.set(scroll);

        let p = Paragraph::new(details)
            .block(block)
            .style(styles::text())
            .wrap(Wrap { trim: false })
            .scroll((scroll, 0));

        f.render_widget(p, area);

        if visual_lines > inner_area.height {
            let scrollbar = ratatui::widgets::Scrollbar::new(
                ratatui::widgets::ScrollbarOrientation::VerticalRight,
            )
            .symbols(ratatui::symbols::scrollbar::VERTICAL)
            .begin_symbol(Some("▲"))
            .end_symbol(Some("▼"));

            let mut scrollbar_state =
                ratatui::widgets::ScrollbarState::new(usize::from(visual_lines))
                    .position(usize::from(scroll));

            f.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
        }
    } else {
        let p = Paragraph::new(Line::from(Span::styled(
            "loading…",
            styles::muted().add_modifier(Modifier::ITALIC),
        )))
        .block(block);
        f.render_widget(p, area);
    }
}
