//! Proton VPN SRP Authentication Library
//!
//! This implements the Secure Remote Password (SRP-6a) protocol
//! as used by ProtonVPN for username/password authentication.
//!
//! # Auth Flow
//! 1. POST /auth/info - Get SRP parameters (Modulus, Salt, ServerEphemeral, Version)
//! 2. Verify PGP signature on Modulus
//! 3. Compute SRP client values (ClientEphemeral, ClientProof)
//! 4. POST /auth - Submit SRP proof, get tokens (UID, AccessToken, RefreshToken)
//! 5. (Optional) POST /auth/2fa - If 2FA is enabled

use anyhow::{anyhow, Context, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use num_bigint::BigUint;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha512};

// ============================================================================
// Constants
// ============================================================================

/// Proton VPN API base URL
pub const API_BASE: &str = "https://vpn-api.proton.me";

/// App version string for API requests
pub const APP_VERSION: &str = "linux-vpn-cli@4.13.1+x86-64";

/// SRP generator (g = 2)
const SRP_GENERATOR: u32 = 2;

// ============================================================================
// Error Types
// ============================================================================

/// Authentication-specific errors
#[derive(Debug, Clone)]
pub enum AuthError {
    /// CAPTCHA verification required (error code 9001)
    CaptchaRequired { captcha_url: String },
    /// Invalid credentials (error code 8002)
    InvalidCredentials,
    /// General API error
    ApiError { code: i64, message: String },
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthError::CaptchaRequired { captcha_url } => {
                write!(f, "CAPTCHA required. URL: {}", captcha_url)
            }
            AuthError::InvalidCredentials => write!(f, "Invalid username or password"),
            AuthError::ApiError { code, message } => {
                write!(f, "API error {}: {}", code, message)
            }
        }
    }
}

impl std::error::Error for AuthError {}

// ============================================================================
// API Response Types
// ============================================================================

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
#[allow(dead_code)]
struct AuthInfoResponse {
    code: i32,
    modulus: Option<String>,
    server_ephemeral: Option<String>,
    version: Option<i32>,
    salt: Option<String>,
    #[serde(rename = "SRPSession")]
    srp_session: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
struct AuthRequest {
    username: String,
    client_ephemeral: String,
    client_proof: String,
    #[serde(rename = "SRPSession")]
    srp_session: String,
}

// ============================================================================
// Public Result Types
// ============================================================================

/// Successful authentication result containing session tokens
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResult {
    /// User ID
    pub uid: String,
    /// Access token for API requests
    pub access_token: String,
    /// Refresh token for obtaining new access tokens
    pub refresh_token: String,
    /// API scopes granted
    pub scopes: Vec<String>,
    /// Whether 2FA is enabled on the account
    pub two_factor_enabled: bool,
}

// ============================================================================
// Proton's Custom Hash (PMHash) - Extends SHA512 to 2048 bits
// ============================================================================

/// PMHash: Creates a 256-byte (2048-bit) hash by concatenating
/// SHA512(data || 0x00) || SHA512(data || 0x01) || SHA512(data || 0x02) || SHA512(data || 0x03)
fn pm_hash(data: &[u8]) -> Vec<u8> {
    let mut result = Vec::with_capacity(256);
    for suffix in 0u8..4 {
        let mut hasher = Sha512::new();
        hasher.update(data);
        hasher.update([suffix]);
        result.extend_from_slice(&hasher.finalize());
    }
    result
}

/// Custom hash that converts result to BigUint (little-endian)
fn custom_hash_to_int(inputs: &[&[u8]]) -> BigUint {
    let mut combined = Vec::new();
    for input in inputs {
        combined.extend_from_slice(input);
    }
    let hash = pm_hash(&combined);
    BigUint::from_bytes_le(&hash)
}

// ============================================================================
// Password Hashing (Proton SRP v4)
// ============================================================================

/// Hash password according to Proton's SRP v3/v4 algorithm:
/// 1. Prepare salt: salt + "proton", truncate to 16 bytes
/// 2. bcrypt hash with cost 10 using $2y$ format
/// 3. Hash (bcrypt_output || modulus) with PMHash
fn hash_password(password: &str, salt: &[u8], modulus: &[u8]) -> Result<BigUint> {
    // Step 1: Prepare salt - append "proton" and take first 16 bytes
    let mut salted = salt.to_vec();
    salted.extend_from_slice(b"proton");
    salted.truncate(16);

    // Ensure we have exactly 16 bytes (pad with zeros if needed)
    while salted.len() < 16 {
        salted.push(0);
    }

    let salt_arr: [u8; 16] = salted[..16]
        .try_into()
        .map_err(|_| anyhow!("Salt must be 16 bytes"))?;

    // Step 2: bcrypt hash with cost 10
    let hashed = bcrypt::hash_with_salt(password.as_bytes(), 10, salt_arr)
        .map_err(|e| anyhow!("bcrypt error: {}", e))?;

    // Format as $2y$ (PHP compatible format that Python/Proton uses)
    let bcrypt_output = hashed.format_for_version(bcrypt::Version::TwoY);
    let hash_bytes = bcrypt_output.as_bytes();

    // Step 3: Combine with modulus and hash with PMHash
    let mut combined = hash_bytes.to_vec();
    combined.extend_from_slice(modulus);
    let result = pm_hash(&combined);

    Ok(BigUint::from_bytes_le(&result))
}

// ============================================================================
// SRP-6a Protocol Implementation
// ============================================================================

struct SrpClient {
    /// Prime modulus N
    modulus: BigUint,
    /// Generator g (usually 2)
    generator: BigUint,
    /// Client's private ephemeral (a)
    private_ephemeral: BigUint,
    /// Client's public ephemeral A = g^a mod N
    public_ephemeral: BigUint,
}

impl SrpClient {
    fn new(modulus: BigUint) -> Self {
        let generator = BigUint::from(SRP_GENERATOR);

        // Generate random private ephemeral (256 bits)
        let mut rng = rand::rngs::OsRng;
        let mut a_bytes = [0u8; 32];
        rng.fill_bytes(&mut a_bytes);
        // Ensure MSB is set for consistent bit length
        a_bytes[31] |= 0x80;
        let private_ephemeral = BigUint::from_bytes_le(&a_bytes);

        // Calculate A = g^a mod N
        let public_ephemeral = generator.modpow(&private_ephemeral, &modulus);

        Self {
            modulus,
            generator,
            private_ephemeral,
            public_ephemeral,
        }
    }

