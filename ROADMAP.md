# BridgeLab Roadmap

Public product roadmap for BridgeLab, the HL7 v2 / FHIR desktop workbench
by TECHEMV SRL. Dates are indicative and reordered as user feedback comes
in — the best way to influence priorities is a GitHub issue or discussion.
Pricing and plans live on the website, not here.

---

## Where BridgeLab stands today

Shipped and available in the current release:

- Modern desktop UI (Svelte 5 + Monaco) on Windows, macOS (Intel/ARM) and Linux
- Native HL7 v2 and FHIR (JSON/XML) parsing, tree view and field inspector
- 10 MB+ message handling with smart truncation (base64 payloads included)
- Validation against the HL7 standard, plugin packs for custom rules
- Anonymization engine (21 built-in PHI fields, extensible via plugins)
- MLLP client/listener, HTTP client, SOAP 1.1/1.2 client (Enterprise)
- FHIR Bundle visualizer and FHIRPath evaluator
- XSD schema export for HL7 v2 message types
- Test-message generator, batch validation and batch anonymization
- Test case library, message templates, side-by-side compare
- `bridgelab-cli` for headless validation in CI
- 5-language UI (EN, IT, FR, ES, DE)
- Offline Ed25519 license verification, online activation codes

---

## Development roadmap

### Core polish (ongoing)

- [x] Full HL7 v2.5 message catalogue (248 messages, 149 segments, 78 composites)
- [ ] Additional HL7 versions (v2.3, v2.4, v2.6, v2.7, v2.8) via the same importer
- [ ] Keyboard shortcut customization and theme editor polish
- [ ] Open-source the HL7 parser core as a separate crate on crates.io
- [ ] Documentation site and short video tutorials

### FHIR

- [x] Interactive FHIR Bundle visualizer (reference navigation, graph view, inline inspector)
- [x] FHIRPath evaluator panel
- [ ] FHIR profile validation (via the HL7 official validator)
- [ ] Custom FHIR validation rules builder
- [ ] FHIR resource templates (Patient, Observation, Bundle examples)

### Integration & testing

- [x] `bridgelab-cli` — headless validation for CI
- [x] Test case library (reusable HL7/FHIR scenarios)
- [x] SOAP 1.1/1.2 client with WS-Security UsernameToken and WS-Addressing (Enterprise)
- [ ] WSDL import for the SOAP client
- [x] Message generator with realistic fake data (seeded; ADT/ORU/ORM)
- [x] Batch validation with CSV report
- [x] Batch anonymization
- [ ] Test case pack export/import (share scenarios via a folder or Git)
- [ ] Git integration (save messages to a repo, diff across commits)

### Collaboration

- [ ] Team workspaces: shared test case libraries, field-level comments, review workflows
- [ ] HL7 v2 → FHIR converter based on the official v2-to-FHIR implementation guide, with a visual mapping editor and round-trip testing
- [ ] Audit trail logging
- [ ] Anonymization rules editor (beyond the 21 built-ins)

### Platform

- [x] Declarative plugin packs (validation rules, PHI fields) — no code execution
- [ ] Scripted plugins (sandboxed) and a community plugin marketplace
- [ ] Optional, end-to-end encrypted cloud sync for settings and templates
- [ ] Docker image for headless/CI use
- [ ] Mirth Connect message importer
- [ ] Web-based lite version for quick checks

### Enterprise

- [ ] SAML/SSO support
- [ ] Role-based access control
- [ ] Advanced audit logging
- [ ] Priority support SLA portal

### Longer term

- [ ] AI-assisted HL7 v2 → FHIR mapping suggestions
- [ ] Natural-language queries over message sets
- [ ] Anomaly detection on message streams
- [ ] Automated interface documentation

---

Have a need that is not listed? Open an issue on GitHub or write to
info@techemv.it.
