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

## Shipped versions

| Version | Messages | Segments | Composites |
| ------- | -------: | -------: | ---------: |
| 2.1     |       39 |       38 |          8 |
| 2.2     |       69 |       62 |         52 |
| 2.3     |      239 |      112 |         75 |
| 2.3.1   |      176 |      111 |         78 |
| 2.4     |      220 |      138 |         80 |
| 2.5     |      248 |      149 |         78 |
| 2.5.1   |      248 |      149 |         78 |
| 2.6     |      371 |      175 |         78 |
| 2.7     |      355 |      171 |         74 |

HL7 **v2.7.1** ships no payload of its own: it is a technical-correction
release of v2.7, and hl7-dictionary holds byte-identical definitions for the
two. It is declared an alias of v2.7 in `Hl7Version::aliases()`, so MSH-12 =
2.7.1 resolves to the v2.7 catalogue instead of embedding the same 2 MB twice
or falling back to the default version. `non_alias_versions_have_distinct_catalogues`
fails the build if any other pair of versions ever ends up with identical
payloads without being declared an alias.

To regenerate every shipped file after a dictionary bump:

```bash
npm pack hl7-dictionary && tar xzf hl7-dictionary-*.tgz
cargo build --release
for v in 2.1 2.2 2.3 2.3.1 2.4 2.5 2.5.1 2.6 2.7; do
    f="v${v//./_}"
    node scripts/convert-hl7-dictionary.mjs ./package "$v" "/tmp/$f.json"
    ./target/release/hl7-schema-importer --format bridgelab-json \
        --source-dir /tmp --hl7-version "$v" \
        --output "../../src-tauri/resources/hl7/$f.json"
done
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
- [x] hl7-dictionary conversion via `scripts/convert-hl7-dictionary.mjs`, then
      validated here — every version hl7-dictionary covers is shipped (see
      the table above): 2,320 message structures across 10 selectable versions.
- [ ] Native Rust `hl7-dictionary` ingestor, dropping the Node step: parse
      `lib/<version>/{messages,segments,fields}.js` directly.
- [ ] `hapi-conf` ingestor: parse HAPI's `.conf` XML bundles. Needed for
      HL7 v2.8+, which hl7-dictionary (last published 2015) does not cover.
- [ ] Validation: flag composites/segments whose children reference each
      other cyclically (HL7 v2.5 has none by design, but HL7 v2.7+ does).
