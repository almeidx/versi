use std::path::Path;

pub(crate) fn replace_file(src: &Path, dst: &Path) -> std::io::Result<()> {
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

pub(crate) fn quarantine_invalid_file(path: &Path, fallback_name: &str) {
    use std::time::{SystemTime, UNIX_EPOCH};

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(fallback_name);
    let mut last_error = None;

    for attempt in 0..5 {
        let suffix = if attempt == 0 {
            format!("{file_name}.corrupt-{timestamp}")
        } else {
            format!("{file_name}.corrupt-{timestamp}-{attempt}")
        };
        let backup_path = path.with_file_name(suffix);

        match std::fs::rename(path, &backup_path) {
            Ok(()) => {
                log::warn!(
                    "Quarantined invalid file {} to {}",
                    path.display(),
                    backup_path.display()
                );
                return;
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return,
            Err(error) => {
                last_error = Some(error);
            }
        }
    }

    if let Some(error) = last_error {
        log::warn!(
            "Failed to quarantine invalid file {}: {error}",
            path.display()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replace_file_moves_content_atomically() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("src.txt");
        let dst = dir.path().join("dst.txt");
        std::fs::write(&src, b"new content").unwrap();
        std::fs::write(&dst, b"old content").unwrap();

        replace_file(&src, &dst).unwrap();

        assert!(!src.exists());
        assert_eq!(std::fs::read_to_string(&dst).unwrap(), "new content");
    }

    #[test]
    fn replace_file_creates_destination_when_missing() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("src.txt");
        let dst = dir.path().join("dst.txt");
        std::fs::write(&src, b"content").unwrap();

        replace_file(&src, &dst).unwrap();

        assert!(!src.exists());
        assert_eq!(std::fs::read_to_string(&dst).unwrap(), "content");
    }

    #[test]
    fn quarantine_renames_file_with_corrupt_suffix() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("data.json");
        std::fs::write(&file, b"bad data").unwrap();

        quarantine_invalid_file(&file, "data.json");

        assert!(!file.exists());
        let entries: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .collect();
        assert_eq!(entries.len(), 1);
        let name = entries[0].file_name().to_string_lossy().to_string();
        assert!(name.starts_with("data.json.corrupt-"), "got: {name}");
    }

    #[test]
    fn quarantine_is_noop_when_file_does_not_exist() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("missing.json");

        quarantine_invalid_file(&missing, "missing.json");

        let entries: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .collect();
        assert!(entries.is_empty());
    }
}
