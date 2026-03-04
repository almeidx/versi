use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use versi_backend::RemoteVersion;
use versi_core::{ReleaseSchedule, SecurityAdvisory, VersionMeta};
use versi_platform::AppPaths;

#[derive(Serialize, Deserialize)]
pub struct DiskCache {
    pub remote_versions: Vec<RemoteVersion>,
    pub release_schedule: Option<ReleaseSchedule>,
    #[serde(default)]
    pub version_metadata: Option<HashMap<String, VersionMeta>>,
    #[serde(default)]
    pub security_advisories: Option<HashMap<String, SecurityAdvisory>>,
    pub cached_at: DateTime<Utc>,
}

#[derive(Debug, thiserror::Error)]
pub enum DiskCacheLoadError {
    #[error("failed to resolve app paths: {0}")]
    Paths(#[source] versi_platform::AppPathsError),
    #[error("failed to read cache file at {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse cache file at {path}: {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
}

#[derive(Serialize)]
struct DiskCacheSnapshot<'a> {
    remote_versions: &'a [RemoteVersion],
    release_schedule: Option<&'a ReleaseSchedule>,
    version_metadata: Option<&'a HashMap<String, VersionMeta>>,
    security_advisories: Option<&'a HashMap<String, SecurityAdvisory>>,
    cached_at: DateTime<Utc>,
}

impl DiskCache {
    fn load_from_path(path: &Path) -> Result<Option<Self>, DiskCacheLoadError> {
        if !path.exists() {
            return Ok(None);
        }

        let data = std::fs::read_to_string(path).map_err(|source| DiskCacheLoadError::Read {
            path: path.to_path_buf(),
            source,
        })?;
        serde_json::from_str(&data)
            .map(Some)
            .map_err(|source| DiskCacheLoadError::Parse {
                path: path.to_path_buf(),
                source,
            })
    }

    pub fn load() -> Result<Option<Self>, DiskCacheLoadError> {
        let paths = AppPaths::new().map_err(DiskCacheLoadError::Paths)?;
        let path = paths.version_cache_file();

        match Self::load_from_path(&path) {
            Ok(cache) => Ok(cache),
            Err(error @ DiskCacheLoadError::Parse { .. }) => {
                quarantine_invalid_cache_file(&path);
                Err(error)
            }
            Err(error) => Err(error),
        }
    }
}

pub fn save_snapshot(
    remote_versions: &[RemoteVersion],
    release_schedule: Option<&ReleaseSchedule>,
    version_metadata: Option<&HashMap<String, VersionMeta>>,
    security_advisories: Option<&HashMap<String, SecurityAdvisory>>,
    cached_at: DateTime<Utc>,
) {
    let paths = match AppPaths::new() {
        Ok(p) => p,
        Err(e) => {
            log::warn!("Cache save: failed to resolve paths: {e}");
            return;
        }
    };
    if let Err(e) = paths.ensure_dirs() {
        log::warn!("Cache save: failed to create dirs: {e}");
        return;
    }

    save_snapshot_to_path(
        &paths.version_cache_file(),
        remote_versions,
        release_schedule,
        version_metadata,
        security_advisories,
        cached_at,
    );
}

fn save_snapshot_to_path(
    path: &Path,
    remote_versions: &[RemoteVersion],
    release_schedule: Option<&ReleaseSchedule>,
    version_metadata: Option<&HashMap<String, VersionMeta>>,
    security_advisories: Option<&HashMap<String, SecurityAdvisory>>,
    cached_at: DateTime<Utc>,
) {
    let payload = DiskCacheSnapshot {
        remote_versions,
        release_schedule,
        version_metadata,
        security_advisories,
        cached_at,
    };
    let data = match serde_json::to_vec(&payload) {
        Ok(d) => d,
        Err(e) => {
            log::warn!("Cache save: serialization failed: {e}");
            return;
        }
    };
    if let Err(e) = write_atomic(path, &data) {
        log::warn!("Cache save: failed to write {}: {e}", path.display());
    }
}

fn write_atomic(path: &Path, data: &[u8]) -> std::io::Result<()> {
    let parent = path.parent().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "cache path has no parent")
    })?;

    let file_name = path
        .file_name()
        .and_then(std::ffi::OsStr::to_str)
        .unwrap_or("cache");
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());
    let pid = std::process::id();

    let mut tmp_path = None;
    for attempt in 0..16_u8 {
        let candidate = parent.join(format!(".{file_name}.{pid}.{timestamp}.{attempt}.tmp"));
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate)
        {
            Ok(mut file) => {
                file.write_all(data)?;
                file.sync_all()?;
                tmp_path = Some(candidate);
                break;
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error),
        }
    }

    let Some(tmp_path) = tmp_path else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            "failed to create unique cache temp file",
        ));
    };

    if let Err(error) = replace_file(&tmp_path, path) {
        let _ = std::fs::remove_file(&tmp_path);
        return Err(error);
    }

    Ok(())
}

