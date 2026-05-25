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
        .split(frame.area());

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

    // Count matching servers (exclude country headers and IP headers)
    let match_count: usize = if !app.search_query.is_empty() {
        if app.split_view {
            app.split_server_items
                .iter()
                .filter(|item| matches!(item, DisplayItem::Server(_)))
                .count()
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
        frame.set_cursor_position((area.x + 3 + app.search_cursor_position as u16, area.y + 1));
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
            DisplayItem::EntryIpHeader(country_code, entry_ip) => {
                let key = (country_code.clone(), entry_ip.clone());
                let is_expanded = app.expanded_entry_ips.contains(&key);

                // Count servers with this entry IP in this country
                let server_count = app
                    .all_servers
                    .iter()
                    .filter(|s| {
                        &s.exit_country == country_code
                            && s.servers.first().map(|i| &i.entry_ip) == Some(entry_ip)
                    })
                    .count();

                let (icon, style) = if is_expanded {
                    (
                        "▼",
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD),
                    )
                } else {
                    ("▶", Style::default().fg(Color::DarkGray))
                };

                ListItem::new(Line::from(vec![
                    Span::styled(format!("   {} ", icon), style),
                    Span::styled(format!("{} ", entry_ip), style),
                    Span::styled(
                        format!("[{}]", server_count),
                        Style::default().fg(Color::DarkGray),
                    ),
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
                    Span::styled("      ╰─ ", Style::default().fg(Color::DarkGray)),
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

    // Build country title with counts
    let country_title = if is_searching {
        format!(
            " Countries ({}/{}) ",
            app.country_list.len(),
            app.full_country_list.len()
        )
    } else {
        format!(" Countries ({}) ", app.full_country_list.len())
    };

    if is_searching && app.country_list.is_empty() {
        render_empty_list(frame, split_chunks[0], country_title, country_border_style);
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
                    .title(country_title)
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

    // Get selected country name for title with counts
    let server_count = app
        .split_server_items
        .iter()
        .filter(|item| matches!(item, DisplayItem::Server(_)))
        .count();
    let selected_country_name = if is_searching {
        format!("Servers ({}/{})", server_count, app.all_servers.len())
    } else {
        app.country_state
            .selected()
            .and_then(|idx| app.country_list.get(idx))
            .map(|code| {
                format!(
                    "Servers - {} ({}/{})",
                    countries::get_country_name(code),
                    server_count,
                    app.all_servers.len()
                )
            })
            .unwrap_or_else(|| format!("Servers ({})", app.all_servers.len()))
    };

    let server_list_title =
        add_connected_badge(&selected_country_name, app.connection_status.is_some());

    if is_searching && app.split_server_items.is_empty() {
        render_empty_list(
            frame,
            split_chunks[1],
            server_list_title,
            server_border_style,
        );
    } else {
        let server_items: Vec<ListItem> = app
            .split_server_items
            .iter()
            .map(|item| match item {
                DisplayItem::EntryIpHeader(_, entry_ip) => {
                    ListItem::new(Line::from(vec![Span::styled(
                        format!(" {} ", entry_ip),
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD),
                    )]))
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
                }
                DisplayItem::CountryHeader(_) => ListItem::new(Line::from("")),
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
    let stats = format!(
        "Servers: {} | Entry IPs: {}",
        app.total_servers, app.unique_entry_ips
    );

    let footer_text = if let Some(status) = &app.connection_status {
        let duration = status.connected_at.elapsed();
        let secs = duration.as_secs();
        let h = secs / 3600;
        let m = (secs % 3600) / 60;
        let s = secs % 60;
        if app.status_message.is_empty() {
            format!(
                " Connected: {} | Uptime {:02}:{:02}:{:02} | {} ",
                status.server_name, h, m, s, stats
            )
        } else {
            format!(
                " Connected: {} | Uptime {:02}:{:02}:{:02} | {} ",
                status.server_name, h, m, s, app.status_message
            )
        }
    } else if app.status_message.is_empty() {
        format!(" {} ", stats)
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

    let hints = if app.show_help_popup {
        // Help popup: any key closes it
        Line::from(vec![
            Span::styled(" Any Key ", key_style),
            Span::styled(" Close ", desc_style),
        ])
    } else if app.show_connection_popup {
        // Connection popup: disconnect or close
        Line::from(vec![
            Span::styled(" d ", key_style),
            Span::styled(" Disconnect ", desc_style),
            Span::styled(" | ", sep_style),
            Span::styled(" Esc ", key_style),
            Span::styled(" Close ", desc_style),
        ])
    } else if app.input_mode == InputMode::Search {
        // Search mode: confirm, cancel, help
        Line::from(vec![
            Span::styled(" Enter ", key_style),
            Span::styled(" Confirm ", desc_style),
            Span::styled(" | ", sep_style),
            Span::styled(" Esc ", key_style),
            Span::styled(" Cancel ", desc_style),
            Span::styled(" | ", sep_style),
            Span::styled(" ? ", key_style),
            Span::styled(" Help ", desc_style),
        ])
    } else if app.split_view {
        // Split view: pane-specific hints
        match app.split_focus {
            SplitFocus::Countries => {
                // Countries pane focused
                Line::from(vec![
                    Span::styled(" Tab ", key_style),
                    Span::styled(" Switch ", desc_style),
                    Span::styled(" | ", sep_style),
                    Span::styled(" Enter ", key_style),
                    Span::styled(" Select ", desc_style),
                    Span::styled(" | ", sep_style),
                    Span::styled(" v ", key_style),
                    Span::styled(" Tree ", desc_style),
                    Span::styled(" | ", sep_style),
                    Span::styled(" ? ", key_style),
                    Span::styled(" Help ", desc_style),
                    Span::styled(" | ", sep_style),
                    Span::styled(" q ", key_style),
                    Span::styled(" Quit ", desc_style),
                ])
            }
            SplitFocus::Servers => {
                // Servers pane focused
                Line::from(vec![
                    Span::styled(" Tab ", key_style),
                    Span::styled(" Switch ", desc_style),
                    Span::styled(" | ", sep_style),
                    Span::styled(" Enter ", key_style),
                    Span::styled(" Connect ", desc_style),
                    Span::styled(" | ", sep_style),
                    Span::styled(" ← ", key_style),
                    Span::styled(" Back ", desc_style),
                    Span::styled(" | ", sep_style),
                    Span::styled(" ? ", key_style),
                    Span::styled(" Help ", desc_style),
                    Span::styled(" | ", sep_style),
                    Span::styled(" q ", key_style),
                    Span::styled(" Quit ", desc_style),
                ])
            }
        }
    } else {
        // Tree view: essential navigation hints
        Line::from(vec![
            Span::styled(" / ", key_style),
            Span::styled(" Search ", desc_style),
            Span::styled(" | ", sep_style),
            Span::styled(" Enter ", key_style),
            Span::styled(" Connect ", desc_style),
            Span::styled(" | ", sep_style),
            Span::styled(" v ", key_style),
            Span::styled(" Split ", desc_style),
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

    let area = centered_rect(60, 50, frame.area());

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
    // Color palette
    let bg_color = Color::Rgb(22, 22, 30);
    let border_color = Color::Rgb(88, 91, 112);
    let accent_color = Color::Rgb(137, 180, 250);
    let key_fg = Color::Rgb(249, 226, 175);
    let key_bg = Color::Rgb(49, 50, 68);
    let desc_color = Color::Rgb(205, 214, 244);
    let section_color = Color::Rgb(166, 227, 161);
    let divider_color = Color::Rgb(69, 71, 90);
    let footer_bg = Color::Rgb(49, 50, 68);
    let footer_text = Color::Rgb(186, 194, 222);

    // Fixed column widths for table-like alignment
    const KEY_WIDTH: usize = 10;
    const DESC_WIDTH: usize = 18;
    const LEFT_PAD: usize = 5;
    const COL_GAP: usize = 2;

    // Calculate content width: pad + key + space + desc + gap + key + space + desc + pad
    let content_width =
        LEFT_PAD + KEY_WIDTH + 1 + DESC_WIDTH + COL_GAP + KEY_WIDTH + 1 + DESC_WIDTH + LEFT_PAD;
    // Add borders (2) and margin (2)
    let popup_width = (content_width + 4) as u16;

    let block = Block::default()
        .title(" 󰋖  Keyboard Shortcuts ")
        .title_style(
            Style::default()
                .fg(accent_color)
                .add_modifier(Modifier::BOLD),
        )
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(bg_color));

    // Center the popup with fixed width
    let frame_size = frame.area();
    let popup_width = popup_width.min(frame_size.width);
    let popup_height = (frame_size.height * 85 / 100).min(frame_size.height);
    let x = (frame_size.width.saturating_sub(popup_width)) / 2;
    let y = (frame_size.height.saturating_sub(popup_height)) / 2;
    let area = Rect::new(x, y, popup_width, popup_height);

    frame.render_widget(Clear, area);
    frame.render_widget(block, area);

    let inner_area = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)].as_ref())
        .margin(1)
        .split(area);

    let content_area = inner_area[0];
    let footer_area = inner_area[1];

    // Styles
    let section_style = Style::default()
        .fg(section_color)
        .add_modifier(Modifier::BOLD);
    let key_style = Style::default()
        .fg(key_fg)
        .bg(key_bg)
        .add_modifier(Modifier::BOLD);
    let desc_style = Style::default().fg(desc_color);
    let divider_style = Style::default().fg(divider_color);

    let left_pad = " ".repeat(LEFT_PAD);
    let col_gap = " ".repeat(COL_GAP);

    // Helper to create a formatted key with background color, padded to fixed width
    let fmt_key = |key: &str| -> Span {
        Span::styled(
            format!(" {:^width$} ", key, width = KEY_WIDTH - 2),
            key_style,
        )
    };

    // Helper to create description padded to fixed width
    let fmt_desc = |desc: &str| -> Span {
        Span::styled(format!("{:<width$}", desc, width = DESC_WIDTH), desc_style)
    };

    // Helper to create a two-column row with consistent alignment
    let make_row = |key1: &str, desc1: &str, key2: &str, desc2: &str| -> Line {
        Line::from(vec![
            Span::raw(left_pad.clone()),
            fmt_key(key1),
            Span::raw(" "),
            fmt_desc(desc1),
            Span::raw(col_gap.clone()),
            fmt_key(key2),
            Span::raw(" "),
            fmt_desc(desc2),
        ])
    };

    // Divider width = content width minus left/right padding
    let divider_width = KEY_WIDTH + 1 + DESC_WIDTH + COL_GAP + KEY_WIDTH + 1 + DESC_WIDTH;
    let divider = "─".repeat(divider_width);

    let help_text = vec![
        // Navigation Section
        Line::from(vec![
            Span::raw(left_pad.clone()),
            Span::styled("  Navigation", section_style),
        ]),
        Line::from(""),
        make_row("↑ / k", "Move up", "↓ / j", "Move down"),
        make_row("Home / g", "Go to first", "End / G", "Go to last"),
        make_row("PgUp", "Page up", "PgDn", "Page down"),
        make_row("Ctrl-u", "Half page up", "Ctrl-d", "Half page down"),
        Line::from(""),
        Line::from(Span::styled(
            format!("{}{}", left_pad, divider),
            divider_style,
        )),
        Line::from(""),
        // Tree View Section
        Line::from(vec![
            Span::raw(left_pad.clone()),
            Span::styled("  Tree View", section_style),
        ]),
        Line::from(""),
        make_row("→ / l", "Expand node", "← / h", "Collapse node"),
        make_row("Space", "Toggle expand", "Enter", "Connect/Toggle"),
        Line::from(""),
        Line::from(Span::styled(
            format!("{}{}", left_pad, divider),
            divider_style,
        )),
        Line::from(""),
        // Split View Section
        Line::from(vec![
            Span::raw(left_pad.clone()),
            Span::styled("  Split View", section_style),
        ]),
        Line::from(""),
        make_row("v", "Toggle split", "Tab", "Switch pane"),
        make_row("← / →", "Switch pane", "Enter", "Connect server"),
        Line::from(""),
        Line::from(Span::styled(
            format!("{}{}", left_pad, divider),
            divider_style,
        )),
        Line::from(""),
        // Actions Section
        Line::from(vec![
            Span::raw(left_pad.clone()),
            Span::styled("  Actions", section_style),
        ]),
        Line::from(""),
        make_row("/", "Search servers", "s", "Save config"),
        make_row("i", "Toggle IP group", "d", "Disconnect VPN"),
        make_row("?", "Show this help", "q", "Quit application"),
    ];

    let paragraph = Paragraph::new(help_text);
    frame.render_widget(paragraph, content_area);

    // Footer with close instruction - centered with empty line before
    let footer = Paragraph::new(vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            " Press any key to close ",
            Style::default().fg(footer_text).bg(footer_bg),
        )]),
    ])
    .alignment(ratatui::layout::Alignment::Center);
    frame.render_widget(footer, footer_area);
}