    /// Get client's public ephemeral (A) as base64
    fn get_challenge(&self) -> String {
        let bytes = self.public_ephemeral.to_bytes_le();
        // Pad to 256 bytes
        let mut padded = vec![0u8; 256];
        padded[..bytes.len()].copy_from_slice(&bytes);
        BASE64.encode(&padded)
    }

    /// Process server challenge and compute client proof
    fn process_challenge(
        &self,
        password: &str,
        salt: &[u8],
        server_ephemeral: &BigUint,
    ) -> Result<(String, Vec<u8>)> {
        let n = &self.modulus;
        let g = &self.generator;
        let a = &self.private_ephemeral;
        let a_pub = &self.public_ephemeral;
        let b_pub = server_ephemeral;

        // u = H(A, B)
        let a_bytes = to_padded_bytes(a_pub, 256);
        let b_bytes = to_padded_bytes(b_pub, 256);
        let u = custom_hash_to_int(&[&a_bytes, &b_bytes]);

        // x = hash_password(password, salt, modulus)
        let modulus_bytes = to_padded_bytes(n, 256);
        let x = hash_password(password, salt, &modulus_bytes)?;

        // k = H(g, N) - NOTE: order is g first, then N!
        let g_bytes = to_padded_bytes(g, 256);
        let k = custom_hash_to_int(&[&g_bytes, &modulus_bytes]);

        // S = (B - k * g^x)^(a + u*x) mod N
        let g_x = g.modpow(&x, n);
        let k_g_x = (&k * &g_x) % n;

        // Handle potential underflow: if B < k*g^x, add N
        let base = if b_pub >= &k_g_x {
            b_pub - &k_g_x
        } else {
            (b_pub + n) - &k_g_x
        };

        let exp = a + &u * &x;
        let s = base.modpow(&exp, n);

        // K = S as bytes (NOT hashed!) - this is Proton's specific implementation
        let s_bytes = to_padded_bytes(&s, 256);
        let session_key = s_bytes.clone();

        // M = H(A, B, K) - client proof
        let mut m_input = Vec::new();
        m_input.extend_from_slice(&a_bytes);
        m_input.extend_from_slice(&b_bytes);
        m_input.extend_from_slice(&session_key);
        let client_proof = pm_hash(&m_input);

        let proof_b64 = BASE64.encode(&client_proof);

        Ok((proof_b64, session_key))
    }
}

/// Convert BigUint to fixed-size little-endian bytes
fn to_padded_bytes(n: &BigUint, size: usize) -> Vec<u8> {
    let bytes = n.to_bytes_le();
    let mut padded = vec![0u8; size];
    let copy_len = bytes.len().min(size);
    padded[..copy_len].copy_from_slice(&bytes[..copy_len]);
    padded
}

// ============================================================================
// PGP Modulus Verification
// ============================================================================

/// Extract modulus value from PGP signed message
fn parse_modulus(pgp_signed: &str) -> Result<BigUint> {
    let lines: Vec<&str> = pgp_signed.lines().collect();
    let mut modulus_b64 = String::new();
    let mut in_data = false;

    for line in &lines {
        if line.starts_with("-----BEGIN PGP SIGNATURE") {
            break;
        }
        if in_data && !line.is_empty() {
            modulus_b64.push_str(line.trim());
        }
        if line.is_empty() && !in_data {
            in_data = true;
        }
    }

    let modulus_bytes = BASE64
        .decode(&modulus_b64)
        .context("Failed to decode modulus base64")?;

    Ok(BigUint::from_bytes_le(&modulus_bytes))
}

// ============================================================================
// Proton Auth Client
// ============================================================================

/// Proton authentication client
pub struct ProtonAuth {
    http_client: reqwest::Client,
}

impl ProtonAuth {
    /// Create a new ProtonAuth client
    pub fn new() -> Result<Self> {
        let http_client = reqwest::Client::builder()
            .user_agent("ProtonVPN/4.13.1 (Linux; ubuntu/22.04)")
            .cookie_store(true)
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self { http_client })
    }

