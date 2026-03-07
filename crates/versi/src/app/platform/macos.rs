use super::LaunchAtLoginError;

pub(crate) fn set_update_badge(visible: bool) {
    use objc2::MainThreadMarker;
    use objc2_app_kit::NSApplication;
    use objc2_foundation::NSString;

    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    let app = NSApplication::sharedApplication(mtm);
    let tile = app.dockTile();
    if visible {
        tile.setBadgeLabel(Some(&NSString::from_str("1")));
    } else {
        tile.setBadgeLabel(None);
    }
}

pub(crate) fn set_dock_visible(visible: bool) {
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

pub(crate) fn is_wayland() -> bool {
    false
}

pub(crate) fn set_launch_at_login(enable: bool) -> Result<(), LaunchAtLoginError> {
    use std::fs;
    use std::path::Path;

    let home = dirs::home_dir().ok_or(LaunchAtLoginError::HomeDirectoryUnavailable)?;
    let plist_path = home
        .join("Library/LaunchAgents")
        .join(versi_platform::LAUNCHAGENT_PLIST_FILENAME);

    if !enable {
        if plist_path.exists() {
            fs::remove_file(&plist_path)
                .map_err(|error| LaunchAtLoginError::io("failed to remove launch agent", error))?;
        }
        return Ok(());
    }

    let exe = std::env::current_exe()
        .map_err(|error| LaunchAtLoginError::io("failed to resolve current executable", error))?;

    let bundle_root = exe.ancestors().find(|ancestor| {
        ancestor.file_name().and_then(|n| Path::new(n).extension()) == Some("app".as_ref())
    });

    let (program_arg, extra_arg) = if let Some(bundle_path) = bundle_root {
        let escaped = escape_xml(&bundle_path.to_string_lossy());
        (
            "open".to_string(),
            format!("<string>-a</string>\n    <string>{escaped}</string>"),
        )
    } else {
        (escape_xml(&exe.to_string_lossy()), String::new())
    };

    let extra_line = if extra_arg.is_empty() {
        String::new()
    } else {
        format!("\n    {extra_arg}")
    };

    let app_id = versi_platform::APP_ID;
    let plist = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{app_id}</string>
    <key>ProgramArguments</key>
    <array>
    <string>{program_arg}</string>{extra_line}
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <false/>
</dict>
</plist>
"#
    );

    if let Some(parent) = plist_path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            LaunchAtLoginError::io("failed to create LaunchAgents directory", error)
        })?;
    }
    fs::write(&plist_path, plist)
        .map_err(|error| LaunchAtLoginError::io("failed to write launch agent plist", error))?;
    Ok(())
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

pub(crate) fn reveal_in_file_manager(path: &std::path::Path) {
    let _ = std::process::Command::new("open")
        .args(["-R", &path.to_string_lossy()])
        .spawn();
}
