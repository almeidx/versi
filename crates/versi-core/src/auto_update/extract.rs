use std::io::Read;
use std::path::Path;

use log::{debug, info, warn};
use sha2::{Digest, Sha256};

use super::AutoUpdateError;

pub(super) fn verify_download_checksum(
    expected_sha256: Option<&str>,
    asset_name: &str,
    downloaded_path: &Path,
) -> Result<(), AutoUpdateError> {
    let expected_sha256 = expected_sha256.ok_or_else(|| {
        AutoUpdateError::Invalid(format!(
            "Missing SHA-256 digest for {asset_name}. Refusing to apply unverified update."
        ))
    })?;
    let actual = sha256_file(downloaded_path)?;
    let expected = expected_sha256.to_ascii_lowercase();

    if actual.eq_ignore_ascii_case(&expected) {
        info!("Update checksum verified for {asset_name}");
        Ok(())
    } else {
        Err(AutoUpdateError::Invalid(format!(
            "Checksum mismatch for {asset_name}. Refusing to apply update."
        )))
    }
}

pub(super) fn sha256_file(path: &Path) -> Result<String, AutoUpdateError> {
    let mut file = std::fs::File::open(path).map_err(|error| {
        AutoUpdateError::io_with_path("failed to open file for checksum", path, &error)
    })?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 8192];

    loop {
        let read = file.read(&mut buffer).map_err(|error| {
            AutoUpdateError::io_with_path("failed to read file for checksum", path, &error)
        })?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

pub(super) fn extract_zip(zip_path: &Path, dest: &Path) -> Result<(), AutoUpdateError> {
    let file = std::fs::File::open(zip_path).map_err(|error| {
        AutoUpdateError::io_with_path("failed to open zip file", zip_path, &error)
    })?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|error| AutoUpdateError::zip("failed to read zip archive", error))?;

    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|error| AutoUpdateError::zip("failed to read zip entry", error))?;
        let Some(name) = entry.enclosed_name() else {
            warn!("Skipping zip entry with unsafe path");
            continue;
        };
        let out_path = dest.join(name);

        if entry.is_dir() {
            std::fs::create_dir_all(&out_path).map_err(|error| {
                AutoUpdateError::io_with_path(
                    "failed to create extraction directory",
                    &out_path,
                    &error,
                )
            })?;
        } else {
            if let Some(parent) = out_path.parent() {
                std::fs::create_dir_all(parent).map_err(|error| {
                    AutoUpdateError::io_with_path(
                        "failed to create extraction parent directory",
                        parent,
                        &error,
                    )
                })?;
            }
            let mut outfile = std::fs::File::create(&out_path).map_err(|error| {
                AutoUpdateError::io_with_path("failed to create extracted file", &out_path, &error)
            })?;
            std::io::copy(&mut entry, &mut outfile).map_err(|error| {
                AutoUpdateError::io_with_path("failed to extract archive entry", &out_path, &error)
            })?;

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Some(mode) = entry.unix_mode() {
                    let _ =
                        std::fs::set_permissions(&out_path, std::fs::Permissions::from_mode(mode));
                }
            }
        }
    }

    debug!("Extraction complete to {}", dest.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io::Write as _;

    use super::*;

    #[test]
    fn extract_zip_expands_files_and_directories() {
        let temp = tempfile::tempdir().expect("tempdir should be created");
        let zip_path = temp.path().join("update.zip");
        let extract_dir = temp.path().join("extract");

        let zip_file = std::fs::File::create(&zip_path).expect("zip file should be created");
        let mut writer = zip::ZipWriter::new(zip_file);
        let options = zip::write::SimpleFileOptions::default().unix_permissions(0o644);
        writer
            .add_directory("nested/", options)
            .expect("directory entry should be written");
        writer
            .start_file("nested/versi", options)
            .expect("file entry should be started");
        writer
            .write_all(b"binary-content")
            .expect("file entry should be written");
        writer.finish().expect("zip archive should be finalized");

        extract_zip(&zip_path, &extract_dir).expect("zip should extract");

        let extracted = std::fs::read(extract_dir.join("nested/versi"))
            .expect("extracted file should exist and be readable");
        assert_eq!(extracted, b"binary-content");
    }

    #[test]
    fn extract_zip_skips_unsafe_paths() {
        let temp = tempfile::tempdir().expect("tempdir should be created");
        let zip_path = temp.path().join("unsafe.zip");
        let extract_dir = temp.path().join("extract");

        let zip_file = std::fs::File::create(&zip_path).expect("zip file should be created");
        let mut writer = zip::ZipWriter::new(zip_file);
        let options = zip::write::SimpleFileOptions::default().unix_permissions(0o644);
        writer
            .start_file("../outside.txt", options)
            .expect("unsafe file entry should be started");
        writer
            .write_all(b"should not be extracted")
            .expect("unsafe file entry should be written");
        writer.finish().expect("zip archive should be finalized");

        extract_zip(&zip_path, &extract_dir).expect("zip extraction should not fail");

        assert!(
            !temp.path().join("outside.txt").exists(),
            "unsafe path should not be extracted outside destination"
        );
        assert!(
            !extract_dir.join("../outside.txt").exists(),
            "unsafe relative extraction output should not exist"
        );
    }

    #[test]
    fn sha256_file_returns_known_digest() {
        let temp = tempfile::tempdir().expect("tempdir should be created");
        let file_path = temp.path().join("payload.bin");
        std::fs::write(&file_path, b"versi").expect("payload file should be written");

        let digest = sha256_file(&file_path).expect("checksum should be computed");
        assert_eq!(
            digest,
            "50639d63848d275a7efcd04478de62ca0df8f35dfd75be490e4fcae667ecd436"
        );
    }

    #[test]
    fn verify_checksum_succeeds_on_match() {
        let temp = tempfile::tempdir().expect("tempdir should be created");
        let path = temp.path().join("payload.bin");
        std::fs::write(&path, b"versi").expect("file should be written");

        let result = verify_download_checksum(
            Some("50639d63848d275a7efcd04478de62ca0df8f35dfd75be490e4fcae667ecd436"),
            "payload.bin",
            &path,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn verify_checksum_succeeds_case_insensitive() {
        let temp = tempfile::tempdir().expect("tempdir should be created");
        let path = temp.path().join("payload.bin");
        std::fs::write(&path, b"versi").expect("file should be written");

        let result = verify_download_checksum(
            Some("50639D63848D275A7EFCD04478DE62CA0DF8F35DFD75BE490E4FCAE667ECD436"),
            "payload.bin",
            &path,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn verify_checksum_fails_on_mismatch() {
        let temp = tempfile::tempdir().expect("tempdir should be created");
        let path = temp.path().join("payload.bin");
        std::fs::write(&path, b"versi").expect("file should be written");

        let result = verify_download_checksum(Some("0000dead"), "payload.bin", &path);
        assert!(
            matches!(result, Err(AutoUpdateError::Invalid(ref msg)) if msg.contains("Checksum mismatch"))
        );
    }

    #[test]
    fn verify_checksum_fails_when_digest_missing() {
        let temp = tempfile::tempdir().expect("tempdir should be created");
        let path = temp.path().join("payload.bin");
        std::fs::write(&path, b"versi").expect("file should be written");

        let result = verify_download_checksum(None, "payload.bin", &path);
        assert!(
            matches!(result, Err(AutoUpdateError::Invalid(ref msg)) if msg.contains("Missing SHA-256"))
        );
    }
}
