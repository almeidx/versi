use log::{debug, info, trace};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use iced::Task;

use versi_backend::{BackendDetection, BackendProvider, VersionManager};
use versi_platform::EnvironmentId;
use versi_shell::detect_shells;

use crate::backend_kind::BackendKind;
use crate::error::AppError;
use crate::message::{EnvironmentInfo, InitResult, Message};
use crate::state::{
    AppState, BackendOption, EnvironmentState, MainState, OnboardingState, ShellConfigStatus,
};

use super::Versi;
use super::async_helpers::run_with_timeout;

impl Versi {
    pub(super) fn handle_initialized(&mut self, result: InitResult) -> Task<Message> {
        versi_core::auto_update::cleanup_old_app_bundle();

        info!(
            "Handling initialization result: backend_found={}, environments={}",
            result.backend_found,
            result.environments.len()
        );

        if !result.backend_found {
            return self.enter_onboarding_flow();
        }

        let native_env = result.environments.first();
        let active_backend_name = native_env.map_or(BackendKind::DEFAULT, |e| e.backend_name);

        let (backend_path, backend_dir, backend) =
            self.prepare_main_backend(&result, active_backend_name);

        let mut main_state = MainState::new_with_environments(
            backend,
            build_environment_states(&result),
            active_backend_name,
        );
        main_state.detected_backends = result.detected_backends;
        load_disk_cache_into_state(&mut main_state);

        self.state = AppState::Main(Box::new(main_state));

        self.mark_non_active_environments_not_loading();

        let mut tasks = Vec::new();
        if let Some(task) = self.build_active_environment_load_task(
            &result.environments,
            &backend_path,
            result.backend_in_path,
            backend_dir.as_deref(),
        ) {
            tasks.push(task);
        }
        tasks.extend(self.build_post_init_tasks());

        Task::batch(tasks)
    }

    fn enter_onboarding_flow(&mut self) -> Task<Message> {
        info!("No backend found, entering onboarding flow");
        let shell_statuses = detect_onboarding_shell_statuses();
        let onboarding = OnboardingState {
            detected_shells: shell_statuses,
            available_backends: self.available_backend_options_for_onboarding(),
            ..Default::default()
        };
        self.state = AppState::Onboarding(onboarding);
        Task::none()
    }

    fn available_backend_options_for_onboarding(&self) -> Vec<BackendOption> {
        self.providers
            .iter()
            .map(|(kind, p)| BackendOption {
                kind: *kind,
                display_name: p.display_name(),
                detected: false,
            })
            .collect()
    }

    fn prepare_main_backend(
        &mut self,
        result: &InitResult,
        active_backend_name: BackendKind,
    ) -> (PathBuf, Option<PathBuf>, Arc<dyn VersionManager>) {
        if let Some(provider) = self.providers.get(&active_backend_name) {
            self.provider = provider.clone();
        }

        let backend_path = result
            .backend_path
            .clone()
            .unwrap_or_else(|| PathBuf::from(self.provider.name()));
        let backend_dir = result.backend_dir.clone();

        self.backend_path.clone_from(&backend_path);
        self.backend_in_path = result.backend_in_path;
        self.backend_dir.clone_from(&backend_dir);

        let detection = BackendDetection {
            found: true,
            path: Some(backend_path.clone()),
            version: result.backend_version.clone(),
            in_path: result.backend_in_path,
            data_dir: backend_dir.clone(),
        };
        let backend = self.provider.create_manager(&detection);
        (backend_path, backend_dir, backend)
    }

    fn build_active_environment_load_task(
        &mut self,
        environments: &[EnvironmentInfo],
        backend_path: &Path,
        backend_in_path: bool,
        backend_dir: Option<&Path>,
    ) -> Option<Task<Message>> {
        let active_env = environments.first()?;
        if !active_env.available {
            debug!(
                "Skipping startup load for unavailable active environment: {:?}",
                active_env.id
            );
            return None;
        }

        self.build_environment_load_task(active_env, backend_path, backend_in_path, backend_dir)
    }

