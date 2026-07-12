use std::path::{Path, PathBuf};

use nyaterm_core::{AppRuntime, SessionsConfig};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MigrationError {
    #[error("failed to read {path}: {source}")]
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("failed to parse {path}: {source}")]
    Parse {
        path: PathBuf,
        source: serde_json::Error,
    },
}

#[derive(Debug, Clone)]
pub struct LegacyProject {
    root: PathBuf,
}

impl LegacyProject {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn rust_source_dir(&self) -> PathBuf {
        self.root.join("src-tauri").join("src")
    }

    pub fn frontend_source_dir(&self) -> PathBuf {
        self.root.join("src")
    }

    pub fn package_json(&self) -> PathBuf {
        self.root.join("package.json")
    }

    pub fn cargo_toml(&self) -> PathBuf {
        self.root.join("src-tauri").join("Cargo.toml")
    }

    pub fn exists(&self) -> bool {
        self.package_json().exists() && self.cargo_toml().exists()
    }
}

#[derive(Debug, Clone)]
pub struct MigrationInventory {
    pub legacy_root: PathBuf,
    pub exists: bool,
    pub rust_files: usize,
    pub frontend_files: usize,
    pub command_modules: usize,
    pub copied_vendor_roots: Vec<PathBuf>,
}

pub fn inventory(legacy: &LegacyProject) -> MigrationInventory {
    MigrationInventory {
        legacy_root: legacy.root().to_path_buf(),
        exists: legacy.exists(),
        rust_files: count_files_with_extension(&legacy.rust_source_dir(), "rs"),
        frontend_files: count_frontend_files(&legacy.frontend_source_dir()),
        command_modules: count_files_with_extension(&legacy.rust_source_dir().join("cmd"), "rs"),
        copied_vendor_roots: vec![
            PathBuf::from("vendor/russh"),
            PathBuf::from("vendor/russh-sftp"),
            PathBuf::from("vendor/zmodem2"),
            PathBuf::from("crates/nyaterm-otp"),
        ],
    }
}

pub fn load_legacy_sessions(
    runtime: &AppRuntime,
) -> Result<Option<SessionsConfig>, MigrationError> {
    let candidates = [
        runtime.config_dir().join("settings").join("sessions.json"),
        runtime.config_dir().join("sessions.json"),
        runtime.config_dir().join("sessions").join("sessions.json"),
    ];

    for path in candidates {
        if !path.exists() {
            continue;
        }
        let content = std::fs::read_to_string(&path).map_err(|source| MigrationError::Read {
            path: path.clone(),
            source,
        })?;
        let config = serde_json::from_str::<SessionsConfig>(&content).map_err(|source| {
            MigrationError::Parse {
                path: path.clone(),
                source,
            }
        })?;
        return Ok(Some(config));
    }

    Ok(None)
}

fn count_frontend_files(root: &Path) -> usize {
    ["ts", "tsx", "css"]
        .iter()
        .map(|extension| count_files_with_extension(root, extension))
        .sum()
}

fn count_files_with_extension(root: &Path, extension: &str) -> usize {
    let Ok(entries) = std::fs::read_dir(root) else {
        return 0;
    };

    entries
        .flatten()
        .map(|entry| entry.path())
        .map(|path| {
            if path.is_dir() {
                count_files_with_extension(&path, extension)
            } else if path.extension().and_then(|value| value.to_str()) == Some(extension) {
                1
            } else {
                0
            }
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inventory_counts_legacy_source_files() {
        let root = unique_temp_dir("inventory");
        let rust_dir = root.join("src-tauri").join("src").join("cmd");
        let frontend_dir = root.join("src");
        std::fs::create_dir_all(&rust_dir).expect("rust dir");
        std::fs::create_dir_all(&frontend_dir).expect("frontend dir");
        std::fs::write(root.join("package.json"), "{}").expect("package json");
        std::fs::write(root.join("src-tauri").join("Cargo.toml"), "[package]\n").expect("cargo");
        std::fs::write(rust_dir.join("session.rs"), "").expect("rust file");
        std::fs::write(frontend_dir.join("App.tsx"), "").expect("tsx file");
        std::fs::write(frontend_dir.join("index.css"), "").expect("css file");

        let project = LegacyProject::new(&root);
        let inventory = inventory(&project);

        assert!(inventory.exists);
        assert_eq!(inventory.rust_files, 1);
        assert_eq!(inventory.command_modules, 1);
        assert_eq!(inventory.frontend_files, 2);

        std::fs::remove_dir_all(root).ok();
    }

    fn unique_temp_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("nyaterm-legacy-{name}-{}", std::process::id()));
        std::fs::remove_dir_all(&dir).ok();
        dir
    }
}
