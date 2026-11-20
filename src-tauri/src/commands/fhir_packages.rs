//! IPC commands for the FHIR profile package manager.

use std::path::{Path, PathBuf};

use tauri::State;

use super::validation::Registries;
use crate::licensing::feature_gate;
use crate::parser::fhir::profile::{package, PackageInfo, ProfileRegistry};

/// Packages currently loaded.
#[tauri::command]
pub fn fhir_packages_list(registry: State<'_, ProfileRegistry>) -> Vec<PackageInfo> {
    registry.packages()
}

/// Re-read the installed packages from disk.
#[tauri::command]
pub fn fhir_packages_reload(registry: State<'_, ProfileRegistry>) -> Result<Vec<PackageInfo>, String> {
    registry.reload()?;
    Ok(registry.packages())
}

/// Install a `.tgz` the user picked, then reload.
///
/// Reading the archive is the slow part (a core package holds thousands of
/// files), so it happens once here and the distilled index is what the app
/// loads from then on. On a blocking-task thread, so a large package
/// freezes neither the window nor the async workers every other command
/// (open, save…) runs on.
#[tauri::command]
pub async fn fhir_packages_install(path: String, app: tauri::AppHandle) -> Result<Vec<PackageInfo>, String> {
    feature_gate::require("fhir_profile_validation")?;
    install_off_thread(PathBuf::from(path), app).await
}

pub(crate) async fn install_off_thread(path: PathBuf, states: impl Registries) -> Result<Vec<PackageInfo>, String> {
    tauri::async_runtime::spawn_blocking(move || install_and_reload(&path, states.profiles()))
        .await
        .map_err(|e| format!("The package could not be installed: {}", e))?
}

fn install_and_reload(path: &Path, registry: &ProfileRegistry) -> Result<Vec<PackageInfo>, String> {
    package::install(path)?;
    registry.reload()?;
    Ok(registry.packages())
}

#[tauri::command]
pub fn fhir_packages_remove(
    name: String,
    version: String,
    registry: State<'_, ProfileRegistry>,
) -> Result<Vec<PackageInfo>, String> {
    package::remove(&name, &version)?;
    registry.reload()?;
    Ok(registry.packages())
}

/// Directory the packages live in, so the UI can offer to open it.
#[tauri::command]
pub fn fhir_packages_dir() -> Result<String, String> {
    package::packages_root()
        .map(|p| p.display().to_string())
        .ok_or_else(|| "Could not determine the config directory".into())
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use crate::message_store::MessageStore;
    use crate::plugins::PluginRegistry;
    use std::sync::Arc;
    use std::time::Duration;

    struct TestStates {
        store: MessageStore,
        plugins: PluginRegistry,
        profiles: ProfileRegistry,
    }

    impl Registries for Arc<TestStates> {
        fn store(&self) -> &MessageStore {
            &self.store
        }
        fn plugins(&self) -> &PluginRegistry {
            &self.plugins
        }
        fn profiles(&self) -> &ProfileRegistry {
            &self.profiles
        }
    }

    /// A named pipe picked as a package is refused at once. Its read used
    /// to wait for a writer forever, each on one of the async workers: with
    /// four of them pending (one per core) open and save stopped answering.
    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn installing_a_named_pipe_is_refused_at_once() {
        let dir = std::env::temp_dir().join(format!("bl-pkg-fifo-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let states = Arc::new(TestStates {
            store: MessageStore::new(),
            plugins: PluginRegistry::with_root(dir.join("plugins")),
            profiles: ProfileRegistry::new(),
        });
        let installs: Vec<_> = (0..5)
            .map(|i| {
                let fifo = dir.join(format!("p{i}.tgz"));
                let c = std::ffi::CString::new(fifo.to_str().unwrap()).unwrap();
                assert_eq!(unsafe { libc::mkfifo(c.as_ptr(), 0o600) }, 0);
                tokio::spawn(install_off_thread(fifo, states.clone()))
            })
            .collect();
        // Another command gets its turn on the one worker meanwhile.
        tokio::time::timeout(Duration::from_secs(2), tokio::spawn(async {}))
            .await
            .expect("another command waited for the installs")
            .unwrap();
        for install in installs {
            let refused = tokio::time::timeout(Duration::from_secs(10), install)
                .await
                .expect("the install of a named pipe never ended")
                .unwrap()
                .unwrap_err();
            assert!(refused.contains("is not a file") && refused.contains("named pipe"), "{refused}");
        }
        std::fs::remove_dir_all(&dir).ok();
    }
}