    /// Authenticate with username and password
    pub async fn authenticate(&self, username: &str, password: &str) -> Result<AuthResult> {
        // Step 1: Get auth info
        let auth_info = self.get_auth_info(username).await?;

        if auth_info.code != 1000 {
            return Err(anyhow!(
                "Auth info failed: {} (code {})",
                auth_info.error.unwrap_or_default(),
                auth_info.code
            ));
        }

        let modulus_pgp = auth_info.modulus.context("No modulus in response")?;
        let server_ephemeral_b64 = auth_info.server_ephemeral.context("No server ephemeral")?;
        let salt_b64 = auth_info.salt.context("No salt")?;
        let srp_session = auth_info.srp_session.context("No SRP session")?;

        // Step 2: Parse modulus
        let modulus = parse_modulus(&modulus_pgp)?;

        // Step 3: Create SRP client
        let srp_client = SrpClient::new(modulus);
        let client_ephemeral = srp_client.get_challenge();

        // Step 4: Compute proof
        let salt = BASE64.decode(&salt_b64)?;
        let server_ephemeral_bytes = BASE64.decode(&server_ephemeral_b64)?;
        let server_ephemeral = BigUint::from_bytes_le(&server_ephemeral_bytes);
        let (client_proof, _session_key) =
            srp_client.process_challenge(password, &salt, &server_ephemeral)?;

        let auth_request = AuthRequest {
            username: username.to_string(),
            client_ephemeral,
            client_proof,
            srp_session,
        };

        // Step 5: Authenticate
        let auth_response = self.do_authenticate(&auth_request).await?;

        let code = auth_response
            .get("Code")
            .and_then(|c| c.as_i64())
            .unwrap_or(0);

        // Handle error codes
        match code {
            1000 => {
                // Success - extract tokens
                let uid = auth_response
                    .get("UID")
                    .and_then(|u| u.as_str())
                    .context("No UID in response")?
                    .to_string();

                let access_token = auth_response
                    .get("AccessToken")
                    .and_then(|t| t.as_str())
                    .context("No AccessToken in response")?
                    .to_string();

                let refresh_token = auth_response
                    .get("RefreshToken")
                    .and_then(|t| t.as_str())
                    .context("No RefreshToken in response")?
                    .to_string();

                let scopes = auth_response
                    .get("Scopes")
                    .and_then(|s| s.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str())
                            .map(String::from)
                            .collect()
                    })
                    .unwrap_or_default();

                let two_factor_enabled = auth_response
                    .get("2FA")
                    .and_then(|tfa| tfa.get("Enabled"))
                    .and_then(|e| e.as_i64())
                    .unwrap_or(0)
                    > 0;

                Ok(AuthResult {
                    uid,
                    access_token,
                    refresh_token,
                    scopes,
                    two_factor_enabled,
                })
            }
            9001 => {
                // CAPTCHA required
                let challenge_token = auth_response
                    .get("Details")
                    .and_then(|d| d.get("HumanVerificationToken"))
                    .and_then(|t| t.as_str())
                    .unwrap_or("");

                let captcha_url = format!("{}/core/v4/captcha?Token={}", API_BASE, challenge_token);

                Err(AuthError::CaptchaRequired { captcha_url }.into())
            }
            8002 => Err(AuthError::InvalidCredentials.into()),
            _ => {
                let message = auth_response
                    .get("Error")
                    .and_then(|e| e.as_str())
                    .unwrap_or("Unknown error")
                    .to_string();
                Err(AuthError::ApiError { code, message }.into())
            }
        }
    }

    async fn get_auth_info(&self, username: &str) -> Result<AuthInfoResponse> {
        let url = format!("{}/auth/info", API_BASE);

        let resp = self
            .http_client
            .post(&url)
            .header("x-pm-appversion", APP_VERSION)
            .header("Accept", "application/vnd.protonmail.v1+json")
            .header("Accept-Language", "en-US,en;q=0.9")
            .json(&serde_json::json!({"Username": username}))
            .send()
            .await
            .context("Failed to send auth/info request")?;

        resp.json()
            .await
            .context("Failed to parse auth/info response")
    }

    async fn do_authenticate(&self, request: &AuthRequest) -> Result<serde_json::Value> {
        let url = format!("{}/auth", API_BASE);

        let resp = self
            .http_client
            .post(&url)
            .header("x-pm-appversion", APP_VERSION)
            .header("Accept", "application/vnd.protonmail.v1+json")
            .header("Accept-Language", "en-US,en;q=0.9")
            .json(request)
            .send()
            .await
            .context("Failed to send auth request")?;

        resp.json().await.context("Failed to parse auth response")
    }
}

impl Default for ProtonAuth {
    fn default() -> Self {
        Self::new().expect("Failed to create ProtonAuth client")
    }
}
