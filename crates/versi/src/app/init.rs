use log::{debug, info, trace};
#[cfg(windows)]
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use iced::Task;

use versi_backend::{BackendDetection, BackendProvider, VersionManager};
use versi_platform::EnvironmentId;
use versi_shell::detect_shells;

use crate::message::{EnvironmentInfo, InitResult, Message};
use crate::state::{
    AppState, BackendOption, EnvironmentState, MainState, OnboardingState, ShellConfigStatus,
};

use super::Versi;

impl Versi {
    pub(super) fn handle_initialized(&mut self, result: InitResult) -> Task<Message> {
        info!(
            "Handling initialization result: backend_found={}, environments={}",
            result.backend_found,
            result.environments.len()
        );

        if !result.backend_found {
            info!("No backend found, entering onboarding flow");
            let shells = detect_shells();
            debug!("Detected {} shells for configuration", shells.len());

            let shell_statuses: Vec<ShellConfigStatus> = shells
                .into_iter()
                .map(|s| ShellConfigStatus {
                    shell_type: s.shell_type.clone(),
                    shell_name: s.shell_type.name().to_string(),
                    configured: s.is_configured,
                    config_path: s.config_file,
                    configuring: false,
                    error: None,
                })
                .collect();

            let mut onboarding = OnboardingState::new();
            onboarding.detected_shells = shell_statuses;

            onboarding.available_backends = self
                .providers
                .values()
                .map(|p| BackendOption {
                    name: p.name(),
                    display_name: p.display_name(),
                    detected: false,
                })
                .collect();

            self.state = AppState::Onboarding(onboarding);
            return Task::none();
        }

        let native_env = result.environments.first();
        let active_backend_name = native_env.map(|e| e.backend_name).unwrap_or("fnm");

        if let Some(provider) = self.providers.get(active_backend_name) {
            self.provider = provider.clone();
        }

        let backend_path = result
            .backend_path
            .unwrap_or_else(|| PathBuf::from(self.provider.name()));
        let backend_dir = result.backend_dir;

        self.backend_path = backend_path.clone();
        self.backend_dir = backend_dir.clone();

        let detection = BackendDetection {
            found: true,
            path: Some(backend_path.clone()),
            version: result.backend_version.clone(),
            in_path: true,
            data_dir: backend_dir.clone(),
        };
        let backend = self.provider.create_manager(&detection);

        let environments: Vec<EnvironmentState> = result
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
            .collect();

        let mut main_state =
            MainState::new_with_environments(backend, environments, active_backend_name);
        main_state.detected_backends = result.detected_backends;

        if let Some(disk_cache) = crate::cache::DiskCache::load() {
            debug!(
                "Loaded disk cache from {:?} ({} versions, schedule={})",
                disk_cache.cached_at,
                disk_cache.remote_versions.len(),
                disk_cache.release_schedule.is_some()
            );
            if !disk_cache.remote_versions.is_empty() {
                main_state.available_versions.versions = disk_cache.remote_versions;
                main_state.available_versions.loaded_from_disk = true;
            }
            if let Some(schedule) = disk_cache.release_schedule {
                main_state.available_versions.schedule = Some(schedule);
            }
        }

        self.state = AppState::Main(Box::new(main_state));

        let mut load_tasks: Vec<Task<Message>> = Vec::new();

        for env_info in &result.environments {
            if !env_info.available {
                debug!(
                    "Skipping load for unavailable environment: {:?}",
                    env_info.id
                );
                continue;
            }

            let env_id = env_info.id.clone();
            let env_backend_name = env_info.backend_name;

            let provider = self
                .providers
                .get(env_backend_name)
                .cloned()
                .unwrap_or_else(|| self.provider.clone());

            let backend =
                create_backend_for_environment(&env_id, &backend_path, &backend_dir, &provider);

            load_tasks.push(Task::perform(
                async move {
                    let versions = tokio::time::timeout(
                        std::time::Duration::from_secs(30),
                        backend.list_installed(),
                    )
                    .await
                    .unwrap_or(Ok(Vec::new()))
                    .unwrap_or_default();
                    (env_id, versions)
                },
                move |(env_id, versions)| Message::EnvironmentLoaded { env_id, versions },
            ));
        }

        let fetch_remote = self.handle_fetch_remote_versions();
        let fetch_schedule = self.handle_fetch_release_schedule();
        let check_app_update = self.handle_check_for_app_update();
        let check_backend_update = self.handle_check_for_backend_update();

        load_tasks.extend([
            fetch_remote,
            fetch_schedule,
            check_app_update,
            check_backend_update,
        ]);

        Task::batch(load_tasks)
    }
}