    fn build_environment_load_task(
        &mut self,
        env_info: &EnvironmentInfo,
        backend_path: &Path,
        backend_in_path: bool,
        backend_dir: Option<&Path>,
    ) -> Option<Task<Message>> {
        let env_id = env_info.id.clone();
        let provider = self
            .providers
            .get(&env_info.backend_name)
            .cloned()
            .unwrap_or_else(|| self.provider.clone());

        let backend = create_backend_for_environment(
            &env_id,
            backend_path,
            backend_in_path,
            backend_dir,
            &provider,
        );
        let request_seq = self.mark_environment_loading(&env_id)?;

        let fetch_timeout = std::time::Duration::from_secs(self.settings.fetch_timeout_secs);
        Some(Task::perform(
            async move {
                let result = run_with_timeout(
                    fetch_timeout,
                    "Loading versions",
                    backend.list_installed(),
                    AppError::environment_load_failed,
                )
                .await;
                (env_id, request_seq, result)
            },
            move |(env_id, request_seq, result)| Message::EnvironmentLoaded {
                env_id,
                request_seq,
                result,
            },
        ))
    }

    fn mark_environment_loading(&mut self, env_id: &EnvironmentId) -> Option<u64> {
        let AppState::Main(state) = &mut self.state else {
            return None;
        };
        let env = state.environments.iter_mut().find(|e| &e.id == env_id)?;
        env.loading = true;
        env.error = None;
        env.load_request_seq = env.load_request_seq.wrapping_add(1);
        Some(env.load_request_seq)
    }

    fn mark_non_active_environments_not_loading(&mut self) {
        let AppState::Main(state) = &mut self.state else {
            return;
        };

        for (idx, env) in state.environments.iter_mut().enumerate() {
            if idx != state.active_environment_idx && env.available {
                env.loading = false;
                env.load_cancel_token = None;
            }
        }
    }

    fn build_post_init_tasks(&mut self) -> [Task<Message>; 6] {
        [
            self.handle_fetch_remote_versions(),
            self.handle_fetch_release_schedule(),
            self.handle_fetch_version_metadata(),
            self.handle_fetch_security_advisories(),
            self.handle_check_for_app_update(),
            self.handle_check_for_backend_update(),
        ]
    }
}

fn detect_onboarding_shell_statuses() -> Vec<ShellConfigStatus> {
    let shells = detect_shells();
    debug!("Detected {} shells for configuration", shells.len());
    shells
        .into_iter()
        .map(|shell| ShellConfigStatus {
            shell_type: shell.shell_type,
            shell_name: shell.shell_type.name().to_string(),
            configured: shell.is_configured,
            config_path: shell.config_file,
            configuring: false,
            error: None,
        })
        .collect()
}

fn build_environment_states(result: &InitResult) -> Vec<EnvironmentState> {
    result
        .environments
        .iter()
        .map(|env_info| {
            if env_info.available {
                EnvironmentState::new(
                    env_info.id.clone(),
                    env_info.backend_name,
                    env_info.backend_version.clone(),
                )
            } else {
                EnvironmentState::unavailable(
                    env_info.id.clone(),
                    env_info.backend_name,
                    env_info
                        .unavailable_reason
                        .as_deref()
                        .unwrap_or("Unavailable"),
                )
            }
        })
        .collect()
}

fn load_disk_cache_into_state(main_state: &mut MainState) {
    match crate::cache::DiskCache::load() {
        Ok(Some(disk_cache)) => {
            debug!(
                "Loaded disk cache from {:?} ({} versions, schedule={})",
                disk_cache.cached_at,
                disk_cache.remote_versions.len(),
                disk_cache.release_schedule.is_some()
            );
            main_state.available_versions.disk_cached_at = Some(disk_cache.cached_at);
            if !disk_cache.remote_versions.is_empty() {
                main_state
                    .available_versions
                    .set_versions(disk_cache.remote_versions);
                main_state.available_versions.loaded_from_disk = true;
            }
            if let Some(schedule) = disk_cache.release_schedule {
                main_state.available_versions.schedule = Some(schedule);
            }
            if let Some(metadata) = disk_cache.version_metadata {
                main_state.available_versions.metadata = Some(Arc::new(metadata));
            }
            if let Some(advisories) = disk_cache.security_advisories {
                main_state.available_versions.security_advisories = Some(Arc::new(advisories));
            }
        }
        Ok(None) => {}
        Err(error) => {
            log::warn!("Failed to load version cache from disk: {error}");
        }
    }
}

