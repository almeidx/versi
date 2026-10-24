pub(crate) fn set_update_badge(visible: bool) {
    use log::debug;
    if let Err(error) = badge_sender().send(visible) {
        debug!("Failed to queue update badge request: {error}");
    }
}

fn badge_sender() -> &'static std::sync::mpsc::Sender<bool> {
    use std::sync::{OnceLock, mpsc};

    static BADGE_SENDER: OnceLock<mpsc::Sender<bool>> = OnceLock::new();

    BADGE_SENDER.get_or_init(|| {
        let (sender, receiver) = mpsc::channel::<bool>();
        std::thread::spawn(move || run_badge_worker(&receiver));
        sender
    })
}

fn run_badge_worker(receiver: &std::sync::mpsc::Receiver<bool>) {
    use log::debug;

    let mut connection: Option<zbus::Connection> = None;

    while let Ok(mut visible) = receiver.recv() {
        while let Ok(next) = receiver.try_recv() {
            visible = next;
        }

        if connection.is_none() {
            match zbus::block_on(zbus::Connection::session()) {
                Ok(new_connection) => connection = Some(new_connection),
                Err(error) => {
                    debug!("Failed to connect to session bus for update badge: {error}");
                    continue;
                }
            }
        }

        if let Some(active_connection) = connection.as_ref()
            && let Err(error) = emit_badge_update(active_connection, visible)
        {
            debug!("Failed to set update badge: {error}");
            connection = None;
        }
    }
}

fn emit_badge_update(connection: &zbus::Connection, visible: bool) -> Result<(), zbus::Error> {
    let count = i64::from(visible);
    let mut props = std::collections::HashMap::new();
    props.insert("count", zbus::Variant::from(count));
    props.insert("count-visible", zbus::Variant::from(visible));

    zbus::block_on(connection.emit_signal(
        None::<zbus::names::BusName>,
        "/",
        "com.canonical.Unity.LauncherEntry",
        "Update",
        &(
            format!("application://{}", versi_platform::DESKTOP_ENTRY_FILENAME),
            props,
        ),
    ))
}

pub(crate) fn set_dock_visible(_visible: bool) {}

pub(crate) fn is_wayland() -> bool {
    is_wayland_with(|name| std::env::var(name))
}

fn is_wayland_with<F>(get_var: F) -> bool
where
    F: for<'a> Fn(&'a str) -> Result<String, std::env::VarError>,
{
    get_var("XDG_SESSION_TYPE").map_or_else(
        |_| get_var("WAYLAND_DISPLAY").is_ok(),
        |value| value == "wayland",
    )
}

use super::LaunchAtLoginError;

pub(crate) fn set_launch_at_login(enable: bool) -> Result<(), LaunchAtLoginError> {
    use std::fs;

    let autostart_dir = dirs::config_dir()
        .ok_or(LaunchAtLoginError::ConfigDirectoryUnavailable)?
        .join("autostart");
    let desktop_path = autostart_dir.join(versi_platform::DESKTOP_ENTRY_FILENAME);

    if !enable {
        if desktop_path.exists() {
            fs::remove_file(&desktop_path)
                .map_err(|error| LaunchAtLoginError::io("failed to remove desktop entry", error))?;
        }
        return Ok(());
    }

    let exe = std::env::current_exe()
        .map_err(|error| LaunchAtLoginError::io("failed to resolve current executable", error))?;
    let exec_entry = desktop_exec_for_path(&exe);
    let entry = format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Name=Versi\n\
         Exec={exec_entry}\n\
         X-GNOME-Autostart-enabled=true\n"
    );

    fs::create_dir_all(&autostart_dir)
        .map_err(|error| LaunchAtLoginError::io("failed to create autostart directory", error))?;
    fs::write(&desktop_path, entry)
        .map_err(|error| LaunchAtLoginError::io("failed to write desktop entry", error))?;
    Ok(())
}

fn desktop_exec_for_path(path: &std::path::Path) -> String {
    let raw = path.to_string_lossy();
    if raw.chars().any(char::is_whitespace) {
        let escaped = raw.replace('\\', "\\\\").replace('"', "\\\"");
        format!("\"{escaped}\"")
    } else {
        raw.into_owned()
    }
}

pub(crate) fn reveal_in_file_manager(path: &std::path::Path) {
    if let Some(parent) = path.parent() {
        let _ = std::process::Command::new("xdg-open").arg(parent).spawn();
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::desktop_exec_for_path;
    use super::is_wayland_with;

    #[test]
    fn xdg_session_wayland_is_detected() {
        let result = is_wayland_with(|name| match name {
            "XDG_SESSION_TYPE" => Ok("wayland".to_string()),
            _ => Err(std::env::VarError::NotPresent),
        });

        assert!(result);
    }

    #[test]
    fn xdg_session_non_wayland_returns_false_without_fallback() {
        let result = is_wayland_with(|name| match name {
            "XDG_SESSION_TYPE" => Ok("x11".to_string()),
            _ => Err(std::env::VarError::NotPresent),
        });

        assert!(!result);
    }

    #[test]
    fn falls_back_to_wayland_display_when_session_type_missing() {
        let result = is_wayland_with(|name| {
            if name == "WAYLAND_DISPLAY" {
                Ok("wayland-0".to_string())
            } else {
                Err(std::env::VarError::NotPresent)
            }
        });

        assert!(result);
    }

    #[test]
    fn returns_false_when_no_wayland_variables_exist() {
        let result = is_wayland_with(|_| Err(std::env::VarError::NotPresent));

        assert!(!result);
    }

    #[test]
    fn desktop_exec_quotes_paths_with_spaces() {
        let exec = desktop_exec_for_path(Path::new("/opt/Versi App/versi"));
        assert_eq!(exec, "\"/opt/Versi App/versi\"");
    }

    #[test]
    fn desktop_exec_keeps_simple_paths_unquoted() {
        let exec = desktop_exec_for_path(Path::new("/usr/local/bin/versi"));
        assert_eq!(exec, "/usr/local/bin/versi");
    }
}
