use thiserror::Error;

#[derive(Debug, Error)]
pub enum AcquireError {
    #[error("another Versi instance is already running")]
    AlreadyRunning,
    #[error("failed to resolve application paths: {0}")]
    Paths(#[from] versi_platform::AppPathsError),
    #[cfg(not(windows))]
    #[error("{context}: {source}")]
    Io {
        context: &'static str,
        #[source]
        source: std::io::Error,
    },
    #[cfg(windows)]
    #[error("win32 call {api} failed with code {code}")]
    Win32 { api: &'static str, code: u32 },
}

impl AcquireError {
    #[cfg(not(windows))]
    fn io(context: &'static str, source: std::io::Error) -> Self {
        Self::Io { context, source }
    }
}

#[cfg(windows)]
mod windows_impl {
    use std::ptr;
    use windows_sys::Win32::Foundation::{CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HANDLE};
    use windows_sys::Win32::System::Threading::CreateMutexA;

    const MUTEX_NAME: &[u8] = b"Global\\VersiAppMutex\0";

    pub struct SingleInstance {
        handle: HANDLE,
    }

    impl SingleInstance {
        pub fn acquire() -> Result<Self, super::AcquireError> {
            // SAFETY: calling Win32 mutex APIs with a static NUL-terminated
            // name and null security attributes is valid here; handle results
            // are checked before use.
            unsafe {
                let handle = CreateMutexA(ptr::null(), 1, MUTEX_NAME.as_ptr());

                if handle.is_null() {
                    let code = GetLastError();
                    return Err(super::AcquireError::Win32 {
                        api: "CreateMutexA",
                        code,
                    });
                }

                let last_error = GetLastError();
                if last_error == ERROR_ALREADY_EXISTS {
                    CloseHandle(handle);
                    return Err(super::AcquireError::AlreadyRunning);
                }

                Ok(Self { handle })
            }
        }
    }

    impl Drop for SingleInstance {
        fn drop(&mut self) {
            // SAFETY: `self.handle` was returned by `CreateMutexA` and remains
            // owned by this guard until drop.
            unsafe {
                CloseHandle(self.handle);
            }
        }
    }

    pub fn bring_existing_window_to_front() {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            SW_RESTORE, SetForegroundWindow, ShowWindow,
        };

        // SAFETY: `find_versi_window` returns a HWND owned by the system.
        // `ShowWindow`/`SetForegroundWindow` are invoked only when a handle is
        // found.
        unsafe {
            if let Some(hwnd) = crate::windows_window::find_versi_window() {
                ShowWindow(hwnd, SW_RESTORE);
                SetForegroundWindow(hwnd);
            }
        }
    }
}

#[cfg(not(windows))]
mod other_impl {
    use std::fs::OpenOptions;
    use std::io::{Seek, SeekFrom, Write};
    use std::path::PathBuf;

    use fd_lock::RwLock;
    use versi_platform::AppPaths;

    fn lock_file_path() -> Result<PathBuf, super::AcquireError> {
        let paths = AppPaths::new()?;
        paths
            .ensure_dirs()
            .map_err(|error| super::AcquireError::io("failed to create app directories", error))?;
        Ok(paths.data_dir.join("instance.lock"))
    }

    pub struct SingleInstance {
        _lock: RwLock<std::fs::File>,
    }

    impl SingleInstance {
        pub fn acquire() -> Result<Self, super::AcquireError> {
            let lock_file_path = lock_file_path()?;
            let file = OpenOptions::new()
                .create(true)
                .read(true)
                .write(true)
                .truncate(false)
                .open(lock_file_path)
                .map_err(|error| {
                    super::AcquireError::io("failed to open instance lock file", error)
                })?;

            let mut lock = RwLock::new(file);

            let mut guard = match lock.try_write() {
                Ok(guard) => guard,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    return Err(super::AcquireError::AlreadyRunning);
                }
                Err(error) => {
                    return Err(super::AcquireError::io(
                        "failed to acquire instance lock",
                        error,
                    ));
                }
            };

            guard
                .set_len(0)
                .and_then(|()| guard.seek(SeekFrom::Start(0)).map(|_| ()))
                .and_then(|()| writeln!(guard, "{}", std::process::id()))
                .map_err(|error| {
                    super::AcquireError::io("failed to write instance lock metadata", error)
                })?;

            // Skip the guard destructor so the OS-level flock is not released.
            // The lock will be released when the file descriptor is closed
            // (i.e. when the RwLock is dropped).
            std::mem::forget(guard);

            Ok(Self { _lock: lock })
        }
    }

    pub fn bring_existing_window_to_front() {}
}

#[cfg(not(windows))]
pub use other_impl::{SingleInstance, bring_existing_window_to_front};
#[cfg(windows)]
pub use windows_impl::{SingleInstance, bring_existing_window_to_front};

#[cfg(all(test, not(windows)))]
mod tests {
    use super::{SingleInstance, bring_existing_window_to_front};

    #[test]
    fn non_windows_acquire_returns_instance() {
        assert!(SingleInstance::acquire().is_ok());
    }

    #[test]
    fn non_windows_bring_existing_window_to_front_is_noop() {
        bring_existing_window_to_front();
    }
}
