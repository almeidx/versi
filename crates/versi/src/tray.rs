use std::cell::RefCell;

use iced::Subscription;
use iced::futures::SinkExt;
use thiserror::Error;
use tray_icon::menu::{Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};
use versi_backend::NodeVersion;
use versi_platform::EnvironmentId;

use crate::message::Message;
use crate::settings::TrayBehavior;
use crate::state::EnvironmentState;

thread_local! {
    static TRAY_ICON: RefCell<Option<TrayIcon>> = const { RefCell::new(None) };
}

#[derive(Debug, Error)]
pub enum TrayError {
    #[cfg(target_os = "linux")]
    #[error("no tray host detected (StatusNotifierWatcher not registered on D-Bus)")]
    NoTrayHost,
    #[error("failed to decode tray icon asset: {0}")]
    IconDecode(#[from] image::ImageError),
    #[error("failed to build tray icon: {0}")]
    IconData(#[from] tray_icon::BadIcon),
    #[error("failed to initialize tray icon: {0}")]
    Init(#[from] tray_icon::Error),
}

#[derive(Debug, Clone)]
pub enum TrayMessage {
    ShowWindow,
    HideWindow,
    OpenSettings,
    OpenAbout,
    Quit,
    SetDefault {
        env_id: EnvironmentId,
        version: String,
    },
}

pub struct TrayMenuData {
    pub environments: Vec<EnvironmentData>,
    pub window_visible: bool,
}

pub struct EnvironmentData {
    pub name: String,
    pub env_index: usize,
    pub env_id: EnvironmentId,
    pub versions: Vec<VersionData>,
}

pub struct VersionData {
    pub version: String,
    pub is_default: bool,
}

impl TrayMenuData {
    pub fn from_environments(environments: &[EnvironmentState], window_visible: bool) -> Self {
        Self {
            window_visible,
            environments: environments
                .iter()
                .enumerate()
                .filter(|(_, env)| env.available && !env.installed_versions.is_empty())
                .map(|(idx, env)| EnvironmentData {
                    name: env.name.clone(),
                    env_index: idx,
                    env_id: env.id.clone(),
                    versions: env
                        .installed_versions
                        .iter()
                        .map(|v| VersionData {
                            version: v.version.to_string(),
                            is_default: v.is_default,
                        })
                        .collect(),
                })
                .collect(),
        }
    }
}

pub fn init_tray(behavior: TrayBehavior) -> Result<(), TrayError> {
    if behavior == TrayBehavior::Disabled {
        return Ok(());
    }

    #[cfg(target_os = "linux")]
    if !has_tray_host() {
        return Err(TrayError::NoTrayHost);
    }

    let icon = load_icon()?;
    let menu = build_menu(&TrayMenuData {
        environments: vec![],
        window_visible: true,
    });

    let tray_icon = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("Versi")
        .with_icon(icon)
        .build()
        .map_err(TrayError::from)?;

    TRAY_ICON.with(|cell| {
        *cell.borrow_mut() = Some(tray_icon);
    });

    Ok(())
}

#[cfg(target_os = "linux")]
fn has_tray_host() -> bool {
    std::process::Command::new("dbus-send")
        .args([
            "--session",
            "--print-reply",
            "--reply-timeout=3000",
            "--dest=org.freedesktop.DBus",
            "/org/freedesktop/DBus",
            "org.freedesktop.DBus.NameHasOwner",
            "string:org.kde.StatusNotifierWatcher",
        ])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).contains("true"))
        .unwrap_or(false)
}

pub fn destroy_tray() {
    TRAY_ICON.with(|cell| {
        *cell.borrow_mut() = None;
    });
}

pub fn is_tray_active() -> bool {
    TRAY_ICON.with(|cell| cell.borrow().is_some())
}

fn load_icon() -> Result<Icon, TrayError> {
    let icon_bytes = include_bytes!("../../../assets/logo.png");
    let img = image::load_from_memory(icon_bytes).map_err(TrayError::from)?;
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();
    Icon::from_rgba(rgba.into_raw(), width, height).map_err(TrayError::from)
}

fn build_menu(data: &TrayMenuData) -> Menu {
    let menu = Menu::new();
    let show_multiple_envs = data.environments.len() > 1;

    for (i, env) in data.environments.iter().enumerate() {
        if show_multiple_envs {
            let _ = menu.append(&MenuItem::with_id(
                MenuId::new(format!("env_header:{}", env.env_index)),
                &env.name,
                false,
                None,
            ));
        }

        for ver in &env.versions {
            let label = if ver.is_default {
                format!("{} ✓", ver.version)
            } else {
                ver.version.clone()
            };

            let _ = menu.append(&MenuItem::with_id(
                MenuId::new(format!(
                    "set:{}:{}",
                    encode_environment_id(&env.env_id).unwrap_or_else(|| "invalid-env".to_string()),
                    ver.version
                )),
                label,
                true,
                None,
            ));
        }

        if show_multiple_envs && i < data.environments.len() - 1 {
            let _ = menu.append(&PredefinedMenuItem::separator());
        }
    }

    if !data.environments.is_empty() && data.environments.iter().any(|e| !e.versions.is_empty()) {
        let _ = menu.append(&PredefinedMenuItem::separator());
    }

    if data.window_visible {
        let _ = menu.append(&MenuItem::with_id(
            MenuId::new("hide_window"),
            "Hide Versi",
            true,
            None,
        ));
    } else {
        let _ = menu.append(&MenuItem::with_id(
            MenuId::new("show_window"),
            "Open Versi",
            true,
            None,
        ));
    }
    let _ = menu.append(&MenuItem::with_id(
        MenuId::new("open_settings"),
        "Settings",
        true,
        None,
    ));
    let _ = menu.append(&MenuItem::with_id(
        MenuId::new("open_about"),
        "About",
        true,
        None,
    ));
    let _ = menu.append(&PredefinedMenuItem::separator());
    let _ = menu.append(&MenuItem::with_id(MenuId::new("quit"), "Quit", true, None));

    menu
}

pub fn update_menu(data: &TrayMenuData) {
    TRAY_ICON.with(|cell| {
        if let Some(tray) = cell.borrow().as_ref() {
            let menu = build_menu(data);
            tray.set_menu(Some(Box::new(menu)));
        }
    });
}

pub fn tooltip_text(default_version: Option<&NodeVersion>) -> String {
    match default_version {
        Some(version) => format!("Versi - Node {version}"),
        None => "Versi".to_string(),
    }
}

#[cfg(target_os = "linux")]
pub fn update_tooltip(_tooltip: &str) {}

#[cfg(not(target_os = "linux"))]
pub fn update_tooltip(tooltip: &str) {
    TRAY_ICON.with(|cell| {
        if let Some(tray) = cell.borrow().as_ref()
            && let Err(error) = tray.set_tooltip(Some(tooltip))
        {
            log::warn!("Failed to update tray tooltip: {error}");
        }
    });
}

fn parse_menu_event(id: &str) -> Option<TrayMessage> {
    match id {
        "show_window" => Some(TrayMessage::ShowWindow),
        "hide_window" => Some(TrayMessage::HideWindow),
        "open_settings" => Some(TrayMessage::OpenSettings),
        "open_about" => Some(TrayMessage::OpenAbout),
        "quit" => Some(TrayMessage::Quit),
        s if s.starts_with("set:") => {
            let parts: Vec<&str> = s.splitn(3, ':').collect();
            if parts.len() == 3 {
                let env_id = decode_environment_id(parts[1])?;
                let version = parts[2].to_string();
                Some(TrayMessage::SetDefault { env_id, version })
            } else {
                None
            }
        }
        other => {
            log::warn!("Unknown tray menu event ID: {other}");
            None
        }
    }
}

fn encode_environment_id(env_id: &EnvironmentId) -> Option<String> {
    let bytes = serde_json::to_vec(env_id).ok()?;
    Some(hex_encode(&bytes))
}

fn decode_environment_id(encoded: &str) -> Option<EnvironmentId> {
    let bytes = hex_decode(encoded)?;
    serde_json::from_slice(&bytes).ok()
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        let _ = write!(out, "{byte:02x}");
    }
    out
}

