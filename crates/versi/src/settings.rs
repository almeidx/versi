use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use versi_platform::AppPaths;

use crate::backend_kind::BackendKind;

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
    pub launch_at_login: bool,

    #[serde(default)]
    pub fnm_dir: Option<PathBuf>,

    #[serde(default)]
    pub node_dist_mirror: Option<String>,

    #[serde(default)]
    #[serde(
        deserialize_with = "deserialize_backend_shell_options",
        serialize_with = "serialize_backend_shell_options"
    )]
    pub backend_shell_options: HashMap<BackendKind, ShellOptions>,

    #[serde(default, skip_serializing)]
    shell_options: Option<ShellOptions>,

    #[serde(default)]
    pub preferred_backend: Option<BackendKind>,

    #[serde(default)]
    pub debug_logging: bool,

    #[serde(default)]
    pub app_update_behavior: AppUpdateBehavior,

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

    #[serde(default = "default_wsl_list_timeout")]
    pub wsl_list_timeout_secs: u64,

    #[serde(default = "default_wsl_distro_timeout")]
    pub wsl_distro_timeout_secs: u64,

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

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
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

fn default_wsl_list_timeout() -> u64 {
    15
}

fn default_wsl_distro_timeout() -> u64 {
    10
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

const CACHE_TTL_HOURS_RANGE: std::ops::RangeInclusive<u64> = 1..=168;
const INSTALL_TIMEOUT_SECS_RANGE: std::ops::RangeInclusive<u64> = 30..=7_200;
const OPERATION_TIMEOUT_SECS_RANGE: std::ops::RangeInclusive<u64> = 5..=900;
const FETCH_TIMEOUT_SECS_RANGE: std::ops::RangeInclusive<u64> = 5..=300;
const WSL_LIST_TIMEOUT_SECS_RANGE: std::ops::RangeInclusive<u64> = 5..=120;
const WSL_DISTRO_TIMEOUT_SECS_RANGE: std::ops::RangeInclusive<u64> = 3..=60;
const HTTP_TIMEOUT_SECS_RANGE: std::ops::RangeInclusive<u64> = 3..=120;
const TOAST_TIMEOUT_SECS_RANGE: std::ops::RangeInclusive<u64> = 1..=60;
const MAX_VISIBLE_TOASTS_RANGE: std::ops::RangeInclusive<usize> = 1..=10;
const SEARCH_RESULTS_LIMIT_RANGE: std::ops::RangeInclusive<usize> = 1..=200;
const MODAL_PREVIEW_LIMIT_RANGE: std::ops::RangeInclusive<usize> = 1..=50;
const MAX_LOG_SIZE_BYTES_RANGE: std::ops::RangeInclusive<u64> = 1_024 * 1_024..=100 * 1_024 * 1_024;
const MAX_RETRY_DELAY_SECS: u64 = 600;
const MAX_RETRY_STEPS: usize = 8;

fn deserialize_backend_shell_options<'de, D>(
    deserializer: D,
) -> Result<HashMap<BackendKind, ShellOptions>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let raw = HashMap::<String, ShellOptions>::deserialize(deserializer)?;
    Ok(raw
        .into_iter()
        .filter_map(|(name, options)| BackendKind::from_name(&name).map(|kind| (kind, options)))
        .collect())
}

