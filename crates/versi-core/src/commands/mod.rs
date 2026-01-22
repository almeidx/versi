pub use crate::backend::FnmBackend;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

pub trait HideWindow {
    fn hide_window(&mut self) -> &mut Self;
}

impl HideWindow for tokio::process::Command {
    #[cfg(windows)]
    fn hide_window(&mut self) -> &mut Self {
        self.creation_flags(CREATE_NO_WINDOW)
    }

    #[cfg(not(windows))]
    fn hide_window(&mut self) -> &mut Self {
        self
    }
}

impl HideWindow for std::process::Command {
    #[cfg(windows)]
    fn hide_window(&mut self) -> &mut Self {
        self.creation_flags(CREATE_NO_WINDOW)
    }

    #[cfg(not(windows))]
    fn hide_window(&mut self) -> &mut Self {
        self
    }
}