fn hex_decode(value: &str) -> Option<Vec<u8>> {
    if !value.len().is_multiple_of(2) {
        return None;
    }

    let mut out = Vec::with_capacity(value.len() / 2);
    for pair in value.as_bytes().chunks_exact(2) {
        let hi = hex_nibble(pair[0])?;
        let lo = hex_nibble(pair[1])?;
        out.push((hi << 4) | lo);
    }
    Some(out)
}

fn hex_nibble(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}

pub fn tray_subscription() -> Subscription<Message> {
    Subscription::run(|| {
        iced::stream::channel(
            16,
            |mut output: iced::futures::channel::mpsc::Sender<Message>| async move {
                let (event_tx, mut event_rx) = tokio::sync::mpsc::channel(16);

                MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
                    let id_str = event.id().as_ref();
                    if let Some(message) = parse_menu_event(id_str)
                        && let Err(tokio::sync::mpsc::error::TrySendError::Full(_)) =
                            event_tx.try_send(message)
                    {
                        log::debug!("Tray event queue full; dropping event");
                    }
                }));

                while let Some(message) = event_rx.recv().await {
                    if output.send(Message::TrayEvent(message)).await.is_err() {
                        break;
                    }
                }

                MenuEvent::set_event_handler(None::<fn(MenuEvent)>);
            },
        )
    })
}

