use rand::RngCore;
use rand::rngs::OsRng;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use crate::models::Keypair;

#[tauri::command]
pub fn generate_keypair() -> Result<Keypair, String> {
    let mut secret = [0u8; 32];
    OsRng.fill_bytes(&mut secret);
    let sk = x25519_dalek::StaticSecret::from(secret);
    let pk = x25519_dalek::PublicKey::from(&sk);
    Ok(Keypair {
        public_key: format!("-----BEGIN NEBULA X25519 PUBLIC KEY-----\n{}\n-----END NEBULA X25519 PUBLIC KEY-----", BASE64.encode(pk.as_bytes())),
        private_key: format!("-----BEGIN NEBULA X25519 PRIVATE KEY-----\n{}\n-----END NEBULA X25519 PRIVATE KEY-----", BASE64.encode(sk.to_bytes())),
    })
}
