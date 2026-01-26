#[cfg(windows)]
mod windows_impl {
    use std::ptr;
    use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, ERROR_ALREADY_EXISTS, HANDLE};
    use windows_sys::Win32::System::Threading::CreateMutexA;

    const MUTEX_NAME: &[u8] = b"Global\\VersiAppMutex\0";

    pub struct SingleInstance {
        handle: HANDLE,
    }

    impl SingleInstance {
        pub fn acquire() -> Result<Self, ()> {
            unsafe {
                let handle = CreateMutexA(ptr::null(), 1, MUTEX_NAME.as_ptr());

                if handle.is_null() {
                    return Err(());
                }

                let last_error = GetLastError();
                if last_error == ERROR_ALREADY_EXISTS {
                    CloseHandle(handle);
                    return Err(());
                }

                Ok(Self { handle })
            }
        }
    }

    impl Drop for SingleInstance {
        fn drop(&mut self) {
            unsafe {
                CloseHandle(self.handle);
            }
        }
    }

    pub fn bring_existing_window_to_front() {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            FindWindowA, SetForegroundWindow, ShowWindow, SW_RESTORE,
        };

        unsafe {
            let hwnd = FindWindowA(ptr::null(), b"Versi\0".as_ptr());
            if !hwnd.is_null() {
                ShowWindow(hwnd, SW_RESTORE);
                SetForegroundWindow(hwnd);
            }
        }
    }
}

#[cfg(not(windows))]
mod other_impl {
    pub struct SingleInstance;

    impl SingleInstance {
        pub fn acquire() -> Result<Self, ()> {
            Ok(Self)
        }
    }

    pub fn bring_existing_window_to_front() {}
}

#[cfg(not(windows))]
pub use other_impl::{bring_existing_window_to_front, SingleInstance};
#[cfg(windows)]
pub use windows_impl::{bring_existing_window_to_front, SingleInstance};
