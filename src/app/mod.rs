mod connection;
mod navigation;
mod search;
mod split_view;
mod state;

pub use connection::ConfigTarget;
pub use search::{CountrySearchCache, ServerSearchCache};
pub use state::{ConnectionStatus, DisplayItem, InputMode, SplitFocus};

use ratatui::widgets::ListState;
use std::collections::{HashMap, HashSet};
use std::fs;

use crate::api::ProtonClient;
use crate::config::AppConfig;
use crate::countries;
use crate::models::LogicalServer;

pub struct App {
    pub all_servers: Vec<LogicalServer>,
    pub sorted_server_indices: Vec<usize>, // Pre-sorted by country, entry IP, name
    pub displayed_items: Vec<DisplayItem>,
    pub state: ListState,
    pub client: ProtonClient,
    pub total_servers: usize,
    pub unique_entry_ips: usize,
    pub status_message: String,
    pub input_mode: InputMode,
    pub search_query: String,
    pub search_cursor_position: usize,
    pub expanded_countries: HashSet<String>,
    pub expanded_entry_ips: HashSet<(String, String)>, // (country_code, entry_ip)
    pub group_by_entry_ip: bool,
    pub server_counts: HashMap<String, usize>,
    pub should_redraw: bool,
    pub connection_status: Option<ConnectionStatus>,
    pub show_connection_popup: bool,
    pub show_help_popup: bool,
    // Split view state
    pub split_view: bool,
    pub split_focus: SplitFocus,
    pub country_list: Vec<String>,
    pub full_country_list: Vec<String>,
    pub country_state: ListState,
    pub split_server_items: Vec<DisplayItem>,
    pub server_state: ListState,
    // Pre-computed search cache (avoids repeated to_lowercase calls)
    pub search_cache: Vec<ServerSearchCache>,
    pub country_search_cache: Vec<CountrySearchCache>,
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

        // Build search caches (pre-compute lowercase strings once)
        let search_cache = Self::build_search_cache(&servers);
        let country_search_cache = Self::build_country_search_cache(&country_list);

        // Pre-compute sorted server indices using cached data
        let mut sorted_indices: Vec<usize> = (0..servers.len()).collect();
        sorted_indices.sort_by(|&a, &b| {
            let cache_a = &search_cache[a];
            let cache_b = &search_cache[b];
            let country_cmp = cache_a.country_name.cmp(&cache_b.country_name);
            if country_cmp != std::cmp::Ordering::Equal {
                return country_cmp;
            }
            let entry_ip_cmp = cache_a.entry_ip.cmp(&cache_b.entry_ip);
            if entry_ip_cmp != std::cmp::Ordering::Equal {
                return entry_ip_cmp;
            }
            servers[a].name.cmp(&servers[b].name)
        });

        // Compute unique entry IPs
        let mut entry_ips: HashSet<&str> = HashSet::new();
        for server in &servers {
            for instance in &server.servers {
                entry_ips.insert(&instance.entry_ip);
            }
        }
        let total_servers = servers.len();
        let unique_entry_ips = entry_ips.len();

        let (group_by_entry_ip, split_view) = AppConfig::load()
            .map(|c| (c.group_by_entry_ip, c.split_view))
            .unwrap_or((true, false));

        let mut app = App {
            all_servers: servers,
            sorted_server_indices: sorted_indices,
            displayed_items: Vec::new(),
            state: ListState::default(),
            client,
            total_servers,
            unique_entry_ips,
            status_message: String::new(),
            input_mode: InputMode::Normal,
            search_query: String::new(),
            search_cursor_position: 0,
            expanded_countries: HashSet::new(),
            expanded_entry_ips: HashSet::new(),
            group_by_entry_ip,
            server_counts: counts,
            should_redraw: false,
            connection_status: None,
            show_connection_popup: false,
            show_help_popup: false,
            // Split view state
            split_view,
            split_focus: SplitFocus::Countries,
            full_country_list: country_list.clone(),
            country_list,
            country_state: ListState::default(),
            split_server_items: Vec::new(),
            server_state: ListState::default(),
            // Pre-computed search cache
            search_cache,
            country_search_cache,
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

    pub fn toggle_group_by_entry_ip(&mut self) {
        self.group_by_entry_ip = !self.group_by_entry_ip;
        if self.split_view {
            if self.search_query.is_empty() {
                self.update_server_list_for_selected_country();
            } else {
                self.update_split_view_for_search();
            }
        } else {
            self.update_display_list();
        }
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
