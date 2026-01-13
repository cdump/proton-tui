use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};

use crate::app::{App, DisplayItem, InputMode, SplitFocus};
use crate::countries;

const SPLIT_COUNTRY_PERCENT: u16 = 35;
const SPLIT_SERVER_PERCENT: u16 = 65;

pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

pub fn ui(frame: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Length(3), // Search bar
                Constraint::Min(0),    // List
                Constraint::Length(3), // Status
                Constraint::Length(1), // Keybinding hints
            ]
            .as_ref(),
        )
        .split(frame.size());

    // --- Search Bar ---
    render_search_bar(frame, app, chunks[0]);

    // --- List (Tree View or Split View) ---
    if app.split_view {
        render_split_view(frame, app, chunks[1]);
    } else {
        render_tree_view(frame, app, chunks[1]);
    }

    // --- Footer (Status) ---
    render_status_bar(frame, app, chunks[2]);

    // --- Keybinding Hints Bar ---
    render_hints_bar(frame, app, chunks[3]);

    // --- Popups ---
    if app.show_connection_popup {
        render_connection_popup(frame, app);
    }

    if app.show_help_popup {
        render_help_popup(frame);
    }
}

fn render_search_bar(frame: &mut Frame, app: &App, area: Rect) {
    let search_style = match app.input_mode {
        InputMode::Normal => Style::default().fg(Color::DarkGray),
        InputMode::Search => Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    };

    // Count matching servers (exclude country headers)
    let match_count: usize = if !app.search_query.is_empty() {
        if app.split_view {
            app.server_list.len()
        } else {
            app.displayed_items
                .iter()
                .filter(|item| matches!(item, DisplayItem::Server(_)))
                .count()
        }
    } else {
        0
    };

    let search_title = if app.input_mode == InputMode::Search {
        if !app.search_query.is_empty() {
            format!(
                " Search ({} matches) - Enter to confirm, Esc to cancel ",
                match_count
            )
        } else {
            " Search (Enter to confirm, Esc to cancel) ".to_string()
        }
    } else if !app.search_query.is_empty() {
        format!(" Search ({} matches) - '/' to edit ", match_count)
    } else {
        " Search ('/' to type) ".to_string()
    };

    let search_bar = Paragraph::new(format!("  {}", app.search_query))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(search_title)
                .border_style(search_style),
        )
        .style(search_style);
    frame.render_widget(search_bar, area);

    if app.input_mode == InputMode::Search {
        frame.set_cursor(area.x + 3 + app.search_cursor_position as u16, area.y + 1);
    }
}

