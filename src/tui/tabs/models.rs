use crate::api::modelfile;
use crate::api::types::ShowModelResponse;
use crate::app::{App, ModelsFocus, SortColumn};
use crate::tui::components::markdown::MarkdownViewer;
use crate::tui::styles;
use crate::utils::vram;
use chrono::DateTime;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Cell, Paragraph, Row, Table, TableState},
};

pub fn draw(f: &mut Frame, app: &mut App, area: Rect) {
    let table_area = if app.show_info {
        let (table_area, info_area) = styles::split_horizontal(
            area,
            Constraint::Percentage(52),
            Constraint::Percentage(48),
        );
        app.hits.models_info = Some(info_area);
        draw_inspect_pane(f, app, info_area);
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
    let title = if app.show_info {
        format!("models · {n} · f3/esc close inspect")
    } else if n == 0 {
        "models".to_string()
    } else {
        format!("models · {n} · f3 inspect")
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

    if app.models.is_empty() {
        let hint = Paragraph::new(vec![
            Line::from(Span::styled("no models yet", styles::accent_bold())),
            Line::from(""),
            Line::from(vec![
                Span::styled(" p ", styles::key_cap()),
                Span::styled(" pull a model", styles::key_desc()),
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

fn draw_inspect_pane(f: &mut Frame, app: &mut App, area: Rect) {
    let focused = app.models_focus == ModelsFocus::Info;
    let model = app.models.get(app.selected_model_index);
    let model_name = model.map(|m| m.name.as_str()).unwrap_or("—");
    let short = styles::short_model(model_name, 36);

    let title = if focused {
        format!("inspect · {short} · ↑↓ scroll · f3/esc close")
    } else {
        format!("inspect · {short} · tab focus · f3 close")
    };

    let block = styles::pane_block(title, focused);
    let inner = block.inner(area);

    let body = if let Some(ref info) = app.model_info {
        format_inspect(model, info)
    } else {
        "### loading\n\nFetching template, parameters, modelfile, and license from ollama…".into()
    };

    let max_scroll = MarkdownViewer::max_scroll_for(&body, inner.width, inner.height);
    app.info_max_scroll = max_scroll;
    app.info_scroll = app.info_scroll.min(max_scroll);

    let viewer = MarkdownViewer::new(&body)
        .block(block)
        .scroll(app.info_scroll)
        .follow(false);

    f.render_widget(viewer, area);
}

fn format_inspect(
    model: Option<&crate::api::types::Model>,
    info: &ShowModelResponse,
) -> String {
    let mut out = String::new();

    out.push_str("## summary\n\n");
    if let Some(m) = model {
        out.push_str(&format!("**name**  {}\n", m.name));
        out.push_str(&format!("**size**  {}\n", vram::format_size(m.size)));
        if let Ok(dt) = DateTime::parse_from_rfc3339(&m.modified_at) {
            out.push_str(&format!("**modified**  {}\n", dt.format("%Y-%m-%d %H:%M")));
        }
        if let Some(d) = m.details.as_ref() {
            out.push_str(&format!("**family**  {}\n", d.family));
            out.push_str(&format!("**params**  {}\n", d.parameter_size));
            out.push_str(&format!("**quant**  {}\n", d.quantization_level));
            out.push_str(&format!("**format**  {}\n", d.format));
        }
    }

    if let Some(d) = info.details.as_ref() {
        out.push('\n');
        out.push_str("## ollama details\n\n");
        out.push_str(&format!("**family**  {}\n", d.family));
        out.push_str(&format!("**params**  {}\n", d.parameter_size));
        out.push_str(&format!("**quant**  {}\n", d.quantization_level));
        out.push_str(&format!("**format**  {}\n", d.format));
        if let Some(ref fams) = d.families {
            if !fams.is_empty() {
                out.push_str(&format!("**families**  {}\n", fams.join(", ")));
            }
        }
    }

    if let Some(ref parameters) = info.parameters {
        let t = parameters.trim();
        if !t.is_empty() {
            out.push_str("\n## parameters\n\n");
            out.push_str(t);
            out.push('\n');
        }
    }

    if let Some(ref template) = info.template {
        let t = template.trim();
        if !t.is_empty() {
            out.push_str("\n## template\n\n```\n");
            out.push_str(t);
            out.push_str("\n```\n");
        }
    }

    if let Some(ref modelfile) = info.modelfile {
        let t = modelfile.trim();
        if !t.is_empty() {
            out.push_str("\n## modelfile\n\n```\n");
            out.push_str(t);
            out.push_str("\n```\n");
        }
    }

    if let Some(ref license) = info.license {
        let t = license.trim();
        if !t.is_empty() {
            out.push_str("\n## license\n\n");
            // License blobs can be huge — keep a usable preview.
            let chars: Vec<char> = t.chars().collect();
            if chars.len() > 4000 {
                out.extend(chars.into_iter().take(4000));
                out.push_str("\n\n… (truncated)\n");
            } else {
                out.push_str(t);
                out.push('\n');
            }
        }
    }

    if out.trim().is_empty() {
        out = "no metadata returned by ollama for this model\n".into();
    }

    out
}
