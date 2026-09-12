# BridgeLab Roadmap

Public product roadmap for BridgeLab, the HL7 v2 / FHIR desktop workbench
by TECHEMV SRL. Dates are indicative and reordered as user feedback comes
in — the best way to influence priorities is a GitHub issue or discussion.
Pricing and plans live on the website, not here.

---

## Where BridgeLab stands today

Shipped and available:

- Modern desktop UI (Svelte 5 + Monaco) on Windows, macOS (Intel/ARM) and Linux
- Native HL7 v2 and FHIR (JSON/XML) parsing, tree view and field inspector
- 10 MB+ message handling with smart truncation (base64 payloads included)
- HL7 v2 schema catalogue for ten versions, v2.1 through v2.7.1 —
  2,320 message structures behind the tree, the Field Inspector and the
  XSD export
- Validation against the HL7 standard, plugin packs for custom rules
- FHIR validation: structural checks, custom rules, and conformance against
  installed StructureDefinitions (cardinality, types, choice elements, fixed
  values, unknown elements)
- FHIRPath 2.0 evaluator — the full language, verified against the official
  HL7 test suite
- Anonymization engine (21 built-in PHI fields, extensible via plugins)
- MLLP client/listener, HTTP client, SOAP 1.1/1.2 client (Enterprise)
- FHIR Bundle visualizer
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
- [x] Additional HL7 versions via the same importer — v2.1, v2.2, v2.3,
      v2.3.1, v2.4, v2.5.1, v2.6, v2.7 and v2.7.1 (2,320 message structures
      in total; v2.7.1 is a technical correction of v2.7 and shares its
      definitions)
- [ ] HL7 v2.8 and later — needs a second data source; hl7-dictionary, the
      MIT-licensed source behind the versions above, stops at v2.7
- [x] Keyboard shortcut customization (rebind any command, persisted, reset
      to defaults)
- [ ] Theme editor — custom colour schemes beyond the built-in dark/light
- [ ] Open-source the HL7 parser core as a separate crate on crates.io
- [ ] Documentation site and short video tutorials

### FHIR

- [x] Interactive FHIR Bundle visualizer (reference navigation, graph view, inline inspector)
- [x] FHIRPath evaluator panel
- [x] FHIR resource templates (Patient, Observation, Bundle examples)
- [x] Full FHIRPath 2.0 language — complete operator set and precedence,
      three-valued logic, partial-precision dates, quantities with unit
      conversion, ~70 functions, and `resolve()` against contained and
      bundled resources. Verified against the official HL7 FHIRPath test
      suite (836 of 922 runnable cases)
- [ ] Close the remaining FHIRPath gap: `lowBoundary()`/`highBoundary()`
      (needs exact decimal arithmetic) and compound UCUM units
- [x] Custom FHIR validation rules builder — declarative packs with
      FHIRPath invariants or selector-plus-check rules, an in-app editor
      that tests a rule against the open resource before saving it, and no
      code execution
- [x] FHIR profile validation — install FHIR NPM packages and validate
      against their StructureDefinitions: cardinality, element types,
      choice elements, fixed values and patterns, and unknown elements.
      Profiles declared in `meta.profile` are applied automatically. A
      Rust implementation rather than the official HL7 validator, which
      is a Java tool and would not fit an offline desktop app
- [ ] Terminology validation — `required` bindings need the ValueSet
      expanded, which means shipping the terminology packages or calling
      a server; both need a decision before it is worth building
- [ ] Slicing beyond the basics, and profile-aware element documentation
      in the Field Inspector

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
- [ ] Anonymization rules editor — extra PHI fields can already be added
      through a declarative plugin pack; this is the in-app editor for them

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
