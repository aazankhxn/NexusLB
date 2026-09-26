use crate::app::App;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Sparkline, Table},
    Frame,
};

pub fn render(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4), // Header & Global Stats
            Constraint::Length(7), // Sparklines (QPS & Latency)
            Constraint::Min(8),    // Backends Table
            Constraint::Length(3), // Status message & Keybindings bar
        ])
        .split(frame.area());

    render_header(frame, app, chunks[0]);
    render_sparklines(frame, app, chunks[1]);
    render_backends_table(frame, app, chunks[2]);
    render_footer(frame, app, chunks[3]);
}

fn render_header(frame: &mut Frame, app: &App, area: Rect) {
    let mut total_reqs = 0;
    let mut total_errors = 0;
    let mut active_conns = 0;

    for b in &app.backends {
        total_reqs += b.stats.total_requests;
        total_errors += b.stats.total_errors;
        active_conns += b.stats.active_connections;
    }

    let error_pct = if total_reqs > 0 {
        (total_errors as f64 / total_reqs as f64) * 100.0
    } else {
        0.0
    };

    let title_line = Line::from(vec![
        Span::styled(
            "⚡ NEXUSLB ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("ADAPTIVE LOAD BALANCER ", Style::default().fg(Color::White)),
        Span::styled("| ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("Target: {} ", app.admin_url),
            Style::default().fg(Color::Yellow),
        ),
        Span::styled("| ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("Nodes: {} ", app.backends.len()),
            Style::default().fg(Color::Green),
        ),
        Span::styled("| ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("Active Conns: {} ", active_conns),
            Style::default().fg(Color::LightBlue),
        ),
        Span::styled("| ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("Total Reqs: {} ", total_reqs),
            Style::default().fg(Color::White),
        ),
        Span::styled("| ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("Error Rate: {:.2}%", error_pct),
            Style::default().fg(if error_pct > 1.0 {
                Color::Red
            } else {
                Color::Green
            }),
        ),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" System Overview ");

    let paragraph = Paragraph::new(title_line).block(block);
    frame.render_widget(paragraph, area);
}

fn render_sparklines(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // Throughput Sparkline
    let throughput_data: Vec<u64> = app.throughput_history.iter().copied().collect();
    let current_qps = throughput_data.last().copied().unwrap_or(0);
    let qps_sparkline = Sparkline::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" Live Throughput ({} req/sec) ", current_qps))
                .border_style(Style::default().fg(Color::Green)),
        )
        .data(&throughput_data)
        .style(Style::default().fg(Color::Green));
    frame.render_widget(qps_sparkline, chunks[0]);

    // Latency Sparkline
    let latency_data: Vec<u64> = app.latency_history.iter().copied().collect();
    let current_latency = latency_data.last().copied().unwrap_or(0);
    let latency_sparkline = Sparkline::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" EWMA Latency ({} µs) ", current_latency))
                .border_style(Style::default().fg(Color::Yellow)),
        )
        .data(&latency_data)
        .style(Style::default().fg(Color::Yellow));
    frame.render_widget(latency_sparkline, chunks[1]);
}

fn render_backends_table(frame: &mut Frame, app: &App, area: Rect) {
    let header_cells = [
        "ID",
        "Name",
        "Address",
        "Weight",
        "State",
        "Circuit",
        "Active",
        "Requests",
        "Avg (µs)",
        "EWMA (µs)",
        "Min (µs)",
        "Max (µs)",
        "Errors",
    ]
    .into_iter()
    .map(|h| {
        Cell::from(h).style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
    });

    let header = Row::new(header_cells).height(1).bottom_margin(1);

    let rows = app.backends.iter().enumerate().map(|(idx, b)| {
        let is_selected = idx == app.selected_index;
        let base_style = if is_selected {
            Style::default()
                .bg(Color::Rgb(30, 45, 60))
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };

        let state_color = match b.state.to_lowercase().as_str() {
            "up" => Color::Green,
            "draining" => Color::Yellow,
            _ => Color::Red,
        };

        let circuit_color = match b.circuit.to_lowercase().as_str() {
            "closed" => Color::Green,
            "halfopen" | "half-open" => Color::Yellow,
            _ => Color::Red,
        };

        let cells = vec![
            Cell::from(b.id.to_string()),
            Cell::from(b.name.clone()),
            Cell::from(b.address.clone()),
            Cell::from(b.weight.to_string()),
            Cell::from(b.state.to_uppercase()).style(Style::default().fg(state_color)),
            Cell::from(b.circuit.to_uppercase()).style(Style::default().fg(circuit_color)),
            Cell::from(b.stats.active_connections.to_string()),
            Cell::from(b.stats.total_requests.to_string()),
            Cell::from(b.stats.avg_latency_micros.to_string()),
            Cell::from(b.stats.ewma_latency_micros.to_string()),
            Cell::from(if b.stats.min_latency_micros == u64::MAX {
                "0".to_string()
            } else {
                b.stats.min_latency_micros.to_string()
            }),
            Cell::from(b.stats.max_latency_micros.to_string()),
            Cell::from(b.stats.total_errors.to_string()).style(if b.stats.total_errors > 0 {
                Style::default().fg(Color::Red)
            } else {
                Style::default()
            }),
        ];

        Row::new(cells).style(base_style).height(1)
    });

    let widths = [
        Constraint::Length(4),
        Constraint::Percentage(16),
        Constraint::Percentage(16),
        Constraint::Length(8),
        Constraint::Length(10),
        Constraint::Length(10),
        Constraint::Length(8),
        Constraint::Length(10),
        Constraint::Length(10),
        Constraint::Length(11),
        Constraint::Length(10),
        Constraint::Length(10),
        Constraint::Length(8),
    ];

    let table = Table::new(rows, widths).header(header).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Registered Upstream Backends ")
            .border_style(Style::default().fg(Color::White)),
    );

    frame.render_widget(table, area);
}

fn render_footer(frame: &mut Frame, app: &App, area: Rect) {
    let footer_line = Line::from(vec![
        Span::styled(
            " [q/Esc] ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" Quit  "),
        Span::styled(
            " [r] ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" Hot Reload  "),
        Span::styled(
            " [d] ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" Toggle Drain  "),
        Span::styled(
            " [↑/↓] ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" Select Backend  | Status: "),
        Span::styled(&app.status_message, Style::default().fg(Color::LightGreen)),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    let paragraph = Paragraph::new(footer_line).block(block);
    frame.render_widget(paragraph, area);
}