fn render_tree_view(frame: &mut Frame, app: &mut App, area: Rect) {
    let servers_title = add_connected_badge("ProtonVPN Servers", app.connection_status.is_some());

    if app.displayed_items.is_empty() && !app.search_query.is_empty() {
        let border_style = Style::default().fg(Color::Gray);
        render_empty_list(frame, area, servers_title, border_style);
        return;
    }

    let items: Vec<ListItem> = app
        .displayed_items
        .iter()
        .map(|item| match item {
            DisplayItem::CountryHeader(country_code) => {
                let country_full_name = countries::get_country_name(country_code);
                let country_flag = countries::get_country_flag(country_code);
                let count = app.server_counts.get(country_code).unwrap_or(&0);
                let is_expanded = app.expanded_countries.contains(country_code);

                let (icon, style) = if is_expanded {
                    (
                        "▼",
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    )
                } else {
                    ("▶", Style::default().fg(Color::White))
                };

                ListItem::new(Line::from(vec![
                    Span::styled(format!(" {} ", icon), style),
                    Span::styled(format!("{} ", country_flag), Style::default()),
                    Span::styled(format!("{} ", country_full_name), style),
                    Span::styled(
                        format!("({}) ", country_code),
                        style.add_modifier(Modifier::DIM),
                    ),
                    Span::styled(format!("[{}]", count), Style::default().fg(Color::DarkGray)),
                ]))
            }
            DisplayItem::Server(idx) => {
                let s = &app.all_servers[*idx];
                let load_color = if s.load < 30 {
                    Color::Green
                } else if s.load < 70 {
                    Color::Yellow
                } else {
                    Color::Red
                };

                let load_str = format!("{:>3}%", s.load);

                ListItem::new(Line::from(vec![
                    Span::styled("   ╰─ ", Style::default().fg(Color::DarkGray)),
                    Span::styled(format!("{:<10}", s.name), Style::default().fg(Color::White)),
                    Span::styled(
                        format!(" {:<15} ", s.city),
                        Style::default().fg(Color::Gray),
                    ),
                    Span::styled(" Load: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(load_str, Style::default().fg(load_color)),
                    Span::styled(
                        format!("  [{}]", App::format_features(s.features)),
                        Style::default()
                            .fg(Color::DarkGray)
                            .add_modifier(Modifier::ITALIC),
                    ),
                ]))
            }
        })
        .collect();

    let servers_list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(servers_title)
                .border_style(Style::default().fg(Color::Gray)),
        )
        .highlight_style(
            Style::default()
                .bg(Color::Rgb(40, 44, 52))
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(" ");

    frame.render_stateful_widget(servers_list, area, &mut app.state);
}

fn render_split_view(frame: &mut Frame, app: &mut App, area: Rect) {
    let split_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage(SPLIT_COUNTRY_PERCENT),
                Constraint::Percentage(SPLIT_SERVER_PERCENT),
            ]
            .as_ref(),
        )
        .split(area);

    let is_searching = !app.search_query.is_empty();

    // Country list
    let country_border_style = if app.split_focus == SplitFocus::Countries {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    if is_searching && app.country_list.is_empty() {
        render_empty_list(
            frame,
            split_chunks[0],
            " Countries ".to_string(),
            country_border_style,
        );
    } else {
        let country_items: Vec<ListItem> = app
            .country_list
            .iter()
            .map(|country_code| {
                let country_name = countries::get_country_name(country_code);
                let country_flag = countries::get_country_flag(country_code);
                let count = app.server_counts.get(country_code).unwrap_or(&0);

                ListItem::new(Line::from(vec![
                    Span::styled(format!(" {} ", country_flag), Style::default()),
                    Span::styled(
                        format!("{} ", country_name),
                        Style::default().fg(Color::White),
                    ),
                    Span::styled(format!("[{}]", count), Style::default().fg(Color::DarkGray)),
                ]))
            })
            .collect();

        let (country_highlight_style, country_highlight_symbol) =
            if app.split_focus == SplitFocus::Countries {
                (
                    Style::default()
                        .bg(Color::Rgb(40, 44, 52))
                        .add_modifier(Modifier::BOLD),
                    "► ",
                )
            } else {
                (Style::default().bg(Color::Rgb(30, 32, 36)), "  ")
            };

        let country_list = List::new(country_items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Countries ")
                    .border_style(country_border_style),
            )
            .highlight_style(country_highlight_style)
            .highlight_symbol(country_highlight_symbol);

        frame.render_stateful_widget(country_list, split_chunks[0], &mut app.country_state);
    }

    // Server list for selected country
    let server_border_style = if app.split_focus == SplitFocus::Servers {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    // Get selected country name for title
    let selected_country_name = app
        .country_state
        .selected()
        .and_then(|idx| app.country_list.get(idx))
        .map(|code| format!("Servers - {}", countries::get_country_name(code)))
        .unwrap_or_else(|| "Servers".to_string());

    let server_list_title =
        add_connected_badge(&selected_country_name, app.connection_status.is_some());

    if is_searching && app.server_list.is_empty() {
        render_empty_list(
            frame,
            split_chunks[1],
            server_list_title,
            server_border_style,
        );
    } else {
        let server_items: Vec<ListItem> = app
            .server_list
            .iter()
            .map(|&idx| {
                let s = &app.all_servers[idx];
                let load_color = if s.load < 30 {
                    Color::Green
                } else if s.load < 70 {
                    Color::Yellow
                } else {
                    Color::Red
                };

                let load_str = format!("{:>3}%", s.load);

                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!(" {:<12}", s.name),
                        Style::default().fg(Color::White),
                    ),
                    Span::styled(format!("{:<15}", s.city), Style::default().fg(Color::Gray)),
                    Span::styled(load_str.to_string(), Style::default().fg(load_color)),
                    Span::styled(
                        format!("  {}", App::format_features(s.features)),
                        Style::default()
                            .fg(Color::DarkGray)
                            .add_modifier(Modifier::ITALIC),
                    ),
                ]))
            })
            .collect();

        let (server_highlight_style, server_highlight_symbol) =
            if app.split_focus == SplitFocus::Servers {
                (
                    Style::default()
                        .bg(Color::Rgb(40, 44, 52))
                        .add_modifier(Modifier::BOLD),
                    "► ",
                )
            } else {
                (Style::default().bg(Color::Rgb(30, 32, 36)), "  ")
            };

        let server_list = List::new(server_items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(server_list_title)
                    .border_style(server_border_style),
            )
            .highlight_style(server_highlight_style)
            .highlight_symbol(server_highlight_symbol);

        frame.render_stateful_widget(server_list, split_chunks[1], &mut app.server_state);
    }
}

