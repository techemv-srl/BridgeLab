fn main() {
    // The Tauri build step (config check, context generation) only makes
    // sense for the desktop app. bridgelab-cli builds this crate without
    // the `desktop` feature and must not need tauri.conf.json at all.
    if std::env::var_os("CARGO_FEATURE_DESKTOP").is_some() {
        tauri_build::build()
    }
}
