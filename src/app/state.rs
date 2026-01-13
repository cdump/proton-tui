use std::time::Instant;

#[derive(PartialEq, Clone, Copy)]
pub enum InputMode {
    Normal,
    Search,
}

#[derive(PartialEq, Clone, Copy)]
pub enum SplitFocus {
    Countries,
    Servers,
}

#[derive(Clone, PartialEq)]
pub enum DisplayItem {
    CountryHeader(String),
    Server(usize), // Index in all_servers
}

pub struct ConnectionStatus {
    pub server_name: String,
    pub interface: String,
    pub connected_at: Instant,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
}
