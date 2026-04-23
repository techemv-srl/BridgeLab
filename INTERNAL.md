# BridgeLab - Internal Operations Guide

**CONFIDENTIAL** — This file lives only in the private 1warpengine repo.
Do NOT merge it into `release/public` or push it to `techemv-srl/BridgeLab`.

## Ed25519 Keypair

| Key | Value |
|-----|-------|
| Public (embedded in app) | `cd9559f4beffe61a9c2878434a84fb2c3de85e36247c4188537c722d9fcc2649` |
| Private (**SECRET**) | Store in your password manager. File: `bridgelab-private.key` |

## Generating a License

```bash
cd tools/bridgelab-keygen
cargo run --release -- generate \
  --private /path/to/bridgelab-private.key \
  --license-type pro \
  --licensee "Customer Name" \
  --email "customer@company.com" \
  --hardware-id "BL-XXXXXXXXXXXXXXXX" \
  --days 365
```

The output is a Base64-encoded signed license key. Send it to the customer.

### License types

| Type | Flag | Features |
|------|------|----------|
| Free | `free` | core, hl7v2, fhir_parse, validation, mllp_send, http_get, anonymize_detect |
| Professional | `pro` | All Free + mllp_listen, http_mutate, http_auth, anonymize_mask, export, fhirpath, bundle_visualizer, plugins_unlimited, test_cases_unlimited |
| Enterprise | `enterprise` | All Pro + soap, priority_support |

### Getting the customer's Hardware ID

The customer opens **Settings → Activation** and reads the `BL-XXXXXXXXXXXXXXXX`
value displayed under "Hardware ID". They send this to you via email. You
embed it in the `--hardware-id` flag so the license is bound to their machine.

### Verifying a license

```bash
cargo run --release -- verify \
  --public /path/to/bridgelab-public.key \
  "BASE64_LICENSE_KEY_HERE"
```

## Triggering a Release

Only from the private repo (1warpengine):

```bash
git tag v0.1.0
git push origin v0.1.0
```

This triggers `.github/workflows/release.yml` which:
1. Creates a draft GitHub Release
2. Builds for Windows (NSIS + MSI), macOS (DMG), Linux (.deb, .AppImage)
3. Publishes the release

On the TECHEMV-SRL repo, the release workflow requires the `production`
environment (Settings → Environments → add a reviewer).

## Syncing dev → public

Canonical commands — mirror the private `release/public` branch onto the
public `techemv-srl/BridgeLab` repo (branch `main`). The `techemv` remote
must already be configured locally:

```bash
git remote add techemv git@github.com:techemv-srl/BridgeLab.git
```

Then, every time you want to publish:

```bash
git fetch origin release/public

# Pre-flight: refuse to mirror if INTERNAL.md or keygen slipped into release/public.
# (Defense-in-depth: the public-safety.yml workflow catches the same thing on GitHub.)
if git ls-tree -r --name-only origin/release/public | grep -qE '^(INTERNAL\.md|tools/bridgelab-keygen/)'; then
  echo "REFUSING TO SYNC: dev-only files present on origin/release/public. Clean release/public first." >&2
  exit 1
fi

git push techemv origin/release/public:main --force
```

This force-pushes whatever is on `origin/release/public` to the public repo's
`main`. GitHub Pages (configured via Actions on the public repo) redeploys
`docs/site/` automatically after the push.

### Bringing new dev work into release/public

To forward new features from the dev branch into `release/public`:

```bash
git checkout release/public
git merge claude/hl7-editor-modern-0s5FX --no-edit
# Resolve any keygen conflicts: git rm tools/bridgelab-keygen/...
git push origin release/public
```

Then run the sync block above.

## Security Checklist (before making TECHEMV public)

- [x] Keygen tool removed from release/public
- [x] README redacted (no keygen instructions, no release trigger)
- [x] verify_signature rejects placeholder key (returns false)
- [x] Dev-mode signature bypass removed
- [x] activate_simple_key gated behind #[cfg(debug_assertions)]
- [x] Release workflow requires `production` environment approval
- [x] Real Ed25519 public key embedded
- [x] Feature-gate enforcement on all Pro/Enterprise IPC commands
- [ ] Generate Tauri signer keypair for auto-updater
- [ ] HMAC-sign trial.json to prevent manual reset
- [ ] Feature-gate enforcement in frontend UI (disable buttons, not just IPC)