fn serialize_backend_shell_options<S>(
    map: &HashMap<BackendKind, ShellOptions>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let raw: HashMap<&str, &ShellOptions> = map
        .iter()
        .map(|(kind, options)| (kind.as_str(), options))
        .collect();
    raw.serialize(serializer)
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme: ThemeSetting::System,
            cache_ttl_hours: default_cache_ttl(),
            tray_behavior: TrayBehavior::WhenWindowOpen,
            start_minimized: false,
            launch_at_login: false,
            fnm_dir: None,
            node_dist_mirror: None,
            preferred_backend: None,
            backend_shell_options: HashMap::new(),
            shell_options: None,
            debug_logging: false,
            app_update_behavior: AppUpdateBehavior::default(),
            window_geometry: None,
            install_timeout_secs: default_install_timeout(),
            uninstall_timeout_secs: default_operation_timeout(),
            set_default_timeout_secs: default_operation_timeout(),
            fetch_timeout_secs: default_fetch_timeout(),
            wsl_list_timeout_secs: default_wsl_list_timeout(),
            wsl_distro_timeout_secs: default_wsl_distro_timeout(),
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum AppUpdateBehavior {
    DoNotCheck,
    #[default]
    CheckPeriodically,
    AutomaticallyUpdate,
}

impl AppSettings {
    pub fn load() -> Self {
        let Ok(paths) = AppPaths::new() else {
            return Self::default();
        };
        Self::load_from_path(&paths.settings_file())
    }

    pub fn save(&self) -> Result<(), std::io::Error> {
        let paths = AppPaths::new().map_err(std::io::Error::other)?;
        paths.ensure_dirs()?;

        self.save_to_path(&paths.settings_file())
    }

    pub fn shell_options_for(&self, backend: BackendKind) -> ShellOptions {
        self.backend_shell_options
            .get(&backend)
            .copied()
            .unwrap_or_default()
    }

    pub fn shell_options_for_mut(&mut self, backend: BackendKind) -> &mut ShellOptions {
        self.backend_shell_options.entry(backend).or_default()
    }

    fn load_from_path(settings_path: &Path) -> Self {
        let mut settings: Self = if settings_path.exists() {
            match std::fs::read_to_string(settings_path) {
                Ok(content) => match serde_json::from_str(&content) {
                    Ok(settings) => settings,
                    Err(error) => {
                        warn_settings_io(&format!(
                            "Failed to parse settings file at {}: {error}",
                            settings_path.display()
                        ));
                        quarantine_invalid_settings_file(settings_path);
                        Self::default()
                    }
                },
                Err(error) => {
                    warn_settings_io(&format!(
                        "Failed to read settings file at {}: {error}",
                        settings_path.display()
                    ));
                    Self::default()
                }
            }
        } else {
            Self::default()
        };

        if let Some(legacy) = settings.shell_options.take()
            && settings.backend_shell_options.is_empty()
        {
            settings
                .backend_shell_options
                .insert(BackendKind::Fnm, legacy);
        }

        if settings.sanitize_in_place() {
            warn_settings_io(
                "Loaded settings contained out-of-range values; defaults were applied where needed.",
            );
        }

        settings
    }

    fn save_to_path(&self, settings_path: &Path) -> Result<(), std::io::Error> {
        let mut settings = self.clone();
        if settings.sanitize_in_place() {
            warn_settings_io("Saving sanitized settings after clamping out-of-range values.");
        }

        let content = serde_json::to_vec_pretty(&settings)?;
        #[cfg(unix)]
        let parent = settings_path.parent().ok_or_else(|| {
            std::io::Error::other("settings path does not have a parent directory")
        })?;
        let temp_path = temp_settings_path(settings_path);

        {
            let mut file = std::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(&temp_path)?;
            file.write_all(&content)?;
            file.sync_all()?;
        }

        if let Err(error) = replace_file(&temp_path, settings_path) {
            let _ = std::fs::remove_file(&temp_path);
            return Err(error);
        }

        #[cfg(unix)]
        if let Ok(dir_handle) = std::fs::File::open(parent) {
            let _ = dir_handle.sync_all();
        }

        Ok(())
    }

    fn sanitize_in_place(&mut self) -> bool {
        let mut changed = false;

        changed |= clamp_in_place(&mut self.cache_ttl_hours, &CACHE_TTL_HOURS_RANGE);
        changed |= clamp_in_place(&mut self.install_timeout_secs, &INSTALL_TIMEOUT_SECS_RANGE);
        changed |= clamp_in_place(
            &mut self.uninstall_timeout_secs,
            &OPERATION_TIMEOUT_SECS_RANGE,
        );
        changed |= clamp_in_place(
            &mut self.set_default_timeout_secs,
            &OPERATION_TIMEOUT_SECS_RANGE,
        );
        changed |= clamp_in_place(&mut self.fetch_timeout_secs, &FETCH_TIMEOUT_SECS_RANGE);
        changed |= clamp_in_place(
            &mut self.wsl_list_timeout_secs,
            &WSL_LIST_TIMEOUT_SECS_RANGE,
        );
        changed |= clamp_in_place(
            &mut self.wsl_distro_timeout_secs,
            &WSL_DISTRO_TIMEOUT_SECS_RANGE,
        );
        changed |= clamp_in_place(&mut self.http_timeout_secs, &HTTP_TIMEOUT_SECS_RANGE);
        changed |= clamp_in_place(&mut self.toast_timeout_secs, &TOAST_TIMEOUT_SECS_RANGE);
        changed |= clamp_in_place(&mut self.max_visible_toasts, &MAX_VISIBLE_TOASTS_RANGE);
        changed |= clamp_in_place(&mut self.search_results_limit, &SEARCH_RESULTS_LIMIT_RANGE);
        changed |= clamp_in_place(&mut self.modal_preview_limit, &MODAL_PREVIEW_LIMIT_RANGE);
        changed |= clamp_in_place(&mut self.max_log_size_bytes, &MAX_LOG_SIZE_BYTES_RANGE);

        let original_retry_delays = self.retry_delays_secs.clone();
        self.retry_delays_secs
            .retain(|delay| *delay <= MAX_RETRY_DELAY_SECS);
        if self.retry_delays_secs.len() > MAX_RETRY_STEPS {
            self.retry_delays_secs.truncate(MAX_RETRY_STEPS);
        }
        if self.retry_delays_secs.is_empty() {
            self.retry_delays_secs = default_retry_delays();
        }
        changed |= self.retry_delays_secs != original_retry_delays;

        changed
    }
}

fn clamp_in_place<T: Ord + Copy>(value: &mut T, range: &std::ops::RangeInclusive<T>) -> bool {
    let clamped = (*value).clamp(*range.start(), *range.end());
    let changed = clamped != *value;
    *value = clamped;
    changed
}

fn warn_settings_io(message: &str) {
    eprintln!("Versi settings warning: {message}");
    log::warn!("{message}");
}

fn quarantine_invalid_settings_file(settings_path: &Path) {
    use std::time::{SystemTime, UNIX_EPOCH};

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let file_name = settings_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("settings.json");
    let mut last_error = None;

    for attempt in 0..5 {
        let suffix = if attempt == 0 {
            format!("{file_name}.corrupt-{timestamp}")
        } else {
            format!("{file_name}.corrupt-{timestamp}-{attempt}")
        };
        let backup_path = settings_path.with_file_name(suffix);

        match std::fs::rename(settings_path, &backup_path) {
            Ok(()) => {
                warn_settings_io(&format!(
                    "Moved invalid settings file to {}",
                    backup_path.display()
                ));
                return;
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return,
            Err(error) => {
                last_error = Some(error);
            }
        }
    }

    if let Some(error) = last_error {
        warn_settings_io(&format!(
            "Failed to quarantine invalid settings file {}: {error}",
            settings_path.display()
        ));
    }
}

fn temp_settings_path(settings_path: &Path) -> PathBuf {
    use std::time::{SystemTime, UNIX_EPOCH};

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let file_name = settings_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("settings.json");
    settings_path.with_file_name(format!(
        "{file_name}.tmp-{}-{timestamp}",
        std::process::id()
    ))
}

fn replace_file(src: &Path, dst: &Path) -> Result<(), std::io::Error> {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::Storage::FileSystem::{
            MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
        };

        let src_utf16: Vec<u16> = src
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let dst_utf16: Vec<u16> = dst
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        // SAFETY: both paths are NUL-terminated UTF-16 buffers that live for
        // the duration of the FFI call.
        let moved = unsafe {
            MoveFileExW(
                src_utf16.as_ptr(),
                dst_utf16.as_ptr(),
                MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
            )
        };
        if moved != 0 {
            Ok(())
        } else {
            Err(std::io::Error::last_os_error())
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        std::fs::rename(src, dst)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowGeometry {
    pub width: f32,
    pub height: f32,
    pub x: f32,
    pub y: f32,
}

impl WindowGeometry {
    pub fn is_likely_visible(&self) -> bool {
        const MIN_VISIBLE: f32 = -50.0;
        const MAX_COORD: f32 = 16_384.0;
        const MIN_SIZE: f32 = 100.0;

        self.x > MIN_VISIBLE
            && self.y > MIN_VISIBLE
            && self.x < MAX_COORD
            && self.y < MAX_COORD
            && self.width >= MIN_SIZE
            && self.height >= MIN_SIZE
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub enum ThemeSetting {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub enum TrayBehavior {
    #[default]
    WhenWindowOpen,
    AlwaysRunning,
    Disabled,
}

#[cfg(test)]
mod tests {
    use std::fs;

    use serde_json::json;
    use tempfile::tempdir;

    use super::{
        AppSettings, AppUpdateBehavior, BackendKind, ShellOptions, ThemeSetting, WindowGeometry,
    };

    #[test]
    fn shell_options_default_enables_use_on_cd_only() {
        let options = ShellOptions::default();

        assert!(options.use_on_cd);
        assert!(!options.resolve_engines);
        assert!(!options.corepack_enabled);
    }

    #[test]
    fn app_settings_defaults_match_expected_timeouts() {
        let settings = AppSettings::default();

        assert_eq!(settings.cache_ttl_hours, 1);
        assert_eq!(settings.install_timeout_secs, 600);
        assert_eq!(settings.uninstall_timeout_secs, 60);
        assert_eq!(settings.set_default_timeout_secs, 60);
        assert_eq!(settings.fetch_timeout_secs, 30);
        assert_eq!(settings.wsl_list_timeout_secs, 15);
        assert_eq!(settings.wsl_distro_timeout_secs, 10);
        assert_eq!(settings.http_timeout_secs, 10);
        assert_eq!(settings.toast_timeout_secs, 5);
        assert_eq!(settings.max_visible_toasts, 3);
        assert_eq!(settings.search_results_limit, 20);
        assert_eq!(settings.modal_preview_limit, 10);
        assert_eq!(settings.max_log_size_bytes, 5 * 1024 * 1024);
        assert_eq!(settings.retry_delays_secs, vec![0, 2, 5, 15]);
        assert_eq!(
            settings.app_update_behavior,
            AppUpdateBehavior::CheckPeriodically
        );
    }

    #[test]
    fn backend_shell_options_deserialization_keeps_known_backends() {
        let value = json!({
            "backend_shell_options": {
                "fnm": { "use_on_cd": true, "resolve_engines": true, "corepack_enabled": false },
                "nvm": { "use_on_cd": false, "resolve_engines": false, "corepack_enabled": true },
                "volta": { "use_on_cd": true, "resolve_engines": true, "corepack_enabled": true },
                "unknown-backend": { "use_on_cd": true, "resolve_engines": false, "corepack_enabled": false }
            }
        });

        let settings: AppSettings =
            serde_json::from_value(value).expect("settings JSON should deserialize");

        assert_eq!(settings.backend_shell_options.len(), 3);
        assert!(
            settings
                .backend_shell_options
                .contains_key(&BackendKind::Fnm)
        );
        assert!(
            settings
                .backend_shell_options
                .contains_key(&BackendKind::Nvm)
        );
        assert!(
            settings
                .backend_shell_options
                .contains_key(&BackendKind::Volta)
        );
    }

    #[test]
    fn backend_shell_options_serialization_uses_backend_names() {
        let mut settings = AppSettings::default();
        settings.backend_shell_options.insert(
            BackendKind::Fnm,
            ShellOptions {
                use_on_cd: false,
                resolve_engines: true,
                corepack_enabled: true,
            },
        );
        settings.backend_shell_options.insert(
            BackendKind::Volta,
            ShellOptions {
                use_on_cd: true,
                resolve_engines: false,
                corepack_enabled: false,
            },
        );

        let value = serde_json::to_value(settings).expect("settings should serialize");
        let backend_options = &value["backend_shell_options"];

        assert!(backend_options.get("fnm").is_some());
        assert!(backend_options.get("volta").is_some());
        assert!(backend_options.get("nvm").is_none());
    }

    #[test]
    fn shell_options_for_returns_default_if_not_overridden() {
        let settings = AppSettings::default();

        let options = settings.shell_options_for(BackendKind::Nvm);

        assert_eq!(options.use_on_cd, ShellOptions::default().use_on_cd);
        assert_eq!(
            options.resolve_engines,
            ShellOptions::default().resolve_engines
        );
        assert_eq!(
            options.corepack_enabled,
            ShellOptions::default().corepack_enabled
        );
    }

    #[test]
    fn shell_options_for_mut_inserts_default_entry() {
        let mut settings = AppSettings::default();

        settings
            .shell_options_for_mut(BackendKind::Fnm)
            .resolve_engines = true;

        let stored = settings
            .backend_shell_options
            .get(&BackendKind::Fnm)
            .expect("mutable accessor should insert missing backend options");
        assert!(stored.resolve_engines);
        assert!(stored.use_on_cd);
    }

    #[test]
    fn window_geometry_visibility_checks_bounds() {
        let visible = WindowGeometry {
            width: 900.0,
            height: 600.0,
            x: 200.0,
            y: 100.0,
        };
        assert!(visible.is_likely_visible());

        let too_small = WindowGeometry {
            width: 90.0,
            height: 99.0,
            x: 0.0,
            y: 0.0,
        };
        assert!(!too_small.is_likely_visible());

        let out_of_bounds = WindowGeometry {
            width: 900.0,
            height: 600.0,
            x: 20_000.0,
            y: 100.0,
        };
        assert!(!out_of_bounds.is_likely_visible());
    }

    #[test]
    fn sanitize_clamps_out_of_range_settings_values() {
        let mut settings = AppSettings {
            cache_ttl_hours: 0,
            install_timeout_secs: 1,
            uninstall_timeout_secs: 9_999,
            set_default_timeout_secs: 0,
            fetch_timeout_secs: 999,
            wsl_list_timeout_secs: 0,
            wsl_distro_timeout_secs: 999,
            http_timeout_secs: 0,
            toast_timeout_secs: 0,
            max_visible_toasts: 0,
            search_results_limit: 999,
            modal_preview_limit: 0,
            max_log_size_bytes: 1,
            retry_delays_secs: vec![900, 800, 700],
            ..AppSettings::default()
        };

        let changed = settings.sanitize_in_place();

        assert!(changed);
        assert_eq!(settings.cache_ttl_hours, 1);
        assert_eq!(settings.install_timeout_secs, 30);
        assert_eq!(settings.uninstall_timeout_secs, 900);
        assert_eq!(settings.set_default_timeout_secs, 5);
        assert_eq!(settings.fetch_timeout_secs, 300);
        assert_eq!(settings.wsl_list_timeout_secs, 5);
        assert_eq!(settings.wsl_distro_timeout_secs, 60);
        assert_eq!(settings.http_timeout_secs, 3);
        assert_eq!(settings.toast_timeout_secs, 1);
        assert_eq!(settings.max_visible_toasts, 1);
        assert_eq!(settings.search_results_limit, 200);
        assert_eq!(settings.modal_preview_limit, 1);
        assert_eq!(settings.max_log_size_bytes, 1_024 * 1_024);
        assert_eq!(settings.retry_delays_secs, vec![0, 2, 5, 15]);
    }

    #[test]
    fn load_from_path_quarantines_invalid_json() {
        let temp_dir = tempdir().expect("create temp dir");
        let settings_path = temp_dir.path().join("settings.json");
        fs::write(&settings_path, "{ invalid json ").expect("write invalid settings");

        let loaded = AppSettings::load_from_path(&settings_path);

        assert!(matches!(loaded.theme, ThemeSetting::System));
        assert!(!settings_path.exists());

        let quarantined_files: Vec<_> = fs::read_dir(temp_dir.path())
            .expect("read temp directory")
            .filter_map(Result::ok)
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .filter(|name| name.starts_with("settings.json.corrupt-"))
            .collect();
        assert_eq!(quarantined_files.len(), 1);
    }

    #[test]
    fn save_to_path_writes_replacement_file_without_temp_leftovers() {
        let temp_dir = tempdir().expect("create temp dir");
        let settings_path = temp_dir.path().join("settings.json");

        let first = AppSettings {
            theme: ThemeSetting::Dark,
            ..AppSettings::default()
        };
        first
            .save_to_path(&settings_path)
            .expect("save first settings payload");

        let second = AppSettings {
            theme: ThemeSetting::Light,
            search_results_limit: 1_000,
            ..AppSettings::default()
        };
        second
            .save_to_path(&settings_path)
            .expect("save second settings payload");

        let loaded = AppSettings::load_from_path(&settings_path);
        assert!(matches!(loaded.theme, ThemeSetting::Light));
        assert_eq!(loaded.search_results_limit, 200);

        let has_temp_files = fs::read_dir(temp_dir.path())
            .expect("read temp directory")
            .filter_map(Result::ok)
            .any(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .contains("settings.json.tmp-")
            });
        assert!(!has_temp_files);
    }
}
