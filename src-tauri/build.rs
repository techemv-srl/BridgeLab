use std::path::PathBuf;
use std::process::Command;

fn main() {
    emit_build_id();

    // The Tauri build step (config check, context generation) only makes
    // sense for the desktop app. bridgelab-cli builds this crate without
    // the `desktop` feature and must not need tauri.conf.json at all.
    if std::env::var_os("CARGO_FEATURE_DESKTOP").is_some() {
        tauri_build::build()
    }
}

/// Sets `BRIDGELAB_BUILD`, the build code shown next to the version (About,
/// `bridgelab-cli --version`), so two builds of the same version can be
/// told apart: the commit date and short hash, e.g. `20261117.762cc1c`.
/// A non-empty `BRIDGELAB_BUILD` in the environment wins (a build without
/// a Git checkout, such as a Docker context); with neither it is `unknown`.
fn emit_build_id() {
    println!("cargo:rerun-if-env-changed=BRIDGELAB_BUILD");
    let id = match std::env::var("BRIDGELAB_BUILD") {
        Ok(v) if !v.trim().is_empty() => v.trim().to_string(),
        _ => {
            watch_git_head();
            match (
                git(&["log", "-1", "--format=%cd", "--date=format:%Y%m%d"]),
                git(&["rev-parse", "--short=7", "HEAD"]),
            ) {
                (Some(date), Some(hash)) => format!("{date}.{hash}"),
                _ => "unknown".to_string(),
            }
        }
    };
    println!("cargo:rustc-env=BRIDGELAB_BUILD={id}");
}

/// Re-runs this script when the checked-out commit changes: HEAD, the
/// branch it points to, and packed refs (worktrees keep HEAD in their own
/// Git directory and refs in the common one).
fn watch_git_head() {
    let (Some(git_dir), Some(common_dir)) = (
        git(&["rev-parse", "--git-dir"]),
        git(&["rev-parse", "--git-common-dir"]),
    ) else {
        return;
    };
    let (git_dir, common_dir) = (PathBuf::from(git_dir), PathBuf::from(common_dir));
    let mut watched = vec![git_dir.join("HEAD"), common_dir.join("packed-refs")];
    if let Some(head_ref) = git(&["symbolic-ref", "-q", "HEAD"]) {
        // A branch that lives only in packed-refs (after `git gc`) gets its
        // loose ref file back with the next commit, which touches neither
        // HEAD nor packed-refs: watch the folder it will appear in.
        let mut path = common_dir.join(head_ref);
        while !path.exists() && path != common_dir && path.pop() {}
        watched.push(path);
    }
    // A missing path would make Cargo re-run the script on every build.
    for path in watched.into_iter().filter(|p| p.exists()) {
        println!("cargo:rerun-if-changed={}", path.display());
    }
}

fn git(args: &[&str]) -> Option<String> {
    let out = Command::new("git").args(args).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8(out.stdout).ok()?.trim().to_string();
    (!s.is_empty()).then_some(s)
}