#[cfg(test)]
mod tests {
    use versi_platform::EnvironmentId;

    use super::{TrayMenuData, TrayMessage, encode_environment_id, parse_menu_event, tooltip_text};
    use crate::backend_kind::BackendKind;
    use crate::state::EnvironmentState;

    fn installed(version: &str, is_default: bool) -> versi_backend::InstalledVersion {
        crate::test_fixtures::installed(version, is_default)
    }

    #[test]
    fn tray_menu_data_includes_only_available_environments_with_versions() {
        let mut native = EnvironmentState::new(EnvironmentId::Native, BackendKind::Fnm, None);
        native.available = true;
        native.loading = false;
        native.installed_versions = vec![installed("v20.11.0", true)];

        let mut unavailable =
            EnvironmentState::unavailable(EnvironmentId::Native, BackendKind::Fnm, "offline");
        unavailable.installed_versions = vec![installed("v18.19.1", false)];

        let mut empty = EnvironmentState::new(
            EnvironmentId::Wsl {
                distro: "Ubuntu".to_string(),
                backend_path: "/home/user/.nvm/nvm.sh".to_string(),
            },
            BackendKind::Nvm,
            None,
        );
        empty.available = true;
        empty.loading = false;
        empty.installed_versions.clear();

        let menu = TrayMenuData::from_environments(&[native, unavailable, empty], true);

        assert!(menu.window_visible);
        assert_eq!(menu.environments.len(), 1);
        assert_eq!(menu.environments[0].env_index, 0);
        assert_eq!(menu.environments[0].env_id, EnvironmentId::Native);
        assert_eq!(menu.environments[0].versions.len(), 1);
        assert_eq!(menu.environments[0].versions[0].version, "v20.11.0");
        assert!(menu.environments[0].versions[0].is_default);
    }

    #[test]
    fn parse_menu_event_maps_actions_and_set_default_payload() {
        assert!(matches!(
            parse_menu_event("show_window"),
            Some(TrayMessage::ShowWindow)
        ));
        assert!(matches!(
            parse_menu_event("hide_window"),
            Some(TrayMessage::HideWindow)
        ));
        assert!(matches!(
            parse_menu_event("open_settings"),
            Some(TrayMessage::OpenSettings)
        ));
        assert!(matches!(
            parse_menu_event("open_about"),
            Some(TrayMessage::OpenAbout)
        ));
        assert!(matches!(parse_menu_event("quit"), Some(TrayMessage::Quit)));

        let native_id =
            encode_environment_id(&EnvironmentId::Native).expect("native environment id encoding");
        assert!(matches!(
            parse_menu_event(&format!("set:{native_id}:v20.11.0")),
            Some(TrayMessage::SetDefault { env_id, version })
                if env_id == EnvironmentId::Native && version == "v20.11.0"
        ));
        assert!(parse_menu_event("set:nothex:v20.11.0").is_none());
        assert!(parse_menu_event("unknown").is_none());
    }

    #[test]
    fn tooltip_text_reflects_default_version_when_available() {
        let default_version: versi_backend::NodeVersion =
            "v22.11.0".parse().expect("test version should parse");

        assert_eq!(
            tooltip_text(Some(&default_version)),
            "Versi - Node v22.11.0"
        );
        assert_eq!(tooltip_text(None), "Versi");
    }
}