fn render_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    let footer_text = if let Some(status) = &app.connection_status {
        let duration = status.connected_at.elapsed();
        let secs = duration.as_secs();
        let h = secs / 3600;
        let m = (secs % 3600) / 60;
        let s = secs % 60;
        format!(
            " Connected: {} | Uptime {:02}:{:02}:{:02} | Press d to disconnect | {} ",
            status.server_name, h, m, s, app.status_message
        )
    } else {
        format!(" {} ", app.status_message)
    };

    let footer = Paragraph::new(footer_text)
        .block(Block::default().borders(Borders::ALL).title(" Status "))
        .style(Style::default().fg(Color::Cyan));

    frame.render_widget(footer, area);
}

fn render_hints_bar(frame: &mut Frame, app: &App, area: Rect) {
    let key_style = Style::default().fg(Color::Black).bg(Color::DarkGray);
    let sep_style = Style::default().fg(Color::DarkGray);
    let desc_style = Style::default().fg(Color::Gray);

    let hints = if app.input_mode == InputMode::Search {
        Line::from(vec![
            Span::styled(" Enter ", key_style),
            Span::styled(" Confirm ", desc_style),
            Span::styled(" | ", sep_style),
            Span::styled(" Esc ", key_style),
            Span::styled(" Cancel ", desc_style),
            Span::styled(" | ", sep_style),
            Span::styled(" Ctrl+U ", key_style),
            Span::styled(" Clear ", desc_style),
            Span::styled(" | ", sep_style),
            Span::styled(" Ctrl+W ", key_style),
            Span::styled(" Del Word ", desc_style),
        ])
    } else if app.split_view {
        Line::from(vec![
            Span::styled(" / ", key_style),
            Span::styled(" Search ", desc_style),
            Span::styled(" | ", sep_style),
            Span::styled(" Tab ", key_style),
            Span::styled(" Switch Pane ", desc_style),
            Span::styled(" | ", sep_style),
            Span::styled(" Enter ", key_style),
            Span::styled(" Connect ", desc_style),
            Span::styled(" | ", sep_style),
            Span::styled(" s ", key_style),
            Span::styled(" Save ", desc_style),
            Span::styled(" | ", sep_style),
            Span::styled(" Esc ", key_style),
            Span::styled(" Back ", desc_style),
            Span::styled(" | ", sep_style),
            Span::styled(" q ", key_style),
            Span::styled(" Quit ", desc_style),
        ])
    } else {
        Line::from(vec![
            Span::styled(" / ", key_style),
            Span::styled(" Search ", desc_style),
            Span::styled(" | ", sep_style),
            Span::styled(" Enter ", key_style),
            Span::styled(" Connect ", desc_style),
            Span::styled(" | ", sep_style),
            Span::styled(" s ", key_style),
            Span::styled(" Save ", desc_style),
            Span::styled(" | ", sep_style),
            Span::styled(" v ", key_style),
            Span::styled(" Split View ", desc_style),
            Span::styled(" | ", sep_style),
            Span::styled(" ? ", key_style),
            Span::styled(" Help ", desc_style),
            Span::styled(" | ", sep_style),
            Span::styled(" q ", key_style),
            Span::styled(" Quit ", desc_style),
        ])
    };

    let hints_bar = Paragraph::new(hints);
    frame.render_widget(hints_bar, area);
}

fn add_connected_badge(title: &str, connected: bool) -> String {
    if connected {
        format!(" {} (Connected) ", title)
    } else {
        format!(" {} ", title)
    }
}

fn render_empty_list(frame: &mut Frame, area: Rect, title: String, border_style: Style) {
    let empty = Paragraph::new(" No results. Try a different query.")
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(border_style),
        )
        .style(Style::default().fg(Color::DarkGray));

    frame.render_widget(empty, area);
}