pub(super) async fn initialize(
    providers: Vec<Arc<dyn BackendProvider>>,
    preferred: Option<BackendKind>,
    wsl_list_timeout_secs: u64,
    wsl_distro_timeout_secs: u64,
) -> InitResult {
    info!(
        "Initializing application with {} providers...",
        providers.len()
    );

    let detections = detect_backends(&providers).await;
    let preferred_name = preferred.unwrap_or(BackendKind::DEFAULT);
    let detected_backends = collect_detected_backends(&detections);
    let Some((backend_name, detection)) = choose_backend_detection(&detections, preferred_name)
    else {
        info!("No backend found on system");
        return no_backend_init_result(preferred_name, detected_backends);
    };

    let native_env = native_environment(*backend_name, detection.version.clone());

    #[cfg(not(windows))]
    let _ = (wsl_list_timeout_secs, wsl_distro_timeout_secs);
    #[cfg(not(windows))]
    let environments = vec![native_env];

    #[cfg(windows)]
    let environments = build_windows_environments(
        native_env,
        &providers,
        *backend_name,
        preferred_name,
        wsl_list_timeout_secs,
        wsl_distro_timeout_secs,
    )
    .await;

    log_detected_environments(&environments);

    InitResult {
        backend_found: detection.found,
        backend_path: detection.path.clone(),
        backend_in_path: detection.in_path,
        backend_dir: detection.data_dir.clone(),
        backend_version: detection.version.clone(),
        environments,
        detected_backends,
    }
}

async fn detect_backends(
    providers: &[Arc<dyn BackendProvider>],
) -> Vec<(BackendKind, BackendDetection)> {
    let futures: Vec<_> = providers
        .iter()
        .filter_map(|provider| {
            let kind = BackendKind::from_name(provider.name())?;
            let provider = Arc::clone(provider);
            Some(async move {
                debug!("Detecting {} installation...", provider.name());
                let detection = provider.detect().await;
                info!(
                    "{} detection: found={}, path={:?}, version={:?}",
                    provider.name(),
                    detection.found,
                    detection.path,
                    detection.version
                );
                (kind, detection)
            })
        })
        .collect();

    iced::futures::future::join_all(futures).await
}

fn collect_detected_backends(detections: &[(BackendKind, BackendDetection)]) -> Vec<BackendKind> {
    detections
        .iter()
        .filter_map(|(kind, detection)| detection.found.then_some(*kind))
        .collect()
}

fn choose_backend_detection(
    detections: &[(BackendKind, BackendDetection)],
    preferred_name: BackendKind,
) -> Option<&(BackendKind, BackendDetection)> {
    detections
        .iter()
        .find(|(name, detection)| detection.found && *name == preferred_name)
        .or_else(|| detections.iter().find(|(_, detection)| detection.found))
}

fn no_backend_init_result(
    preferred_name: BackendKind,
    detected_backends: Vec<BackendKind>,
) -> InitResult {
    InitResult {
        backend_found: false,
        backend_path: None,
        backend_in_path: false,
        backend_dir: None,
        backend_version: None,
        environments: vec![EnvironmentInfo {
            id: EnvironmentId::Native,
            backend_name: preferred_name,
            backend_version: None,
            available: false,
            unavailable_reason: Some("No backend installed".to_string()),
        }],
        detected_backends,
    }
}

fn native_environment(
    backend_name: BackendKind,
    backend_version: Option<String>,
) -> EnvironmentInfo {
    EnvironmentInfo {
        id: EnvironmentId::Native,
        backend_name,
        backend_version,
        available: true,
        unavailable_reason: None,
    }
}

fn log_detected_environments(environments: &[EnvironmentInfo]) {
    info!(
        "Initialization complete with {} environments",
        environments.len()
    );
    for (idx, environment) in environments.iter().enumerate() {
        trace!("  Environment {idx}: {environment:?}");
    }
}

#[cfg(windows)]
async fn build_windows_environments(
    native_env: EnvironmentInfo,
    providers: &[Arc<dyn BackendProvider>],
    native_backend_name: BackendKind,
    preferred_name: BackendKind,
    wsl_list_timeout_secs: u64,
    wsl_distro_timeout_secs: u64,
) -> Vec<EnvironmentInfo> {
    use versi_platform::{WslTimeouts, detect_wsl_distros};

    info!("Running on Windows, detecting WSL distros...");

    let wsl_timeouts = WslTimeouts {
        list: std::time::Duration::from_secs(wsl_list_timeout_secs),
        distro: std::time::Duration::from_secs(wsl_distro_timeout_secs),
    };

    let mut environments = vec![native_env];
    let search_paths = collect_wsl_search_paths(providers);
    let distros = detect_wsl_distros(&search_paths, &wsl_timeouts).await;

    debug!(
        "WSL distros found: {:?}",
        distros.iter().map(|d| &d.name).collect::<Vec<_>>()
    );

    for distro in distros {
        environments.push(
            build_wsl_environment(
                distro,
                native_backend_name,
                preferred_name,
                wsl_distro_timeout_secs,
            )
            .await,
        );
    }

    environments
}

