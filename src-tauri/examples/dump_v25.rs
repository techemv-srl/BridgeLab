//! One-shot dump: `cargo run --example dump_v25 > resources/hl7/v2_5.json`
//!
//! Emits the current in-memory v2.5 schema as JSON. This is used to bootstrap
//! the data-driven loader (so the importer tool can replace the Rust data
//! file without touching the application crate).

fn main() {
    use bridgelab_lib::parser::hl7::schema::{v2_5, HydratedSchema};
    let s = v2_5::schema();
    // We build a serializable shape without the version enum embedded — the
    // loader will set the version from the filename.
    let hydrated = HydratedSchema {
        messages: s.messages,
        segments: s.segments,
        composites: s.composites,
        primitives: s.primitives,
    };
    let json = serde_json::to_string_pretty(&hydrated).unwrap();
    println!("{}", json);
}
