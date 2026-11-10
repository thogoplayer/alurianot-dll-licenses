//! App settings, stored as JSON in the app's config folder.

use std::path::Path;

use serde::{Deserialize, Serialize};

pub const DEFAULT_WORKER: &str = "https://alurianot-dll-licenses.thiago-freitasferraz.workers.dev";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub repo_dir: String,
    pub keys_dir: String,
    pub worker_base: String,
}

impl Default for Settings {
    fn default() -> Self {
        let home = std::env::var("USERPROFILE").unwrap_or_else(|_| r"C:\Users\Public".into());
        Settings {
            repo_dir: format!(r"{home}\Documents\GitHub\alurianot-dll-licenses"),
            keys_dir: r"C:\tmp\licenses".into(),
            worker_base: DEFAULT_WORKER.into(),
        }
    }
}

/// Missing or unreadable file = defaults.
pub fn load(config_dir: &Path) -> Settings {
    std::fs::read_to_string(config_dir.join("settings.json"))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

pub fn save(config_dir: &Path, settings: &Settings) -> Result<(), String> {
    std::fs::create_dir_all(config_dir).map_err(|e| e.to_string())?;
    let text = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    std::fs::write(config_dir.join("settings.json"), text + "\n").map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_defaults_then_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let d = load(dir.path());
        assert_eq!(d.keys_dir, r"C:\tmp\licenses");
        assert_eq!(d.worker_base, DEFAULT_WORKER);
        assert!(d.repo_dir.ends_with(r"\Documents\GitHub\alurianot-dll-licenses"));
        let s = Settings { repo_dir: "r".into(), keys_dir: "k".into(), worker_base: "w".into() };
        save(dir.path(), &s).unwrap();
        assert_eq!(load(dir.path()), s);
    }
}
