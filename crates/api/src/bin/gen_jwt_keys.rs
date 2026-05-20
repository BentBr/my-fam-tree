//! Dev helper that emits a fresh Ed25519 keypair + ready-to-paste .env lines.

use std::env;

use ed25519_dalek::SigningKey;
use ed25519_dalek::pkcs8::spki::der::pem::LineEnding;
use ed25519_dalek::pkcs8::{EncodePrivateKey, EncodePublicKey};
use rand::rngs::OsRng;

#[allow(
    clippy::print_stdout,
    reason = "this binary's sole purpose is to print key material and env lines to stdout"
)]
fn main() -> anyhow::Result<()> {
    let kid = env::args().nth(1).unwrap_or_else(|| {
        let now = chrono::Utc::now();
        format!("local-{}", now.format("%Y%m%d-%H%M%S"))
    });

    let signing = SigningKey::generate(&mut OsRng);
    let verifying = signing.verifying_key();

    let priv_pem = signing.to_pkcs8_pem(LineEnding::LF)?.to_string();
    let pub_pem = verifying.to_public_key_pem(LineEnding::LF)?;

    println!("# kid={kid}");
    println!("# Add the following lines to your .env (private key in JWT_PRIVATE_KEY,");
    println!("# public key in JWT_PUBLIC_KEYS as a JSON array entry).");
    println!();
    println!("JWT_PRIVATE_KEY_ID={kid}");
    println!("JWT_PRIVATE_KEY='{}'", priv_pem.trim_end().replace('\n', "\\n"));
    let json = serde_json::json!([{ "kid": kid, "public_pem": pub_pem.trim_end() }]);
    println!("JWT_PUBLIC_KEYS='{json}'");
    Ok(())
}
