# hl7-schema-importer

Build-time tool that ingests HL7 v2.x schema definitions and emits the JSON
payload consumed at runtime by `src-tauri/src/parser/hl7/schema/mod.rs`
(shipped as `src-tauri/resources/hl7/v<version>.json`).

## Why

Hand-coding ~120 segments, ~50 composites and ~80 message structures per HL7
version in Rust is 5–10k lines of error-prone code. This tool lets the data
live in well-defined source files (hl7-dictionary today; HAPI, official v2.xml
XSDs, or HL7 CSV tables later) and translates them into the exact JSON shape
the application expects.

## Usage

```bash
# Round-trip / reformat an existing BridgeLab schema JSON:
hl7-schema-importer \
    --format bridgelab-json \
    --source-dir ../../src-tauri/resources/hl7 \
    --hl7-version 2.5 \
    --output /tmp/v2_5_reformatted.json

# Ingest from hl7-dictionary (requires a local clone — MIT licensed):
git clone https://github.com/Ensighten/hl7-dictionary ~/hl7-dictionary
hl7-schema-importer \
    --format hl7-dictionary \
    --source-dir ~/hl7-dictionary \
    --hl7-version 2.5 \
    --output ../../src-tauri/resources/hl7/v2_5.json
```

## Output shape

The tool emits a `HydratedSchema` with four arrays:

- **messages**: `[{ code, event, description, elements: [...] }]` where
  each `element` is one of `Segment`, `Group`, `Choice` (externally-tagged).
- **segments**: `[{ code, name, fields: [{ position, name, data_type, required, repeats }] }]`.
- **composites**: `[{ code, components: [{ position, name, data_type, required }] }]`.
- **primitives**: `[{ code }]`.

Validation failures (undefined segment references, undefined data types,
empty message list) abort the import before writing — so the shipped files
are always internally consistent.

## Roadmap

- [x] `bridgelab-json` round-trip (useful to re-format / validate existing files).
- [ ] `hl7-dictionary` ingestor: parse `lib/<version>/{messages,segments,fields,dataTypes}.js`.
- [ ] `hapi-conf` ingestor: parse HAPI's `.conf` XML bundles.
- [ ] Validation: flag composites/segments whose children reference each
      other cyclically (HL7 v2.5 has none by design, but HL7 v2.7+ does).
