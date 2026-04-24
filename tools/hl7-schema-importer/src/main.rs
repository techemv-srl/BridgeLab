//! BridgeLab HL7 schema importer.
//!
//! Ingests HL7 v2.x schema definitions from a supported source format and
//! emits the JSON payload consumed by `src-tauri/src/parser/hl7/schema/mod.rs`
//! (and therefore shipped as `resources/hl7/v<version>.json`).
//!
//! Supported source formats:
//!
//! * **hl7-dictionary**: https://github.com/Ensighten/hl7-dictionary (MIT).
//!   Expected layout under `--source-dir`:
//!
//!         lib/
//!           2.5/
//!             messages.js    (actually a CommonJS module, but the object
//!                             literal inside is JSON-compatible; this tool
//!                             strips the `module.exports = ` preamble and
//!                             the trailing `;`)
//!             segments.js
//!             fields.js
//!             dataTypes.js
//!
//! * **bridgelab-json**: the destination JSON format itself — used to
//!   re-format or validate an existing `v2_X.json` against the current
//!   schema model.
//!
//! The tool is intentionally conservative: it does *not* download anything.
//! Clone hl7-dictionary manually and point `--source-dir` at the checkout.
//!
//! Usage:
//!
//!     hl7-schema-importer \
//!         --format hl7-dictionary \
//!         --source-dir ./hl7-dictionary \
//!         --version 2.5 \
//!         --output ../../src-tauri/resources/hl7/v2_5.json

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use clap::{Parser, ValueEnum};
use serde::{Deserialize, Serialize};

#[derive(Parser, Debug)]
#[command(name = "hl7-schema-importer", version, about)]
struct Cli {
    /// Source layout to parse.
    #[arg(long, value_enum)]
    format: SourceFormat,

    /// Directory containing the source files.
    #[arg(long)]
    source_dir: PathBuf,

    /// HL7 version (e.g. 2.5, 2.3.1). Renamed to avoid clap's builtin --version.
    #[arg(long = "hl7-version")]
    hl7_version: String,

    /// Output file path.
    #[arg(long)]
    output: PathBuf,