#[cfg(windows)]
fn collect_wsl_search_paths(providers: &[Arc<dyn BackendProvider>]) -> Vec<&'static str> {
    let mut paths = Vec::new();
    for provider in providers {
        paths.extend(provider.wsl_search_paths());
    }
    paths.sort_unstable();
    paths.dedup();
    paths
}

#[cfg(windows)]
async fn build_wsl_environment(
    distro: versi_platform::WslDistro,
    native_backend_name: BackendKind,
    preferred_name: BackendKind,
    wsl_distro_timeout_secs: u64,
) -> EnvironmentInfo {
    if !distro.is_running {
        info!(
            "Adding unavailable WSL environment: {} (not running)",
            distro.name
        );
        return unavailable_wsl_environment(distro.name, native_backend_name, "Not running");
    }

    if let Some(backend_path) = distro.backend_path {
        let backend_name = determine_wsl_backend(&backend_path, preferred_name);
        info!(
            "Adding WSL environment: {} ({} at {})",
            distro.name, backend_name, backend_path
        );
        let backend_version =
            get_wsl_backend_version(&distro.name, &backend_path, wsl_distro_timeout_secs).await;
        return EnvironmentInfo {
            id: EnvironmentId::Wsl {
                distro: distro.name,
                backend_path,
            },
            backend_name,
            backend_version,
            available: true,
            unavailable_reason: None,
        };
    }

    info!(
        "Adding unavailable WSL environment: {} (no backend found)",
        distro.name
    );
    unavailable_wsl_environment(distro.name, native_backend_name, "No backend installed")
}

#[cfg(windows)]
fn unavailable_wsl_environment(
    distro: String,
    backend_name: BackendKind,
    reason: &str,
) -> EnvironmentInfo {
    EnvironmentInfo {
        id: EnvironmentId::Wsl {
            distro,
            backend_path: String::new(),
        },
        backend_name,
        backend_version: None,
        available: false,
        unavailable_reason: Some(reason.to_string()),
    }
}

#[cfg(any(windows, test))]
fn determine_wsl_backend(path: &str, default_name: BackendKind) -> BackendKind {
    let normalized = path.to_ascii_lowercase();

    if normalized.contains("volta") {
        BackendKind::Volta
    } else if normalized.contains("nvm") {
        BackendKind::Nvm
    } else if normalized.contains("asdf") {
        BackendKind::Asdf
    } else if normalized.contains("fnm") {
        BackendKind::Fnm
    } else {
        default_name
    }
}

#[cfg(any(windows, test))]
fn normalize_backend_version_output(version_str: &str) -> String {
    let trimmed = version_str.trim();
    trimmed
        .strip_prefix("fnm ")
        .or_else(|| trimmed.strip_prefix("nvm "))
        .or_else(|| trimmed.strip_prefix("volta "))
        .or_else(|| trimmed.strip_prefix("asdf "))
        .unwrap_or(trimmed)
        .trim_start_matches('v')
        .to_string()
}

#[cfg(windows)]
async fn get_wsl_backend_version(
    distro: &str,
    backend_path: &str,
    timeout_secs: u64,
) -> Option<String> {
    use std::time::Duration;

    use tokio::process::Command;
    use versi_core::HideWindow;

    let timeout = Duration::from_secs(timeout_secs);

    let output = tokio::time::timeout(
        timeout,
        Command::new("wsl.exe")
            .args(["-d", distro, "--", backend_path, "--version"])
            .hide_window()
            .output(),
    )
    .await;

    match output {
        Ok(Ok(output)) if output.status.success() => {
            let version_str = String::from_utf8_lossy(&output.stdout);
            let version = normalize_backend_version_output(&version_str);
            debug!("WSL {} backend version: {}", distro, version);
            Some(version)
        }
        Err(_) => {
            log::warn!(
                "WSL backend version check for {} timed out after {:?}",
                distro,
                timeout,
            );
            None
        }
        _ => None,
    }
}