fn quarantine_invalid_cache_file(path: &Path) {
    use std::time::{SystemTime, UNIX_EPOCH};

    if !path.exists() {
        return;
    }

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("versions.json");

    for attempt in 0..5 {
        let suffix = if attempt == 0 {
            format!("{file_name}.corrupt-{timestamp}")
        } else {
            format!("{file_name}.corrupt-{timestamp}-{attempt}")
        };
        let backup_path = path.with_file_name(suffix);
        if std::fs::rename(path, &backup_path).is_ok() {
            log::warn!(
                "Quarantined invalid cache file {} to {}",
                path.display(),
                backup_path.display()
            );
            return;
        }
    }

    log::warn!(
        "Failed to quarantine invalid cache file at {}",
        path.display()
    );
}

fn replace_file(src: &Path, dst: &Path) -> std::io::Result<()> {
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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use chrono::Utc;
    use versi_backend::{NodeVersion, RemoteVersion};
    use versi_core::{SecurityAdvisory, VersionMeta};

    use super::DiskCache;

    fn sample_cache() -> DiskCache {
        DiskCache {
            remote_versions: vec![RemoteVersion {
                version: NodeVersion::new(22, 10, 0),
                lts_codename: Some("LTS".to_string()),
                is_latest: true,
            }],
            release_schedule: None,
            version_metadata: Some(HashMap::from([(
                "v22.10.0".to_string(),
                VersionMeta {
                    date: "2025-11-01".to_string(),
                    security: false,
                    npm: Some("10.9.0".to_string()),
                    v8: Some("12.0".to_string()),
                    openssl: Some("3.0.0".to_string()),
                },
            )])),
            security_advisories: Some(HashMap::from([(
                "163".to_string(),
                SecurityAdvisory {
                    cve: vec!["CVE-2026-21637".to_string()],
                    vulnerable: "20.x || 22.x".to_string(),
                    patched: "^20.20.0 || ^22.22.0".to_string(),
                    severity: "medium".to_string(),
                    reference:
                        "https://nodejs.org/en/blog/vulnerability/december-2025-security-releases"
                            .to_string(),
                    description: "TLS callback issue".to_string(),
                    overview: "overview".to_string(),
                    affected_environments: vec!["all".to_string()],
                },
            )])),
            cached_at: Utc::now(),
        }
    }

    #[test]
    fn save_to_path_and_load_from_path_round_trip() {
        let temp_dir = tempfile::tempdir().expect("temporary directory should be created");
        let path = temp_dir.path().join("versions.json");
        let cache = sample_cache();

        super::save_snapshot_to_path(
            &path,
            &cache.remote_versions,
            cache.release_schedule.as_ref(),
            cache.version_metadata.as_ref(),
            cache.security_advisories.as_ref(),
            cache.cached_at,
        );
        let loaded = DiskCache::load_from_path(&path)
            .expect("cache should parse")
            .expect("cache should load");

        assert_eq!(loaded.remote_versions.len(), 1);
        assert_eq!(
            loaded.remote_versions[0].version,
            NodeVersion::new(22, 10, 0)
        );
        let metadata = loaded
            .version_metadata
            .expect("version metadata should be preserved");
        assert_eq!(
            metadata.get("v22.10.0").and_then(|v| v.npm.as_deref()),
            Some("10.9.0")
        );
        assert!(
            loaded
                .security_advisories
                .as_ref()
                .is_some_and(|advisories| advisories.contains_key("163"))
        );
    }

    #[test]
    fn load_from_path_returns_parse_error_for_invalid_json() {
        let temp_dir = tempfile::tempdir().expect("temporary directory should be created");
        let path = temp_dir.path().join("invalid.json");
        std::fs::write(&path, "{not-valid-json").expect("invalid file should be written");

        assert!(matches!(
            DiskCache::load_from_path(&path),
            Err(super::DiskCacheLoadError::Parse { .. })
        ));
    }

    #[test]
    fn save_to_path_replaces_existing_file_atomically() {
        let temp_dir = tempfile::tempdir().expect("temporary directory should be created");
        let path = temp_dir.path().join("versions.json");
        std::fs::write(&path, "{not-valid-json").expect("invalid file should be written");

        let cache = sample_cache();
        super::save_snapshot_to_path(
            &path,
            &cache.remote_versions,
            cache.release_schedule.as_ref(),
            cache.version_metadata.as_ref(),
            cache.security_advisories.as_ref(),
            cache.cached_at,
        );

        let loaded = DiskCache::load_from_path(&path)
            .expect("cache should parse after overwrite")
            .expect("cache should load after overwrite");
        assert_eq!(loaded.remote_versions.len(), 1);

        let temp_files = std::fs::read_dir(temp_dir.path())
            .expect("read temp dir entries")
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .contains(".versions.json.")
            })
            .count();
        assert_eq!(temp_files, 0);
    }

    #[test]
    fn load_from_path_preserves_backward_compat_without_security_advisories_field() {
        let temp_dir = tempfile::tempdir().expect("temporary directory should be created");
        let path = temp_dir.path().join("versions.json");
        std::fs::write(
            &path,
            r#"{
  "remote_versions": [],
  "release_schedule": null,
  "version_metadata": null,
  "cached_at": "2026-02-20T12:00:00Z"
}"#,
        )
        .expect("legacy cache should be written");

        let loaded = DiskCache::load_from_path(&path)
            .expect("legacy cache should parse")
            .expect("legacy cache should load");
        assert!(loaded.security_advisories.is_none());
    }
}
