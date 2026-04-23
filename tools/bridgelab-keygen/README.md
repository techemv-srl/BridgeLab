# BridgeLab License Generator (CLI)

Tool for generating and verifying signed license keys for BridgeLab.

## Setup (One-time)

Generate your Ed25519 keypair. **Keep the private key secret!**

```bash
cargo run --release -- generate-keypair
```

This creates:
- `bridgelab-private.key` - Used to sign licenses (SECRET)
- `bridgelab-public.key` - Embedded in the app for verification

Then embed the public key in `src-tauri/src/licensing/mod.rs`:

```rust
const PUBLIC_KEY_HEX: &str = "<paste public key hex here>";
```

## Generating Licenses

### Basic examples

```bash
# Professional license, 1 year, any machine
cargo run --release -- generate \
  --license-type pro \
  --licensee "Acme Hospital" \
  --email admin@acme.com \
  --days 365

# Enterprise license bound to a specific machine
cargo run --release -- generate \
  --license-type enterprise \
  --licensee "Big Corp" \
  --email it@bigcorp.com \
  --hardware-id "BL-ABC123..." \
  --days 365 \
  --output bigcorp-license.txt

# Free license (no expiration)
cargo run --release -- generate \
  --license-type free \
  --licensee "Open Source User" \
  --email user@example.com \
  --days 0
```

## Verifying Licenses

```bash
cargo run --release -- verify <license-key-base64>
```

## License Tiers

| Tier | Features |
|------|----------|
| `free` | Core + HL7v2 parsing |
| `pro` / `professional` | + FHIR, MLLP, HTTP, anonymize, export |
| `enterprise` / `ent` | + SOAP, plugins, priority support |

## How it works

1. License payload (JSON) contains: type, licensee, email, hardware_id, dates, features
2. Payload is signed with Ed25519 using the private key
3. Full license (payload + signature) is JSON-serialized then Base64-encoded
4. User pastes the Base64 string into BridgeLab activation dialog
5. App verifies signature with embedded public key (offline)
6. If valid, license is stored in app data directory

The signature prevents tampering - users cannot change their license type or extend
expiration without the private key.