pub(super) fn create_backend_for_environment(
    env_id: &EnvironmentId,
    detected_path: &Path,
    detected_in_path: bool,
    detected_dir: Option<&Path>,
    provider: &Arc<dyn BackendProvider>,
) -> Arc<dyn VersionManager> {
    match env_id {
        EnvironmentId::Native => {
            let detection = BackendDetection {
                found: true,
                path: Some(detected_path.to_path_buf()),
                version: None,
                in_path: detected_in_path,
                data_dir: detected_dir.map(Path::to_path_buf),
            };
            provider.create_manager(&detection)
        }
        EnvironmentId::Wsl {
            distro,
            backend_path,
        } => provider.create_manager_for_wsl(distro.clone(), backend_path.clone()),
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::sync::Arc;

    use versi_backend::{BackendDetection, BackendProvider};
    use versi_platform::EnvironmentId;

    use super::super::test_app_with_two_environments;
    use super::{
        build_environment_states, choose_backend_detection, collect_detected_backends,
        create_backend_for_environment, determine_wsl_backend, native_environment,
        no_backend_init_result, normalize_backend_version_output,
    };
    use crate::backend_kind::BackendKind;
    use crate::message::EnvironmentInfo;

    fn detection(found: bool, path: Option<&str>) -> BackendDetection {
        BackendDetection {
            found,
            path: path.map(PathBuf::from),
            version: Some("1.0.0".to_string()),
            in_path: true,
            data_dir: None,
        }
    }

    #[test]
    fn collect_detected_backends_returns_only_found_entries() {
        let detections = vec![
            (BackendKind::Fnm, detection(true, Some("/usr/bin/fnm"))),
            (BackendKind::Nvm, detection(false, None)),
        ];

        let detected = collect_detected_backends(&detections);

        assert_eq!(detected, vec![BackendKind::Fnm]);
    }

    #[test]
    fn choose_backend_detection_prefers_requested_backend() {
        let detections = vec![
            (BackendKind::Fnm, detection(true, Some("/usr/bin/fnm"))),
            (BackendKind::Nvm, detection(true, Some("/usr/bin/nvm"))),
        ];

        let chosen =
            choose_backend_detection(&detections, BackendKind::Nvm).expect("expected backend");

        assert_eq!(chosen.0, BackendKind::Nvm);
    }

    #[test]
    fn choose_backend_detection_falls_back_to_first_found_backend() {
        let detections = vec![
            (BackendKind::Fnm, detection(true, Some("/usr/bin/fnm"))),
            (BackendKind::Nvm, detection(false, None)),
        ];

        let chosen =
            choose_backend_detection(&detections, BackendKind::Nvm).expect("expected backend");

        assert_eq!(chosen.0, BackendKind::Fnm);
    }

    #[cfg(windows)]
    #[test]
    fn determine_wsl_backend_recognizes_volta_paths() {
        assert_eq!(
            super::determine_wsl_backend("/home/user/.volta/bin/volta", BackendKind::Fnm),
            BackendKind::Volta
        );
    }

    #[test]
    fn no_backend_init_result_marks_native_environment_unavailable() {
        let result = no_backend_init_result(BackendKind::Nvm, vec![]);

        assert!(!result.backend_found);
        assert_eq!(result.environments.len(), 1);
        assert_eq!(result.environments[0].id, EnvironmentId::Native);
        assert!(!result.environments[0].available);
        assert_eq!(
            result.environments[0].unavailable_reason.as_deref(),
            Some("No backend installed")
        );
    }

    #[test]
    fn build_environment_states_preserves_available_and_unavailable_entries() {
        let init = crate::message::InitResult {
            backend_found: true,
            backend_path: Some(PathBuf::from("fnm")),
            backend_in_path: false,
            backend_dir: None,
            backend_version: Some("1.38.0".to_string()),
            environments: vec![
                EnvironmentInfo {
                    id: EnvironmentId::Native,
                    backend_name: BackendKind::Fnm,
                    backend_version: Some("1.38.0".to_string()),
                    available: true,
                    unavailable_reason: None,
                },
                EnvironmentInfo {
                    id: EnvironmentId::Wsl {
                        distro: "Ubuntu".to_string(),
                        backend_path: "/home/user/.nvm/nvm.sh".to_string(),
                    },
                    backend_name: BackendKind::Nvm,
                    backend_version: None,
                    available: false,
                    unavailable_reason: Some("Not running".to_string()),
                },
            ],
            detected_backends: vec![BackendKind::Fnm],
        };

        let states = build_environment_states(&init);

        assert_eq!(states.len(), 2);
        assert!(states[0].available);
        assert!(states[0].error.is_none());
        assert!(!states[1].available);
        assert!(states[1].error.is_some());
    }

    #[test]
    fn create_backend_for_environment_native_uses_detected_path() {
        let provider: Arc<dyn BackendProvider> = Arc::new(versi_fnm::FnmProvider::new());

        let manager = create_backend_for_environment(
            &EnvironmentId::Native,
            PathBuf::from("/custom/fnm").as_path(),
            false,
            Some(Path::new("/custom/fnm-dir")),
            &provider,
        );

        assert_eq!(manager.backend_info().path, PathBuf::from("/custom/fnm"));
        assert_eq!(
            manager.backend_info().data_dir,
            Some(PathBuf::from("/custom/fnm-dir"))
        );
    }

    #[test]
    fn create_backend_for_environment_wsl_uses_wsl_backend_path() {
        let provider: Arc<dyn BackendProvider> = Arc::new(versi_nvm::NvmProvider::new());
        let env = EnvironmentId::Wsl {
            distro: "Ubuntu".to_string(),
            backend_path: "/home/user/.nvm/nvm.sh".to_string(),
        };

        let manager = create_backend_for_environment(
            &env,
            PathBuf::from("ignored").as_path(),
            false,
            None,
            &provider,
        );

        assert_eq!(
            manager.backend_info().path,
            PathBuf::from("/home/user/.nvm/nvm.sh")
        );
    }

    #[test]
    fn native_environment_marks_environment_available() {
        let env = native_environment(BackendKind::Fnm, Some("1.38.0".to_string()));

        assert_eq!(env.id, EnvironmentId::Native);
        assert_eq!(env.backend_name, BackendKind::Fnm);
        assert!(env.available);
        assert!(env.unavailable_reason.is_none());
    }

    #[test]
    fn initialized_marks_only_active_environment_as_loading() {
        let mut app = test_app_with_two_environments();

        let init_result = crate::message::InitResult {
            backend_found: true,
            backend_path: Some(PathBuf::from("fnm")),
            backend_in_path: false,
            backend_dir: None,
            backend_version: Some("1.0.0".to_string()),
            environments: vec![
                EnvironmentInfo {
                    id: EnvironmentId::Native,
                    backend_name: BackendKind::Fnm,
                    backend_version: Some("1.0.0".to_string()),
                    available: true,
                    unavailable_reason: None,
                },
                EnvironmentInfo {
                    id: EnvironmentId::Wsl {
                        distro: "Ubuntu".to_string(),
                        backend_path: "/home/user/.nvm/nvm.sh".to_string(),
                    },
                    backend_name: BackendKind::Nvm,
                    backend_version: Some("0.39.0".to_string()),
                    available: true,
                    unavailable_reason: None,
                },
            ],
            detected_backends: vec![BackendKind::Fnm, BackendKind::Nvm],
        };

        let _ = app.handle_initialized(init_result);

        let state = app.main_state();
        assert!(state.environments[0].loading);
        assert!(!state.environments[1].loading);
    }

    #[test]
    fn determine_wsl_backend_recognizes_known_backend_paths() {
        assert_eq!(
            determine_wsl_backend("/home/user/.asdf/bin/asdf", BackendKind::Fnm),
            BackendKind::Asdf
        );
        assert_eq!(
            determine_wsl_backend("/home/user/.nvm/nvm.sh", BackendKind::Fnm),
            BackendKind::Nvm
        );
        assert_eq!(
            determine_wsl_backend("/home/user/.local/share/fnm/fnm", BackendKind::Nvm),
            BackendKind::Fnm
        );
    }

    #[test]
    fn determine_wsl_backend_uses_default_for_unknown_paths() {
        assert_eq!(
            determine_wsl_backend("/opt/custom/node-manager", BackendKind::Asdf),
            BackendKind::Asdf
        );
    }

    #[test]
    fn normalize_backend_version_output_strips_each_backend_prefix() {
        assert_eq!(normalize_backend_version_output("fnm 1.38.0"), "1.38.0");
        assert_eq!(normalize_backend_version_output("nvm 0.40.0"), "0.40.0");
        assert_eq!(normalize_backend_version_output("volta 2.0.0"), "2.0.0");
        assert_eq!(normalize_backend_version_output("asdf 0.18.0"), "0.18.0");
    }

    #[test]
    fn normalize_backend_version_output_strips_v_prefix() {
        assert_eq!(normalize_backend_version_output("v20.0.0"), "20.0.0");
    }

    #[test]
    fn normalize_backend_version_output_bare_version_unchanged() {
        assert_eq!(normalize_backend_version_output("1.38.0"), "1.38.0");
    }

    #[test]
    fn normalize_backend_version_output_trims_whitespace() {
        assert_eq!(normalize_backend_version_output(" 1.38.0 "), "1.38.0");
    }
}
