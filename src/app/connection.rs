use crossterm::{
    cursor::{Hide, Show},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::os::unix::fs::PermissionsExt;
use std::{fs, io, path::PathBuf, time::Instant};
use tokio::process::Command;

use super::{App, ConnectionStatus, DisplayItem};
use crate::wireguard;

fn suspend_tui() {
    let _ = disable_raw_mode();
    let _ = execute!(io::stdout(), LeaveAlternateScreen, Show);
}

fn resume_tui() {
    let _ = execute!(io::stdout(), EnterAlternateScreen, Hide);
    let _ = enable_raw_mode();
}

impl App {
    pub fn get_interface_name() -> String {
        "proton-tui0".to_string()
    }

    pub fn get_config_path() -> PathBuf {
        PathBuf::from(format!("/tmp/{}.conf", Self::get_interface_name()))
    }

    pub async fn start_wireguard(&mut self, config_path: &str, server_name: String) {
        self.log(format!("Starting WireGuard with {}...", config_path));

        // Suspend TUI
        suspend_tui();

        println!("Running wg-quick... (Sudo password may be required)");

        // Run command interactively
        let status = Command::new("sudo")
            .arg("wg-quick")
            .arg("up")
            .arg(config_path)
            .status()
            .await;

        // Resume TUI
        resume_tui();
        self.should_redraw = true;

        match status {
            Ok(s) => {
                if s.success() {
                    self.log("WireGuard started successfully.".to_string());
                    self.status_message = "Connected to VPN".to_string();
                    self.connection_status = Some(ConnectionStatus {
                        server_name,
                        interface: Self::get_interface_name(),
                        connected_at: Instant::now(),
                        rx_bytes: 0,
                        tx_bytes: 0,
                    });
                    self.show_connection_popup = true;
                } else {
                    self.log("Failed to start WireGuard.".to_string());
                    self.status_message = "Connection Failed".to_string();
                }
            }
            Err(e) => {
                self.log(format!("Failed to execute wg-quick: {}", e));
                self.status_message = "Execution Error".to_string();
            }
        }
    }

    pub async fn stop_wireguard(&mut self) {
        // Suspend TUI
        suspend_tui();

        println!("Stopping WireGuard... (Sudo password may be required)");

        let config_path = Self::get_config_path();
        let status = Command::new("sudo")
            .arg("wg-quick")
            .arg("down")
            .arg(&config_path)
            .status()
            .await;

        // Resume TUI
        resume_tui();
        self.should_redraw = true;

        match status {
            Ok(s) => {
                if s.success() {
                    self.log("Disconnected.".to_string());
                    self.connection_status = None;
                    self.show_connection_popup = false;

                    let id_opt = self.current_config_id.clone();
                    if let Some(id) = id_opt {
                        self.log(format!("Removing config {} from server...", id));
                        match self.client.delete_config(&id).await {
                            Ok(_) => self.log("Configuration removed from server.".to_string()),
                            Err(e) => self.log(format!("Server-side cleanup failed: {}", e)),
                        }
                    }
                    self.current_config_id = None;
                } else {
                    self.log("Failed to stop WireGuard.".to_string());
                }
            }
            Err(e) => {
                self.log(format!("Failed to execute wg-quick: {}", e));
            }
        }
    }

    pub async fn create_config(&mut self, server_idx: usize) -> Option<PathBuf> {
        if let Some(server) = self.all_servers.get(server_idx).cloned() {
            self.log(format!("Generating config for {}...", server.name));

            // 1. Generate Keys
            let keypair = wireguard::generate_keypair();
            let pub_pem = wireguard::get_ed_public_pem(&keypair.ed_public);
            let x_priv = wireguard::get_x25519_private_base64(&keypair.ed_private);

            // 2. Register Config
            let device_name = format!("proton-tui-{}", server.name);
            match self
                .client
                .register_config(&pub_pem, &server, &device_name)
                .await
            {
                Ok(json) => {
                    if let Some(id_str) = json["SerialNumber"].as_str() {
                        self.current_config_id = Some(id_str.to_string());
                    }

                    // 3. Generate Config File
                    if server.servers.is_empty() {
                        self.log("Error: No physical servers found.".to_string());
                        return None;
                    }
                    let instance = &server.servers[0];
                    let config_content = wireguard::generate_wg_config(
                        &x_priv,
                        &instance.x25519_public_key,
                        &instance.entry_ip,
                        &server.name,
                    );

                    let config_path = Self::get_config_path();
                    if let Err(e) = fs::write(&config_path, config_content) {
                        self.log(format!("Error writing config file: {}", e));
                        return None;
                    }

                    // Set permissions to 600 (rw-------)
                    if let Ok(mut perms) = fs::metadata(&config_path).map(|m| m.permissions()) {
                        perms.set_mode(0o600);
                        if let Err(e) = fs::set_permissions(&config_path, perms) {
                            self.log(format!("Warning: Could not set file permissions: {}", e));
                        }
                    }

                    self.log(format!("Config saved to {}.", config_path.display()));
                    return Some(config_path);
                }
                Err(e) => {
                    self.log(format!("API Error: {}", e));
                    return None;
                }
            }
        }
        None
    }

    pub async fn connect_to_selected(&mut self) {
        let selected_idx = match self.state.selected() {
            Some(i) => i,
            None => return,
        };

        let item = match self.displayed_items.get(selected_idx) {
            Some(item) => item,
            None => return,
        };

        match item {
            DisplayItem::CountryHeader(country) => {
                // Toggle expand/collapse on Enter for headers
                if self.expanded_countries.contains(country) {
                    self.expanded_countries.remove(country);
                } else {
                    self.expanded_countries.insert(country.clone());
                }
                self.update_display_list();
            }
            DisplayItem::Server(server_idx) => {
                let idx = *server_idx;

                if let Some(config_path) = self.create_config(idx).await {
                    if let Some(server) = self.all_servers.get(idx) {
                        self.start_wireguard(
                            config_path.to_str().unwrap_or("wg0.conf"),
                            server.name.clone(),
                        )
                        .await;
                    }
                }
            }
        }
    }

    pub async fn save_selected_config(&mut self) {
        let selected_idx = match self.state.selected() {
            Some(i) => i,
            None => return,
        };

        let item = match self.displayed_items.get(selected_idx) {
            Some(item) => item,
            None => return,
        };

        if let DisplayItem::Server(server_idx) = item {
            let _ = self.create_config(*server_idx).await;
        }
    }
}
