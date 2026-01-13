mod connection;
mod navigation;
mod search;
mod split_view;
mod state;

pub use state::{ConnectionStatus, DisplayItem, InputMode, SplitFocus};

use ratatui::widgets::ListState;
use std::collections::{HashMap, HashSet};
use std::fs;

use crate::api::ProtonClient;
use crate::countries;
use crate::models::LogicalServer;

pub struct App {
    pub all_servers: Vec<LogicalServer>,
    pub displayed_items: Vec<DisplayItem>,
    pub state: ListState,
    pub client: ProtonClient,
    pub status_message: String,
    pub input_mode: InputMode,
    pub search_query: String,
    pub search_cursor_position: usize,
    pub expanded_countries: HashSet<String>,
    pub server_counts: HashMap<String, usize>,
    pub should_redraw: bool,
    pub connection_status: Option<ConnectionStatus>,
    pub show_connection_popup: bool,
    pub show_help_popup: bool,
    pub current_config_id: Option<String>,
    // Split view state
    pub split_view: bool,
    pub split_focus: SplitFocus,
    pub country_list: Vec<String>,
    pub full_country_list: Vec<String>,
    pub country_state: ListState,
    pub server_list: Vec<usize>,
    pub server_state: ListState,
}

impl App {
    pub fn new(client: ProtonClient, servers: Vec<LogicalServer>) -> App {
        let mut counts = HashMap::new();
        for server in &servers {
            *counts.entry(server.exit_country.clone()).or_insert(0) += 1;
        }

        // Build sorted country list
        let mut country_list: Vec<String> = counts.keys().cloned().collect();
        country_list.sort_by_key(|a| countries::get_country_name(a));

        let mut app = App {
            all_servers: servers,
            displayed_items: Vec::new(),
            state: ListState::default(),
            client,
            status_message: "Ready. Enter to Connect, 's' to Save, '/' Search, 'q' Quit."
                .to_string(),
            input_mode: InputMode::Normal,
            search_query: String::new(),
            search_cursor_position: 0,
            expanded_countries: HashSet::new(),
            server_counts: counts,
            should_redraw: false,
            connection_status: None,
            show_connection_popup: false,
            show_help_popup: false,
            current_config_id: None,
            // Split view state
            split_view: false,
            split_focus: SplitFocus::Countries,
            full_country_list: country_list.clone(),
            country_list,
            country_state: ListState::default(),
            server_list: Vec::new(),
            server_state: ListState::default(),
        };
        app.update_display_list();
        if !app.displayed_items.is_empty() {
            app.state.select(Some(0));
        }
        // Initialize country selection for split view
        if !app.country_list.is_empty() {
            app.country_state.select(Some(0));
            app.update_server_list_for_selected_country();
        }
        app
    }

    pub fn log(&mut self, msg: String) {
        self.status_message = msg;
    }

    pub fn bytes_to_human(b: u64) -> String {
        const UNIT: u64 = 1024;
        if b < UNIT {
            return format!("{} B", b);
        }
        let div = UNIT;
        if b < div * UNIT {
            return format!("{:.1} KB", b as f64 / div as f64);
        }
        let div = div * UNIT;
        if b < div * UNIT {
            return format!("{:.1} MB", b as f64 / div as f64);
        }
        let div = div * UNIT;
        format!("{:.1} GB", b as f64 / div as f64)
    }

    pub fn update_traffic_stats(&mut self) {
        if let Some(ref mut status) = self.connection_status {
            if let Ok(content) = fs::read_to_string("/proc/net/dev") {
                for line in content.lines() {
                    let parts: Vec<&str> = line.split(':').collect();
                    if parts.len() == 2 {
                        let iface = parts[0].trim();
                        if iface == status.interface {
                            let stats: Vec<&str> = parts[1].split_whitespace().collect();
                            if stats.len() >= 9 {
                                if let (Ok(rx), Ok(tx)) =
                                    (stats[0].parse::<u64>(), stats[8].parse::<u64>())
                                {
                                    status.rx_bytes = rx;
                                    status.tx_bytes = tx;
                                }
                            }
                            break;
                        }
                    }
                }
            }
        }
    }

    pub fn format_features(mask: i32) -> String {
        let mut features = Vec::new();
        if mask & 1 != 0 {
            features.push("SC");
        }
        if mask & 2 != 0 {
            features.push("TOR");
        }
        if mask & 4 != 0 {
            features.push("P2P");
        }
        if mask & 8 != 0 {
            features.push("STR");
        }
        if mask & 16 != 0 {
            features.push("v6");
        }

        if features.is_empty() {
            return String::new();
        }
        features.join(" ")
    }
}
