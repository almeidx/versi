use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
    pub backend_shell_options: HashMap<String, ShellOptions>,

    #[serde(default, skip_serializing)]
    shell_options: Option<ShellOptions>,

    #[serde(default)]
    pub preferred_backend: Option<String>,

    #[serde(default)]
    pub debug_logging: bool,

    #[serde(default)]
    pub window_geometry: Option<WindowGeometry>,

    #[serde(default = "default_install_timeout")]
    pub install_timeout_secs: u64,

    #[serde(default = "default_operation_timeout")]
    pub uninstall_timeout_secs: u64,

    #[serde(default = "default_operation_timeout")]
    pub set_default_timeout_secs: u64,

    #[serde(default = "default_fetch_timeout")]
    pub fetch_timeout_secs: u64,

    #[serde(default = "default_http_timeout")]
    pub http_timeout_secs: u64,

    #[serde(default = "default_toast_timeout")]
    pub toast_timeout_secs: u64,

    #[serde(default = "default_max_visible_toasts")]
    pub max_visible_toasts: usize,

    #[serde(default = "default_search_results_limit")]
    pub search_results_limit: usize,

    #[serde(default = "default_modal_preview_limit")]
    pub modal_preview_limit: usize,

    #[serde(default = "default_max_log_size_bytes")]
    pub max_log_size_bytes: u64,

    #[serde(default = "default_retry_delays")]
    pub retry_delays_secs: Vec<u64>,
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

fn default_install_timeout() -> u64 {
    600
}

fn default_operation_timeout() -> u64 {
    60
}

fn default_fetch_timeout() -> u64 {
    30
}

fn default_http_timeout() -> u64 {
    10
}

fn default_toast_timeout() -> u64 {
    5
}

fn default_max_visible_toasts() -> usize {
    3
}

fn default_search_results_limit() -> usize {
    20
}

fn default_modal_preview_limit() -> usize {
    10
}

fn default_max_log_size_bytes() -> u64 {
    5 * 1024 * 1024
}

fn default_retry_delays() -> Vec<u64> {
    vec![0, 2, 5, 15]
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
            preferred_backend: None,
            backend_shell_options: HashMap::new(),
            shell_options: None,
            debug_logging: false,
            window_geometry: None,
            install_timeout_secs: default_install_timeout(),
            uninstall_timeout_secs: default_operation_timeout(),
            set_default_timeout_secs: default_operation_timeout(),
            fetch_timeout_secs: default_fetch_timeout(),
            http_timeout_secs: default_http_timeout(),
            toast_timeout_secs: default_toast_timeout(),
            max_visible_toasts: default_max_visible_toasts(),
            search_results_limit: default_search_results_limit(),
            modal_preview_limit: default_modal_preview_limit(),
            max_log_size_bytes: default_max_log_size_bytes(),
            retry_delays_secs: default_retry_delays(),
        }
    }
}

impl AppSettings {
    pub fn load() -> Self {
        let paths = AppPaths::new();
        let settings_path = paths.settings_file();

        let mut settings: Self = if settings_path.exists() {
            match std::fs::read_to_string(&settings_path) {
                Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
                Err(_) => Self::default(),
            }
        } else {
            Self::default()
        };

        if let Some(legacy) = settings.shell_options.take()
            && settings.backend_shell_options.is_empty()
        {
            settings
                .backend_shell_options
                .insert("fnm".to_string(), legacy);
        }

        settings
    }

    pub fn save(&self) -> Result<(), std::io::Error> {
        let paths = AppPaths::new();
        paths.ensure_dirs()?;

        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(paths.settings_file(), content)?;
        Ok(())
    }

    pub fn shell_options_for(&self, backend: &str) -> ShellOptions {
        self.backend_shell_options
            .get(backend)
            .cloned()
            .unwrap_or_default()
    }

    pub fn shell_options_for_mut(&mut self, backend: &str) -> &mut ShellOptions {
        self.backend_shell_options
            .entry(backend.to_string())
            .or_default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowGeometry {
    pub width: f32,
    pub height: f32,
    pub x: i32,
    pub y: i32,
}

impl WindowGeometry {
    pub fn is_likely_visible(&self) -> bool {
        const MIN_VISIBLE: i32 = -50;
        const MAX_COORD: i32 = 16384;
        const MIN_SIZE: f32 = 100.0;

        self.x > MIN_VISIBLE
            && self.y > MIN_VISIBLE
            && self.x < MAX_COORD
            && self.y < MAX_COORD
            && self.width >= MIN_SIZE
            && self.height >= MIN_SIZE
    }
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
