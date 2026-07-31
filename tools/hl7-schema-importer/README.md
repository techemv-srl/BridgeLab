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

# Ingest from hl7-dictionary (npm, MIT licensed). The dictionary ships
# CommonJS modules, so the conversion runs in Node; this tool then
# validates the output (referential integrity) before it ships:
npm pack hl7-dictionary && tar xzf hl7-dictionary-*.tgz
node scripts/convert-hl7-dictionary.mjs ./package 2.5 /tmp/v2_5.json
hl7-schema-importer \
    --format bridgelab-json \
    --source-dir /tmp \
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
- [x] hl7-dictionary conversion via `scripts/convert-hl7-dictionary.mjs` (F2: full v2.5 catalogue shipped — 248 messages, 149 segments, 78 composites).
- [ ] `hl7-dictionary` ingestor: parse `lib/<version>/{messages,segments,fields,dataTypes}.js`.
- [ ] `hapi-conf` ingestor: parse HAPI's `.conf` XML bundles.
- [ ] Validation: flag composites/segments whose children reference each
      other cyclically (HL7 v2.5 has none by design, but HL7 v2.7+ does).
