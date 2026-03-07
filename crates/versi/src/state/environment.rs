use std::collections::HashSet;

use tokio_util::sync::CancellationToken;
use versi_backend::{InstalledVersion, NodeVersion, VersionGroup};
use versi_platform::EnvironmentId;

use crate::backend_kind::BackendKind;
use crate::error::AppError;

#[derive(Debug)]
pub struct EnvironmentState {
    pub id: EnvironmentId,
    pub name: String,
    pub installed_versions: Vec<InstalledVersion>,
    pub installed_set: HashSet<NodeVersion>,
    pub version_groups: Vec<VersionGroup>,
    pub default_version: Option<NodeVersion>,
    pub backend_name: BackendKind,
    pub backend_version: Option<String>,
    pub loading: bool,
    pub error: Option<AppError>,
    pub load_request_seq: u64,
    pub load_cancel_token: Option<CancellationToken>,
    pub available: bool,
    pub loaded: bool,
}

impl EnvironmentState {
    pub fn new(
        id: EnvironmentId,
        backend_name: BackendKind,
        backend_version: Option<String>,
    ) -> Self {
        let name = id.display_name();
        Self {
            id,
            name,
            installed_versions: Vec::new(),
            installed_set: HashSet::new(),
            version_groups: Vec::new(),
            default_version: None,
            backend_name,
            backend_version,
            loading: true,
            error: None,
            load_request_seq: 0,
            load_cancel_token: None,
            available: true,
            loaded: false,
        }
    }

    pub fn unavailable(id: EnvironmentId, backend_name: BackendKind, reason: &str) -> Self {
        let name = id.display_name();
        Self {
            id,
            name,
            installed_versions: Vec::new(),
            installed_set: HashSet::new(),
            version_groups: Vec::new(),
            default_version: None,
            backend_name,
            backend_version: None,
            loading: false,
            error: Some(AppError::environment_unavailable(reason)),
            load_request_seq: 0,
            load_cancel_token: None,
            available: false,
            loaded: false,
        }
    }

    pub fn prepare_load(&mut self) -> (EnvironmentId, u64, CancellationToken) {
        if let Some(token) = self.load_cancel_token.take() {
            token.cancel();
        }
        self.loading = true;
        self.error = None;
        self.load_request_seq = self.load_request_seq.wrapping_add(1);
        let cancel_token = CancellationToken::new();
        self.load_cancel_token = Some(cancel_token.clone());
        (self.id.clone(), self.load_request_seq, cancel_token)
    }

    pub fn update_versions(&mut self, versions: Vec<InstalledVersion>) {
        self.default_version = versions.iter().find(|v| v.is_default).map(|v| v.version);
        self.installed_set = versions.iter().map(|v| v.version).collect();
        self.version_groups = VersionGroup::from_versions(&versions);
        self.installed_versions = versions;
        self.loading = false;
        self.loaded = true;
        self.error = None;
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};
    use versi_backend::NodeVersion;
    use versi_platform::EnvironmentId;

    use super::EnvironmentState;
    use crate::backend_kind::BackendKind;

    fn installed(version: &str, is_default: bool) -> versi_backend::InstalledVersion {
        versi_backend::InstalledVersion {
            version: version.parse().expect("test version should parse"),
            is_default,
            lts_codename: Some("LTS".to_string()),
            disk_size: Some(1024),
        }
    }

    #[test]
    fn new_environment_state_starts_loading_and_available() {
        let state = EnvironmentState::new(
            EnvironmentId::Native,
            BackendKind::Fnm,
            Some("1.38.0".to_string()),
        );

        assert_eq!(state.id, EnvironmentId::Native);
        assert_eq!(state.backend_name, BackendKind::Fnm);
        assert_eq!(state.backend_version.as_deref(), Some("1.38.0"));
        assert!(state.loading);
        assert!(state.available);
        assert!(state.error.is_none());
        assert!(state.installed_versions.is_empty());
    }

    #[test]
    fn unavailable_state_sets_error_and_availability_flags() {
        let state = EnvironmentState::unavailable(
            EnvironmentId::Native,
            BackendKind::Nvm,
            "backend unavailable",
        );

        assert!(!state.loading);
        assert!(!state.available);
        assert_eq!(state.backend_name, BackendKind::Nvm);
        assert!(matches!(
            state.error,
            Some(crate::error::AppError::EnvironmentUnavailable { ref reason })
                if reason == &crate::error::AppErrorDetail::from("backend unavailable")
        ));
    }

    #[test]
    fn update_versions_refreshes_collections_and_default() {
        let mut state = EnvironmentState::new(EnvironmentId::Native, BackendKind::Fnm, None);
        state.loading = true;
        state.error = Some(crate::error::AppError::environment_load_failed("old error"));

        state.update_versions(vec![
            installed("v20.11.0", true),
            installed("v18.19.1", false),
        ]);

        assert_eq!(state.installed_versions.len(), 2);
        assert_eq!(
            state.default_version,
            Some(NodeVersion {
                major: 20,
                minor: 11,
                patch: 0,
            })
        );
        assert!(state.installed_set.contains(&NodeVersion::new(20, 11, 0)));
        assert!(state.installed_set.contains(&NodeVersion::new(18, 19, 1)));
        assert_eq!(state.version_groups.len(), 2);
        assert!(!state.loading);
        assert!(state.error.is_none());
    }

    #[test]
    #[ignore = "performance baseline; run manually"]
    fn perf_update_versions_large_input() {
        let mut state = EnvironmentState::new(EnvironmentId::Native, BackendKind::Fnm, None);
        let mut versions = Vec::new();
        for major in 16_u32..=28 {
            for minor in 0_u32..60 {
                versions.push(versi_backend::InstalledVersion {
                    version: NodeVersion::new(major, minor, 0),
                    is_default: major == 22 && minor == 59,
                    lts_codename: None,
                    disk_size: Some(1024),
                });
            }
        }

        let started = Instant::now();
        state.update_versions(versions);
        let elapsed = started.elapsed();

        assert!(
            elapsed < Duration::from_secs(2),
            "update_versions baseline exceeded: {elapsed:?}"
        );
        assert!(!state.version_groups.is_empty());
    }
}
