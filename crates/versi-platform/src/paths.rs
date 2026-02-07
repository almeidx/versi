use std::path::PathBuf;

pub struct AppPaths {
    pub config_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub data_dir: PathBuf,
}

impl AppPaths {
    pub fn new() -> Result<Self, String> {
        #[cfg(target_os = "macos")]
        {
            let home = dirs::home_dir().ok_or("Could not determine home directory")?;
            Ok(Self {
                config_dir: home.join("Library/Application Support/versi"),
                cache_dir: home.join("Library/Caches/versi"),
                data_dir: home.join("Library/Application Support/versi"),
            })
        }

        #[cfg(target_os = "windows")]
        {
            Ok(Self {
                config_dir: dirs::config_dir()
                    .ok_or("Could not determine config directory")?
                    .join("versi"),
                cache_dir: dirs::cache_dir()
                    .ok_or("Could not determine cache directory")?
                    .join("versi"),
                data_dir: dirs::data_dir()
                    .ok_or("Could not determine data directory")?
                    .join("versi"),
            })
        }

        #[cfg(all(unix, not(target_os = "macos")))]
        {
            Ok(Self {
                config_dir: dirs::config_dir()
                    .ok_or("Could not determine config directory")?
                    .join("versi"),
                cache_dir: dirs::cache_dir()
                    .ok_or("Could not determine cache directory")?
                    .join("versi"),
                data_dir: dirs::data_dir()
                    .ok_or("Could not determine data directory")?
                    .join("versi"),
            })
        }
    }

    pub fn settings_file(&self) -> PathBuf {
        self.config_dir.join("settings.json")
    }

    pub fn version_cache_file(&self) -> PathBuf {
        self.cache_dir.join("versions.json")
    }

    pub fn log_file(&self) -> PathBuf {
        self.data_dir.join("debug.log")
    }

    pub fn ensure_dirs(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.config_dir)?;
        std::fs::create_dir_all(&self.cache_dir)?;
        std::fs::create_dir_all(&self.data_dir)?;
        Ok(())
    }
}
