use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use sha2::{Digest, Sha512};

pub struct KeyPair {
    pub ed_private: [u8; 32],
    pub ed_public: [u8; 32],
}

pub fn generate_keypair() -> KeyPair {
    let mut csprng = OsRng;
    let signing_key = SigningKey::generate(&mut csprng);
    let verifying_key = signing_key.verifying_key();

    KeyPair {
        ed_private: signing_key.to_bytes(),
        ed_public: verifying_key.to_bytes(),
    }
}

pub fn get_ed_public_pem(public_bytes: &[u8]) -> String {
    let prefix: [u8; 12] = [
        0x30, 0x2A, 0x30, 0x05, 0x06, 0x03, 0x2B, 0x65, 0x70, 0x03, 0x21, 0x00,
    ];
    let mut full = Vec::with_capacity(prefix.len() + public_bytes.len());
    full.extend_from_slice(&prefix);
    full.extend_from_slice(public_bytes);

    let b64 = BASE64.encode(full);

    format!(
        "-----BEGIN PUBLIC KEY-----\n{}
-----END PUBLIC KEY-----",
        b64
    )
}

pub fn get_x25519_private_base64(ed_private: &[u8]) -> String {
    let mut hasher = Sha512::new();
    hasher.update(ed_private);
    let hash = hasher.finalize();

    let mut x_priv = [0u8; 32];
    x_priv.copy_from_slice(&hash[..32]);

    x_priv[0] &= 248;
    x_priv[31] &= 127;
    x_priv[31] |= 64;

    BASE64.encode(x_priv)
}

pub fn generate_wg_config(
    private_key: &str,
    peer_public_key: &str,
    peer_endpoint_ip: &str,
    peer_name: &str,
) -> String {
    format!(
        r#"[Interface]
# Key for {peer_name}
PrivateKey = {private_key}
Address = 10.2.0.2/32

[Peer]
# {peer_name}
PublicKey = {peer_public_key}
AllowedIPs = 0.0.0.0/0
Endpoint = {peer_endpoint_ip}:51820
PersistentKeepalive = 25
"#
    )
}