pub(super) async fn initialize(
    providers: Vec<Arc<dyn BackendProvider>>,
    preferred: Option<String>,
) -> InitResult {
    info!(
        "Initializing application with {} providers...",
        providers.len()
    );

    let mut detections: Vec<(&'static str, BackendDetection)> = Vec::new();
    for provider in &providers {
        debug!("Detecting {} installation...", provider.name());
        let detection = provider.detect().await;
        info!(
            "{} detection: found={}, path={:?}, version={:?}",
            provider.name(),
            detection.found,
            detection.path,
            detection.version
        );
        detections.push((provider.name(), detection));
    }

    let preferred_name: &'static str = match preferred.as_deref() {
        Some("nvm") => "nvm",
        _ => "fnm",
    };

    let detected_backends: Vec<&'static str> = detections
        .iter()
        .filter(|(_, det)| det.found)
        .map(|(name, _)| *name)
        .collect();

    let chosen = detections
        .iter()
        .find(|(name, det)| det.found && *name == preferred_name)
        .or_else(|| detections.iter().find(|(_, det)| det.found));

    let (backend_name, detection) = match chosen {
        Some((name, det)) => (*name, det.clone()),
        None => {
            info!("No backend found on system");
            return InitResult {
                backend_found: false,
                backend_path: None,
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
            };
        }
    };

    let native_env = EnvironmentInfo {
        id: EnvironmentId::Native,
        backend_name,
        backend_version: detection.version.clone(),
        available: true,
        unavailable_reason: None,
    };

    #[cfg(not(windows))]
    let environments = vec![native_env];

    #[cfg(windows)]
    let environments = {
        let mut envs = vec![native_env];

        use versi_platform::detect_wsl_distros;
        info!("Running on Windows, detecting WSL distros...");

        let mut all_search_paths: Vec<&str> = Vec::new();
        for provider in &providers {
            all_search_paths.extend(provider.wsl_search_paths());
        }
        all_search_paths.sort();
        all_search_paths.dedup();

        let distros = detect_wsl_distros(&all_search_paths);
        debug!(
            "WSL distros found: {:?}",
            distros.iter().map(|d| &d.name).collect::<Vec<_>>()
        );

        let provider_map: HashMap<&str, &Arc<dyn BackendProvider>> =
            providers.iter().map(|p| (p.name(), p)).collect();

        for distro in distros {
            if !distro.is_running {
                info!(
                    "Adding unavailable WSL environment: {} (not running)",
                    distro.name
                );
                envs.push(EnvironmentInfo {
                    id: EnvironmentId::Wsl {
                        distro: distro.name,
                        backend_path: String::new(),
                    },
                    backend_name,
                    backend_version: None,
                    available: false,
                    unavailable_reason: Some("Not running".to_string()),
                });
            } else if let Some(bp) = distro.backend_path {
                let wsl_backend_name = determine_wsl_backend(&bp, &provider_map, preferred_name);
                info!(
                    "Adding WSL environment: {} ({} at {})",
                    distro.name, wsl_backend_name, bp
                );
                let backend_version = get_wsl_backend_version(&distro.name, &bp).await;
                envs.push(EnvironmentInfo {
                    id: EnvironmentId::Wsl {
                        distro: distro.name,
                        backend_path: bp,
                    },
                    backend_name: wsl_backend_name,
                    backend_version,
                    available: true,
                    unavailable_reason: None,
                });
            } else {
                info!(
                    "Adding unavailable WSL environment: {} (no backend found)",
                    distro.name
                );
                envs.push(EnvironmentInfo {
                    id: EnvironmentId::Wsl {
                        distro: distro.name,
                        backend_path: String::new(),
                    },
                    backend_name,
                    backend_version: None,
                    available: false,
                    unavailable_reason: Some("No backend installed".to_string()),
                });
            }
        }

        envs
    };

    info!(
        "Initialization complete with {} environments",
        environments.len()
    );
    for (i, env) in environments.iter().enumerate() {
        trace!("  Environment {}: {:?}", i, env);
    }

    InitResult {
        backend_found: detection.found,
        backend_path: detection.path,
        backend_dir: detection.data_dir,
        backend_version: detection.version,
        environments,
        detected_backends,
    }
}

#[cfg(windows)]
fn determine_wsl_backend<'a>(
    path: &str,
    _providers: &HashMap<&str, &Arc<dyn BackendProvider>>,
    default_name: &'a str,
) -> &'static str {
    if path.contains("nvm") {
        "nvm"
    } else if path.contains("fnm") {
        "fnm"
    } else {
        // Leak is safe here: only "fnm" or "nvm" literals in practice
        let leaked: &'static str = default_name.to_string().leak();
        leaked
    }
}

#[cfg(windows)]
async fn get_wsl_backend_version(distro: &str, backend_path: &str) -> Option<String> {
    use tokio::process::Command;
    use versi_core::HideWindow;

    let output = Command::new("wsl.exe")
        .args(["-d", distro, "--", backend_path, "--version"])
        .hide_window()
        .output()
        .await
        .ok()?;

    if output.status.success() {
        let version_str = String::from_utf8_lossy(&output.stdout);
        let version = version_str
            .trim()
            .strip_prefix("fnm ")
            .unwrap_or(version_str.trim())
            .to_string();
        debug!("WSL {} backend version: {}", distro, version);
        Some(version)
    } else {
        None
    }
}

pub(super) fn create_backend_for_environment(
    env_id: &EnvironmentId,
    detected_path: &Path,
    detected_dir: &Option<PathBuf>,
    provider: &Arc<dyn BackendProvider>,
) -> Box<dyn VersionManager> {
    match env_id {
        EnvironmentId::Native => {
            let detection = BackendDetection {
                found: true,
                path: Some(detected_path.to_path_buf()),
                version: None,
                in_path: true,
                data_dir: detected_dir.clone(),
            };
            provider.create_manager(&detection)
        }
        EnvironmentId::Wsl {
            distro,
            backend_path,
        } => provider.create_manager_for_wsl(distro.clone(), backend_path.clone()),
    }
}
