use rand::RngCore;
use rand::rngs::OsRng;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use crate::models::Keypair;

const PRIV_HEADER: &str = "-----BEGIN NEBULA X25519 PRIVATE KEY-----";
const PRIV_FOOTER: &str = "-----END NEBULA X25519 PRIVATE KEY-----";
const PUB_HEADER: &str = "-----BEGIN NEBULA X25519 PUBLIC KEY-----";
const PUB_FOOTER: &str = "-----END NEBULA X25519 PUBLIC KEY-----";

/// Generate a Nebula-compatible X25519 keypair.
/// Nebula uses X25519 (Curve25519) for key exchange, NOT Ed25519.
#[tauri::command]
pub fn generate_keypair() -> Result<Keypair, String> {
    log::info!("Generating X25519 keypair");

    // Generate random 32-byte secret key
    let mut secret = [0u8; 32];
    OsRng.fill_bytes(&mut secret);

    // Derive public key
    let secret_key = x25519_dalek::StaticSecret::from(secret);
    let public_key = x25519_dalek::PublicKey::from(&secret_key);

    let private_pem = format!(
        "{}\n{}\n{}",
        PRIV_HEADER,
        BASE64.encode(secret_key.to_bytes()),
        PRIV_FOOTER
    );

    let public_pem = format!(
        "{}\n{}\n{}",
        PUB_HEADER,
        BASE64.encode(public_key.as_bytes()),
        PUB_FOOTER
    );

    log::info!("X25519 keypair generated");

    Ok(Keypair {
        public_key: public_pem,
        private_key: private_pem,
    })
}
