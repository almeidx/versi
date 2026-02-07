#[cfg(target_os = "macos")]
pub(super) fn set_dock_visible(visible: bool) {
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy};

    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    let app = NSApplication::sharedApplication(mtm);
    let policy = if visible {
        NSApplicationActivationPolicy::Regular
    } else {
        NSApplicationActivationPolicy::Accessory
    };
    app.setActivationPolicy(policy);
}

#[cfg(not(target_os = "macos"))]
pub(super) fn set_dock_visible(_visible: bool) {}

#[cfg(target_os = "linux")]
pub(super) fn is_wayland() -> bool {
    std::env::var("XDG_SESSION_TYPE")
        .map(|v| v == "wayland")
        .unwrap_or_else(|_| std::env::var("WAYLAND_DISPLAY").is_ok())
}

#[cfg(not(target_os = "linux"))]
pub(super) fn is_wayland() -> bool {
    false
}

pub(super) fn reveal_in_file_manager(path: &std::path::Path) {
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open")
            .args(["-R", &path.to_string_lossy()])
            .spawn();
    }

    #[cfg(target_os = "windows")]
    {
        use versi_core::HideWindow;
        let _ = std::process::Command::new("explorer")
            .arg(format!("/select,{}", path.to_string_lossy()))
            .hide_window()
            .spawn();
    }

    #[cfg(target_os = "linux")]
    {
        if let Some(parent) = path.parent() {
            let _ = std::process::Command::new("xdg-open").arg(parent).spawn();
        }
    }
}
