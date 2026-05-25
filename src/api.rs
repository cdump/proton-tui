use crate::models::{LogicalServer, LogicalServersResponse};
use anyhow::Result;
use reqwest::{header, Client};
use serde_json::json;

/// Proton VPN API base URL
const API_BASE: &str = "https://vpn-api.proton.me";

/// App version string for API requests
const APP_VERSION: &str = "linux-vpn-cli@4.13.1+x86-64";

pub struct ProtonClient {
    client: Client,
}

pub enum CertificateMode {
    Ephemeral,
    Persistent,
}

impl ProtonClient {
    pub fn new(uid: String, access_token: String) -> Self {
        let mut headers = header::HeaderMap::new();
        headers.insert("x-pm-appversion", APP_VERSION.parse().unwrap());
        headers.insert("x-pm-uid", uid.parse().unwrap());
        headers.insert(
            "Accept",
            "application/vnd.protonmail.v1+json".parse().unwrap(),
        );

        // Use Bearer token authentication
        let auth_value = format!("Bearer {}", access_token);
        headers.insert("Authorization", auth_value.parse().unwrap());

        Self {
            client: Client::builder().default_headers(headers).build().unwrap(),
        }
    }

    pub async fn get_logical_servers(&self) -> Result<Vec<LogicalServer>> {
        let url = format!("{}/vpn/logicals", API_BASE);
        let resp = self.client.get(&url).send().await?;
        if !resp.status().is_success() {
            return Err(anyhow::anyhow!(
                "Failed to fetch servers: {} - {:?}",
                resp.status(),
                resp.text().await?
            ));
        }
        let data: LogicalServersResponse = resp.json().await?;
        Ok(data.logical_servers)
    }

    pub async fn register_config(
        &self,
        pub_pem: &str,
        server: &LogicalServer,
        device_name: &str,
        mode: CertificateMode,
    ) -> Result<serde_json::Value> {
        let url = format!("{}/vpn/v1/certificate", API_BASE);

        if server.servers.is_empty() {
            return Err(anyhow::anyhow!(
                "No physical servers found for this logical server"
            ));
        }
        let instance = &server.servers[0];

        let mut body = json!({
           "ClientPublicKey": pub_pem,
           "Features": {
               "peerName": server.name,
               "peerIp": instance.entry_ip,
               "peerPublicKey": instance.x25519_public_key,
               "platform": "Linux",
               "SafeMode": false,
               "SplitTCP": false,
               "PortForwarding": false,
               "RandomNAT": false,
               "NetShieldLevel": 0
           }
        });

        match mode {
            CertificateMode::Ephemeral => {
                body["Duration"] = json!("1440 min");
            }
            CertificateMode::Persistent => {
                body["Mode"] = json!("persistent");
                body["DeviceName"] = json!(device_name);
            }
        }

        let resp = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let text = resp.text().await?;
            return Err(anyhow::anyhow!("Failed to register config: {}", text));
        }

        let json: serde_json::Value = resp.json().await?;
        Ok(json)
    }
}