    /// Pretty-print output JSON (default: on).
    #[arg(long, default_value_t = true)]
    pretty: bool,
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum SourceFormat {
    /// hl7-dictionary npm package layout (JS files with exported objects)
    Hl7Dictionary,
    /// Native BridgeLab JSON (for round-tripping / reformatting)
    BridgelabJson,
}

// ---------- the shared on-disk shape (mirrors schema::HydratedSchema) -------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageElement {
    Segment { code: String, required: bool, repeats: bool },
    Group { name: String, required: bool, repeats: bool, elements: Vec<MessageElement> },
    Choice { required: bool, repeats: bool, segments: Vec<String> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageStructure {
    pub code: String,
    pub event: String,
    pub description: String,
    pub elements: Vec<MessageElement>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldSpec {
    pub position: usize,
    pub name: String,
    pub data_type: String,
    pub required: bool,
    pub repeats: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentSpec {
    pub code: String,
    pub name: String,
    pub fields: Vec<FieldSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentSpec {
    pub position: usize,
    pub name: String,
    pub data_type: String,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositeType {
    pub code: String,
    pub components: Vec<ComponentSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimitiveType {
    pub code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HydratedSchema {
    pub messages: Vec<MessageStructure>,
    pub segments: Vec<SegmentSpec>,
    pub composites: Vec<CompositeType>,
    pub primitives: Vec<PrimitiveType>,
}

// ---------- entry point -----------------------------------------------------

fn main() -> Result<()> {
    let cli = Cli::parse();

    let schema = match cli.format {
        SourceFormat::BridgelabJson => ingest_bridgelab_json(&cli.source_dir, &cli.hl7_version)?,
        SourceFormat::Hl7Dictionary => ingest_hl7_dictionary(&cli.source_dir, &cli.hl7_version)?,
    };

    // Integrity checks — we want to fail loudly before writing a broken file.
    validate(&schema).context("imported schema failed validation")?;

    let mut json = if cli.pretty {
        serde_json::to_string_pretty(&schema)?
    } else {
        serde_json::to_string(&schema)?
    };
    json.push('\n'); // canonical trailing newline
    if let Some(parent) = cli.output.parent() {
        fs::create_dir_all(parent).ok();
    }
    fs::write(&cli.output, json).with_context(|| format!("writing {:?}", cli.output))?;
    eprintln!(
        "wrote {:?}: {} messages, {} segments, {} composites, {} primitives",
        cli.output,
        schema.messages.len(),
        schema.segments.len(),
        schema.composites.len(),
        schema.primitives.len(),
    );
    Ok(())
}

// ---------- bridgelab-json ingestor (identity / reformat) -------------------

fn ingest_bridgelab_json(dir: &std::path::Path, version: &str) -> Result<HydratedSchema> {
    // Accept either <dir>/v<version>.json or <dir>/<version>.json.
    let candidates = [
        dir.join(format!("v{}.json", version.replace('.', "_"))),
        dir.join(format!("{}.json", version.replace('.', "_"))),
        dir.join(format!("v{}.json", version)),
    ];
    for c in &candidates {
        if c.exists() {
            let raw = fs::read_to_string(c)?;
            return Ok(serde_json::from_str(&raw)?);
        }
    }
    bail!(
        "no bridgelab-json file found for version {}. Looked at: {:?}",
        version,
        candidates
    )
}

// ---------- hl7-dictionary ingestor -----------------------------------------

fn ingest_hl7_dictionary(_dir: &std::path::Path, _version: &str) -> Result<HydratedSchema> {
    // TODO (F2): parse hl7-dictionary's lib/<version>/{messages,segments,fields,dataTypes}.js
    // The files are CommonJS exports of plain JS objects with a well-known
    // shape; stripping the `module.exports =` prefix and trailing `;` yields
    // valid JSON that we can deserialize and map into our model.
    //
    // Implementation sketch:
    //   1. read messages.js -> HashMap<StructureCode, Vec<JsElement>>
    //      where JsElement is { name, min, max, items? }
    //   2. read segments.js -> HashMap<SegmentCode, { description, fields[] }>
    //      where fields[] is a list of { name, type, len, usage, rep_max, datatype }
    //   3. read fields.js + dataTypes.js -> composite and primitive tables
    //   4. translate into HydratedSchema, preserving 1-based positions and
    //      mapping "R"/"O" to required=true/false, rep_max>1 to repeats=true,
    //      and detecting groups vs. choices from the shape.
    //
    // Once implemented this is the primary path for F2 (full v2.5) and F3
    // (all other v2.x versions hl7-dictionary ships).
    bail!(
        "hl7-dictionary ingestor not yet implemented. \
         Clone https://github.com/Ensighten/hl7-dictionary and use \
         `--format bridgelab-json` with a pre-converted file for now."
    )
}

// ---------- validation ------------------------------------------------------

fn validate(s: &HydratedSchema) -> Result<()> {
    if s.messages.is_empty() {
        bail!("schema contains no messages");
    }
    // Every segment referenced by a message must be defined.
    for m in &s.messages {
        for code in walk_segments(&m.elements) {
            if !s.segments.iter().any(|sg| sg.code == code) {
                bail!("message {} references undefined segment {}", m.code, code);
            }
        }
    }
    // Every data type referenced by a segment field or composite component
    // must be defined as composite or primitive.
    for seg in &s.segments {
        for f in &seg.fields {
            if !has_type(s, &f.data_type) {
                bail!(
                    "segment {} field {}.{} references undefined type {}",
                    seg.code, seg.code, f.position, f.data_type
                );
            }
        }
    }
    for c in &s.composites {
        for comp in &c.components {
            if !has_type(s, &comp.data_type) {
                bail!(
                    "composite {} component {}.{} references undefined type {}",
                    c.code, c.code, comp.position, comp.data_type
                );
            }
        }
    }
    Ok(())
}

fn has_type(s: &HydratedSchema, code: &str) -> bool {
    s.composites.iter().any(|c| c.code == code)
        || s.primitives.iter().any(|p| p.code == code)
}

fn walk_segments(elements: &[MessageElement]) -> Vec<String> {
    let mut out = Vec::new();
    for e in elements {
        match e {
            MessageElement::Segment { code, .. } => out.push(code.clone()),
            MessageElement::Group { elements, .. } => out.extend(walk_segments(elements)),
            MessageElement::Choice { segments, .. } => out.extend(segments.clone()),
        }
    }
    out
}
