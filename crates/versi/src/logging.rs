#[cfg(debug_assertions)]
use simplelog::{ColorChoice, TermLogger, TerminalMode};
use simplelog::{CombinedLogger, ConfigBuilder, LevelFilter, WriteLogger};
use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::Mutex;
use versi_platform::AppPaths;

struct ResilientFileWriter {
    path: PathBuf,
    file: Mutex<Option<File>>,
}

impl ResilientFileWriter {
    fn new(path: PathBuf) -> io::Result<Self> {
        let file = OpenOptions::new().create(true).append(true).open(&path)?;
        Ok(Self {
            path,
            file: Mutex::new(Some(file)),
        })
    }

    fn ensure_file(&self) -> io::Result<()> {
        let mut guard = self.file.lock().unwrap_or_else(|e| e.into_inner());

        if !self.path.exists() {
            if let Some(parent) = self.path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.path)?;
            *guard = Some(file);
        }

        Ok(())
    }
}

impl Write for ResilientFileWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.ensure_file()?;
        let mut guard = self.file.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(ref mut file) = *guard {
            file.write(buf)
        } else {
            Err(io::Error::other("File not available"))
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        let mut guard = self.file.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(ref mut file) = *guard {
            file.flush()
        } else {
            Ok(())
        }
    }
}

pub fn init_logging(debug_enabled: bool) {
    let paths = AppPaths::new();
    let _ = paths.ensure_dirs();
    let log_path = paths.log_file();

    let config = ConfigBuilder::new()
        .set_time_format_rfc3339()
        .add_filter_allow_str("versi")
        .build();

    let file_logger = ResilientFileWriter::new(log_path.clone())
        .ok()
        .map(|writer| WriteLogger::new(LevelFilter::Debug, config.clone(), writer));

    #[cfg(debug_assertions)]
    {
        let term_logger = TermLogger::new(
            LevelFilter::Debug,
            config,
            TerminalMode::Mixed,
            ColorChoice::Auto,
        );

        if let Some(file_logger) = file_logger {
            let _ = CombinedLogger::init(vec![term_logger, file_logger]);
        } else {
            let _ = CombinedLogger::init(vec![term_logger]);
        }
    }

    #[cfg(not(debug_assertions))]
    {
        if let Some(file_logger) = file_logger {
            let _ = CombinedLogger::init(vec![file_logger]);
        }
    }

    set_logging_enabled(debug_enabled);

    if debug_enabled {
        log::info!("Debug logging initialized, log file: {:?}", log_path);
    }
}

pub fn set_logging_enabled(enabled: bool) {
    if enabled {
        log::set_max_level(log::LevelFilter::Debug);
    } else {
        log::set_max_level(log::LevelFilter::Off);
    }
}