fn render_connection_popup(frame: &mut Frame, app: &App) {
    let block = Block::default()
        .title(" Connection Status ")
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::Black));

    let area = centered_rect(60, 50, frame.size());

    frame.render_widget(Clear, area);
    frame.render_widget(block, area);

    let inner_area = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0)].as_ref())
        .margin(1)
        .split(area)[0];

    let status_text = if let Some(status) = &app.connection_status {
        let duration = status.connected_at.elapsed();
        let secs = duration.as_secs();
        let h = secs / 3600;
        let m = (secs % 3600) / 60;
        let s = secs % 60;

        vec![
            Line::from(vec![
                Span::styled("Status: ", Style::default().fg(Color::Gray)),
                Span::styled(
                    "Connected",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Server: ", Style::default().fg(Color::Gray)),
                Span::styled(&status.server_name, Style::default().fg(Color::White)),
            ]),
            Line::from(vec![
                Span::styled("Interface: ", Style::default().fg(Color::Gray)),
                Span::styled(&status.interface, Style::default().fg(Color::White)),
            ]),
            Line::from(vec![
                Span::styled("Uptime: ", Style::default().fg(Color::Gray)),
                Span::styled(
                    format!("{:02}:{:02}:{:02}", h, m, s),
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Download: ", Style::default().fg(Color::Gray)),
                Span::styled(
                    App::bytes_to_human(status.rx_bytes),
                    Style::default().fg(Color::Cyan),
                ),
            ]),
            Line::from(vec![
                Span::styled("Upload:   ", Style::default().fg(Color::Gray)),
                Span::styled(
                    App::bytes_to_human(status.tx_bytes),
                    Style::default().fg(Color::Magenta),
                ),
            ]),
            Line::from(""),
            Line::from(""),
            Line::from(Span::styled(
                "Press 'd' to Disconnect or Esc to close",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            )),
        ]
    } else {
        vec![Line::from("No active connection info.")]
    };

    let paragraph = Paragraph::new(status_text).alignment(ratatui::layout::Alignment::Center);

    frame.render_widget(paragraph, inner_area);
}

fn render_help_popup(frame: &mut Frame) {
    let block = Block::default()
        .title(" Help - Keybindings ")
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::Black));

    let area = centered_rect(65, 75, frame.size());

    frame.render_widget(Clear, area);
    frame.render_widget(block, area);

    let inner_area = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0)].as_ref())
        .margin(1)
        .split(area)[0];

    let key_style = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD);
    let desc_style = Style::default().fg(Color::White);
    let section_style = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);

    let help_text = vec![
        Line::from(Span::styled("-- Navigation --", section_style)),
        Line::from(vec![
            Span::styled("  Up/k      ", key_style),
            Span::styled("Move up", desc_style),
            Span::styled("          ", desc_style),
            Span::styled("  Down/j    ", key_style),
            Span::styled("Move down", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  Home/g    ", key_style),
            Span::styled("First item", desc_style),
            Span::styled("        ", desc_style),
            Span::styled("  End/G     ", key_style),
            Span::styled("Last item", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  PgUp      ", key_style),
            Span::styled("Page up", desc_style),
            Span::styled("           ", desc_style),
            Span::styled("  PgDn      ", key_style),
            Span::styled("Page down", desc_style),
        ]),
        Line::from(""),
        Line::from(Span::styled("-- Tree View --", section_style)),
        Line::from(vec![
            Span::styled("  Right/l   ", key_style),
            Span::styled("Expand country", desc_style),
            Span::styled("    ", desc_style),
            Span::styled("  Left/h    ", key_style),
            Span::styled("Collapse country", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  Space     ", key_style),
            Span::styled("Toggle expand", desc_style),
            Span::styled("     ", desc_style),
            Span::styled("  Enter     ", key_style),
            Span::styled("Connect/Toggle", desc_style),
        ]),
        Line::from(""),
        Line::from(Span::styled("-- Split View --", section_style)),
        Line::from(vec![
            Span::styled("  v         ", key_style),
            Span::styled("Toggle split view", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  Tab       ", key_style),
            Span::styled("Switch pane (countries/servers)", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  Left/Right", key_style),
            Span::styled("Switch pane", desc_style),
            Span::styled("       ", desc_style),
            Span::styled("  Enter     ", key_style),
            Span::styled("Connect to server", desc_style),
        ]),
        Line::from(""),
        Line::from(Span::styled("-- Actions --", section_style)),
        Line::from(vec![
            Span::styled("  /         ", key_style),
            Span::styled("Search servers", desc_style),
            Span::styled("    ", desc_style),
            Span::styled("  s         ", key_style),
            Span::styled("Save WireGuard config", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  ?         ", key_style),
            Span::styled("Show this help", desc_style),
            Span::styled("    ", desc_style),
            Span::styled("  q         ", key_style),
            Span::styled("Quit", desc_style),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Press any key to close",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let paragraph = Paragraph::new(help_text);
    frame.render_widget(paragraph, inner_area);
}
