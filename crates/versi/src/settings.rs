use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use versi_platform::AppPaths;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    #[serde(default)]
    pub theme: ThemeSetting,

    #[serde(default = "default_cache_ttl")]
    pub cache_ttl_hours: u64,

    #[serde(default)]
    pub tray_behavior: TrayBehavior,

    #[serde(default)]
    pub start_minimized: bool,

    #[serde(default)]
    pub fnm_dir: Option<PathBuf>,

    #[serde(default)]
    pub node_dist_mirror: Option<String>,

    #[serde(default)]
    pub shell_options: ShellOptions,

    #[serde(default)]
    pub debug_logging: bool,

    #[serde(default)]
    pub window_geometry: Option<WindowGeometry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellOptions {
    #[serde(default = "default_true")]
    pub use_on_cd: bool,

    #[serde(default)]
    pub resolve_engines: bool,

    #[serde(default)]
    pub corepack_enabled: bool,
}

fn default_true() -> bool {
    true
}

impl Default for ShellOptions {
    fn default() -> Self {
        Self {
            use_on_cd: true,
            resolve_engines: false,
            corepack_enabled: false,
        }
    }
}

fn default_cache_ttl() -> u64 {
    1
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme: ThemeSetting::System,
            cache_ttl_hours: 1,
            tray_behavior: TrayBehavior::WhenWindowOpen,
            start_minimized: false,
            fnm_dir: None,
            node_dist_mirror: None,
            shell_options: ShellOptions::default(),
            debug_logging: false,
            window_geometry: None,
        }
    }
}

impl AppSettings {
    pub fn load() -> Self {
        let paths = AppPaths::new();
        let settings_path = paths.settings_file();

        if settings_path.exists() {
            match std::fs::read_to_string(&settings_path) {
                Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
                Err(_) => Self::default(),
            }
        } else {
            Self::default()
        }
    }

    pub fn save(&self) -> Result<(), std::io::Error> {
        let paths = AppPaths::new();
        paths.ensure_dirs()?;

        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(paths.settings_file(), content)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowGeometry {
    pub width: f32,
    pub height: f32,
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub enum ThemeSetting {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub enum TrayBehavior {
    #[default]
    WhenWindowOpen,
    AlwaysRunning,
    Disabled,
}
