# BridgeLab Licensing

## How it works

BridgeLab uses **Ed25519 digital signatures** for license verification.

```
              Private key (TECHEMV-only)
                      ↓
License payload  → Sign (Ed25519) → Signed license key (base64)
                                          ↓
              Public key (embedded in app) → Verify → Accept / Reject
```

- The **private key** is held exclusively by TECHEMV SRL and is never
  distributed.
- The **public key** is embedded in the compiled binary. It can verify
  signatures but cannot produce them.
- The signature covers the full license payload (licensee, email,
  license type, feature set, hardware ID, expiration).

## License tiers

| Tier | Features |
|------|----------|
| Free | HL7 v2 editor, parser, tree view, basic validation, CLI |
| Pro | + FHIR support, MLLP/HTTP transport, anonymization, export, plugin packs |
| Enterprise | + SOAP, priority support, advanced plugins |

## Hardware binding

Each license is bound to a hardware fingerprint derived from the
machine's hostname, OS, architecture and user account. Licenses are
non-transferable between machines.

## Trial

First launch starts a 30-day Pro trial. After the trial expires, the
app remains usable with the Free feature set.

## Obtaining a license

Contact **TECHEMV SRL** or purchase via the website.

## Offline validation

After initial activation, the license is verified locally via Ed25519
signature check. No network call is required.
