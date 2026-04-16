use base64::Engine;
use clap::{Parser, Subcommand};
use ed25519_dalek::{Signer, SigningKey, VerifyingKey};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "bridgelab-keygen", version, about = "BridgeLab license generator")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate a new Ed25519 keypair for signing licenses
    GenerateKeypair {
        /// Output file for the private key (keep secret!)
        #[arg(short, long, default_value = "bridgelab-private.key")]
        private: PathBuf,
        /// Output file for the public key (embed in app)
        #[arg(short = 'u', long, default_value = "bridgelab-public.key")]
        public: PathBuf,
    },
    /// Generate a signed license key
    Generate {
        /// Path to the private key file
        #[arg(short, long, default_value = "bridgelab-private.key")]
        private: PathBuf,
        /// License type: free, pro, enterprise
        #[arg(short, long, default_value = "pro")]
        license_type: String,
        /// Licensee name or company
        #[arg(short = 'n', long)]
        licensee: String,
        /// Licensee email
        #[arg(short, long)]
        email: String,
        /// Hardware ID to bind to (leave empty for any machine)
        #[arg(short = 'w', long, default_value = "")]
        hardware_id: String,
        /// Days until expiration (default: 365 for paid, 0 = never)
        #[arg(short, long, default_value = "365")]
        days: i64,
        /// Output file for the license (default: print to stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Verify a license key against a public key
    Verify {
        /// Path to the public key
        #[arg(short = 'u', long, default_value = "bridgelab-public.key")]
        public: PathBuf,
        /// License key to verify (Base64 string)
        key: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LicensePayload {
    license_type: String,
    licensee: String,
    email: String,
    hardware_id: String,
    issued_at: String,
    expires_at: Option<String>,
    features: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LicenseFile {
    payload: LicensePayload,
    signature: String,
}

fn features_for(license_type: &str) -> Vec<String> {
    match license_type {
        "free" => vec!["core".into(), "hl7v2".into()],
        "pro" | "professional" => vec![
            "core".into(), "hl7v2".into(), "fhir".into(),
            "mllp".into(), "http".into(), "anonymize".into(), "export".into(),
        ],
        "ent" | "enterprise" => vec![
            "core".into(), "hl7v2".into(), "fhir".into(),
            "mllp".into(), "http".into(), "anonymize".into(), "export".into(),
            "soap".into(), "plugins".into(), "priority_support".into(),
        ],
        _ => vec!["core".into()],
    }
}

fn normalize_type(t: &str) -> String {
    match t.to_lowercase().as_str() {
        "free" => "free".into(),
        "pro" | "professional" => "professional".into(),
        "ent" | "enterprise" => "enterprise".into(),
        _ => "free".into(),
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

fn hex_decode(s: &str) -> Result<Vec<u8>, String> {
    if s.len() % 2 != 0 { return Err("Invalid hex length".into()); }
    (0..s.len()).step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16)
            .map_err(|e| format!("Invalid hex: {}", e)))
        .collect()
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::GenerateKeypair { private, public } => {
            let mut rng = OsRng;
            let signing_key = SigningKey::generate(&mut rng);
            let verifying_key = signing_key.verifying_key();

            let priv_hex = hex_encode(&signing_key.to_bytes());
            let pub_hex = hex_encode(&verifying_key.to_bytes());

            fs::write(&private, &priv_hex).expect("Failed to write private key");
            fs::write(&public, &pub_hex).expect("Failed to write public key");

            println!("[OK] Keypair generated");
            println!("  Private: {} (KEEP SECRET!)", private.display());
            println!("  Public:  {}", public.display());
            println!();
            println!("Embed this public key in src-tauri/src/licensing/mod.rs:");
            println!("  const PUBLIC_KEY_HEX: &str = \"{}\";", pub_hex);
        }

        Commands::Generate { private, license_type, licensee, email, hardware_id, days, output } => {
            let priv_hex = fs::read_to_string(&private)
                .expect("Failed to read private key");
            let priv_bytes = hex_decode(priv_hex.trim()).expect("Invalid private key");
            if priv_bytes.len() != 32 {
                eprintln!("Error: private key must be 32 bytes");
                std::process::exit(1);
            }

            let signing_key = SigningKey::from_bytes(
                priv_bytes.as_slice().try_into().unwrap()
            );

            let lt = normalize_type(&license_type);
            let expires_at = if days > 0 {
                Some((chrono::Utc::now() + chrono::Duration::days(days)).to_rfc3339())
            } else {
                None
            };

            let payload = LicensePayload {
                license_type: lt.clone(),
                licensee,
                email,
                hardware_id,
                issued_at: chrono::Utc::now().to_rfc3339(),
                expires_at,
                features: features_for(&lt),
            };

            let payload_json = serde_json::to_string(&payload)
                .expect("Failed to serialize payload");

            let signature = signing_key.sign(payload_json.as_bytes());
            let sig_hex = hex_encode(&signature.to_bytes());

            let license = LicenseFile {
                payload,
                signature: sig_hex,
            };

            let license_json = serde_json::to_string(&license)
                .expect("Failed to serialize license");

            let key_b64 = base64::engine::general_purpose::STANDARD.encode(license_json.as_bytes());

            match output {
                Some(path) => {
                    fs::write(&path, &key_b64).expect("Failed to write license");
                    println!("[OK] License written to: {}", path.display());
                    println!();
                    println!("Type:     {}", lt);
                    println!("Features: {}", license.payload.features.join(", "));
                    if let Some(ref exp) = license.payload.expires_at {
                        println!("Expires:  {}", exp);
                    } else {
                        println!("Expires:  Never");
                    }
                    println!();
                    println!("License key (Base64):");
                    println!("{}", key_b64);
                }
                None => {
                    println!("{}", key_b64);
                }
            }
        }

        Commands::Verify { public, key } => {
            let pub_hex = fs::read_to_string(&public)
                .expect("Failed to read public key");
            let pub_bytes = hex_decode(pub_hex.trim()).expect("Invalid public key");
            if pub_bytes.len() != 32 {
                eprintln!("Error: public key must be 32 bytes");
                std::process::exit(1);
            }

            let verifying_key = VerifyingKey::from_bytes(
                pub_bytes.as_slice().try_into().unwrap()
            ).expect("Invalid public key");

            let license_bytes = base64::engine::general_purpose::STANDARD
                .decode(key.trim())
                .expect("Invalid Base64 key");

            let license: LicenseFile = serde_json::from_slice(&license_bytes)
                .expect("Invalid license JSON");

            let payload_json = serde_json::to_string(&license.payload)
                .expect("Failed to serialize payload");

            let sig_bytes = hex_decode(&license.signature).expect("Invalid signature");
            if sig_bytes.len() != 64 {
                eprintln!("[FAIL] Invalid signature length");
                std::process::exit(1);
            }

            let signature = ed25519_dalek::Signature::from_bytes(
                sig_bytes.as_slice().try_into().unwrap()
            );

            use ed25519_dalek::Verifier;
            match verifying_key.verify(payload_json.as_bytes(), &signature) {
                Ok(_) => {
                    println!("[OK] License signature is VALID");
                    println!();
                    println!("Type:       {}", license.payload.license_type);
                    println!("Licensee:   {}", license.payload.licensee);
                    println!("Email:      {}", license.payload.email);
                    println!("Issued:     {}", license.payload.issued_at);
                    match license.payload.expires_at {
                        Some(e) => println!("Expires:    {}", e),
                        None => println!("Expires:    Never"),
                    }
                    if !license.payload.hardware_id.is_empty() {
                        println!("Hardware:   {}", license.payload.hardware_id);
                    }
                    println!("Features:   {}", license.payload.features.join(", "));
                }
                Err(e) => {
                    eprintln!("[FAIL] License signature is INVALID: {}", e);
                    std::process::exit(1);
                }
            }
        }
    }
}
