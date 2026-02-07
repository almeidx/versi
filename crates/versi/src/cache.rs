use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use versi_backend::RemoteVersion;
use versi_core::ReleaseSchedule;
use versi_platform::AppPaths;

#[derive(Serialize, Deserialize)]
pub struct DiskCache {
    pub remote_versions: Vec<RemoteVersion>,
    pub release_schedule: Option<ReleaseSchedule>,
    pub cached_at: DateTime<Utc>,
}

impl DiskCache {
    pub fn load() -> Option<Self> {
        let paths = AppPaths::new().ok()?;
        let path = paths.version_cache_file();
        let data = std::fs::read_to_string(path).ok()?;
        serde_json::from_str(&data).ok()
    }

    pub fn save(&self) {
        let Ok(paths) = AppPaths::new() else {
            return;
        };
        let _ = paths.ensure_dirs();
        let path = paths.version_cache_file();
        if let Ok(data) = serde_json::to_string(self) {
            let _ = std::fs::write(path, data);
        }
    }
}
