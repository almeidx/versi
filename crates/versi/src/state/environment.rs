use versi_backend::{InstalledVersion, NodeVersion, VersionGroup};
use versi_platform::EnvironmentId;

#[derive(Debug)]
pub struct EnvironmentState {
    pub id: EnvironmentId,
    pub name: String,
    pub installed_versions: Vec<InstalledVersion>,
    pub version_groups: Vec<VersionGroup>,
    pub default_version: Option<NodeVersion>,
    pub backend_name: &'static str,
    pub backend_version: Option<String>,
    pub loading: bool,
    pub error: Option<String>,
    pub available: bool,
}

impl EnvironmentState {
    pub fn new(
        id: EnvironmentId,
        backend_name: &'static str,
        backend_version: Option<String>,
    ) -> Self {
        let name = id.display_name();
        Self {
            id,
            name,
            installed_versions: Vec::new(),
            version_groups: Vec::new(),
            default_version: None,
            backend_name,
            backend_version,
            loading: true,
            error: None,
            available: true,
        }
    }

    pub fn unavailable(id: EnvironmentId, backend_name: &'static str, reason: &str) -> Self {
        let name = id.display_name();
        Self {
            id,
            name,
            installed_versions: Vec::new(),
            version_groups: Vec::new(),
            default_version: None,
            backend_name,
            backend_version: None,
            loading: false,
            error: Some(reason.to_string()),
            available: false,
        }
    }

    pub fn update_versions(&mut self, versions: Vec<InstalledVersion>) {
        self.default_version = versions
            .iter()
            .find(|v| v.is_default)
            .map(|v| v.version.clone());
        self.version_groups = VersionGroup::from_versions(versions.clone());
        self.installed_versions = versions;
        self.loading = false;
        self.error = None;
    }
}
