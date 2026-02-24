use iced::widget::{Space, button, container, mouse_area, row, text};
use iced::{Alignment, Element, Length};

use versi_backend::{InstallProgress, RemoteVersion};

use crate::message::Message;
use crate::theme::styles;

use super::VersionListContext;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VersionRowAction {
    Installing,
    Queued,
    Installed,
    Uninstall,
    Install,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RowActivity {
    Active,
    Pending,
    Idle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InstallState {
    Installed,
    NotInstalled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HoverState {
    Hovered,
    NotHovered,
}

fn resolve_version_row_action(
    activity: RowActivity,
    install_state: InstallState,
    hover_state: HoverState,
) -> VersionRowAction {
    if activity == RowActivity::Active {
        return VersionRowAction::Installing;
    }
    if activity == RowActivity::Pending {
        return VersionRowAction::Queued;
    }
    if install_state == InstallState::Installed {
        return if hover_state == HoverState::Hovered {
            VersionRowAction::Uninstall
        } else {
            VersionRowAction::Installed
        };
    }
    VersionRowAction::Install
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VersionBadgeKind {
    Lts,
    Eol,
    Security,
}

fn version_badge_kinds(has_lts: bool, is_eol: bool, has_security: bool) -> Vec<VersionBadgeKind> {
    let mut badges = Vec::new();
    if has_lts && !is_eol {
        badges.push(VersionBadgeKind::Lts);
    }
    if is_eol {
        badges.push(VersionBadgeKind::Eol);
    }
    if has_security {
        badges.push(VersionBadgeKind::Security);
    }
    badges
}

fn format_install_progress(progress: Option<&InstallProgress>) -> String {
    match progress {
        Some(InstallProgress::Downloading {
            downloaded_bytes,
            total_bytes,
        }) if *total_bytes > 0 => {
            let downloaded_mb = format_mb_tenths(*downloaded_bytes);
            let total_mb = format_mb_tenths(*total_bytes);
            let percent = percent_complete(*downloaded_bytes, *total_bytes);
            format!("{downloaded_mb} MB / {total_mb} MB ({percent}%)")
        }
        Some(InstallProgress::Extracting) => "Extracting...".to_string(),
        Some(InstallProgress::Configuring) => "Configuring...".to_string(),
        _ => "Installing...".to_string(),
    }
}

fn format_mb_tenths(bytes: u64) -> String {
    const MB: u128 = 1024 * 1024;
    let scaled = (u128::from(bytes) * 10 + MB / 2) / MB;
    let whole = scaled / 10;
    let tenth = scaled % 10;
    format!("{whole}.{tenth}")
}

fn percent_complete(downloaded: u64, total: u64) -> u64 {
    if total == 0 {
        return 0;
    }

    let rounded = (u128::from(downloaded) * 100 + u128::from(total) / 2) / u128::from(total);
    let clamped = rounded.min(100);
    u64::try_from(clamped).unwrap_or(100)
}

fn action_button<'a>(
    action: VersionRowAction,
    version: &str,
    install_progress: Option<&InstallProgress>,
) -> Element<'a, Message> {
    match action {
        VersionRowAction::Installing => {
            button(text(format_install_progress(install_progress)).size(12))
                .style(styles::primary_button)
                .padding([6, 12])
                .into()
        }
        VersionRowAction::Queued => button(text("Queued").size(12))
            .style(styles::secondary_button)
            .padding([6, 12])
            .into(),
        VersionRowAction::Install => button(text("Install").size(12))
            .on_press(Message::StartInstall(version.to_string()))
            .style(styles::primary_button)
            .padding([6, 12])
            .into(),
        VersionRowAction::Installed => button(text("Installed").size(12))
            .style(styles::secondary_button)
            .padding([6, 12])
            .into(),
        VersionRowAction::Uninstall => button(text("Uninstall").size(12))
            .on_press(Message::RequestUninstall(version.to_string()))
            .style(styles::danger_button)
            .padding([6, 12])
            .into(),
    }
}

fn version_badges(
    version: &RemoteVersion,
    is_eol: bool,
    has_security: bool,
) -> Element<'_, Message> {
    let mut badges = row![].spacing(6).align_y(Alignment::Center);
    for badge_kind in version_badge_kinds(version.lts_codename.is_some(), is_eol, has_security) {
        badges = match badge_kind {
            VersionBadgeKind::Lts => {
                if let Some(lts) = &version.lts_codename {
                    badges.push(
                        container(text(format!("LTS: {lts}")).size(11))
                            .padding([2, 6])
                            .style(styles::badge_lts),
                    )
                } else {
                    badges
                }
            }
            VersionBadgeKind::Eol => badges.push(
                container(text("End-of-Life").size(11))
                    .padding([2, 6])
                    .style(styles::badge_eol),
            ),
            VersionBadgeKind::Security => badges.push(
                container(text("Security").size(11))
                    .padding([2, 6])
                    .style(styles::badge_security),
            ),
        };
    }
    badges.into()
}

pub(super) fn available_version_row<'a>(
    version: &'a RemoteVersion,
    ctx: &VersionListContext<'a>,
) -> Element<'a, Message> {
    let version_label = version.version.to_string();
    let meta = ctx.metadata.and_then(|m| m.get(&version_label));
    let is_eol = ctx
        .schedule
        .is_some_and(|s| !s.is_active(version.version.major));
    let is_installed = ctx.installed_set.contains(&version.version);

    let is_active = ctx.operation_queue.is_current_version(&version_label);
    let is_pending = ctx.operation_queue.has_pending_for_version(&version_label);
    let is_row_hovered = ctx
        .hovered_version
        .as_ref()
        .is_some_and(|h| h == &version_label);

    let activity = if is_active {
        RowActivity::Active
    } else if is_pending {
        RowActivity::Pending
    } else {
        RowActivity::Idle
    };
    let install_state = if is_installed {
        InstallState::Installed
    } else {
        InstallState::NotInstalled
    };
    let hover_state = if is_row_hovered {
        HoverState::Hovered
    } else {
        HoverState::NotHovered
    };
    let action = resolve_version_row_action(activity, install_state, hover_state);
    let has_security = meta.is_some_and(|m| m.security);
    let install_progress = ctx.install_progress.get(&version_label);
    let action_button = action_button(action, &version_label, install_progress);
    let badges = version_badges(version, is_eol, has_security);

    let date_text: Element<Message> = if let Some(date) = meta.map(|m| m.date.as_str()) {
        text(date)
            .size(11)
            .color(crate::theme::tokens::TEXT_MUTED)
            .into()
    } else {
        Space::new().into()
    };

    let row_content = row![
        container(text(version_label.clone()).size(14))
            .padding([2, 4])
            .width(Length::Fixed(crate::theme::tokens::COL_VERSION)),
        container(date_text).width(Length::Fixed(crate::theme::tokens::COL_DATE)),
        badges,
        Space::new().width(Length::Fill),
        action_button,
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .padding([4, 8]);

    let row_style = if is_row_hovered {
        styles::version_row_hovered
    } else {
        |_: &_| iced::widget::container::Style::default()
    };

    let row_container = container(row_content).style(row_style).width(Length::Fill);

    mouse_area(row_container)
        .on_press(Message::ShowVersionDetail(version_label.clone()))
        .on_enter(Message::VersionRowHovered(Some(version_label.clone())))
        .on_exit(Message::VersionRowHovered(None))
        .on_right_press(Message::ShowContextMenu {
            version: version_label,
            is_installed,
            is_default: false,
        })
        .into()
}

#[cfg(test)]
mod tests {
    use super::{
        HoverState, InstallState, RowActivity, VersionBadgeKind, VersionRowAction,
        format_install_progress, format_mb_tenths, percent_complete, resolve_version_row_action,
        version_badge_kinds,
    };
    use versi_backend::InstallProgress;

    #[test]
    fn row_action_prioritizes_active_then_pending() {
        assert_eq!(
            resolve_version_row_action(
                RowActivity::Active,
                InstallState::Installed,
                HoverState::Hovered
            ),
            VersionRowAction::Installing
        );
        assert_eq!(
            resolve_version_row_action(
                RowActivity::Pending,
                InstallState::Installed,
                HoverState::Hovered
            ),
            VersionRowAction::Queued
        );
    }

    #[test]
    fn row_action_handles_installed_hover_state() {
        assert_eq!(
            resolve_version_row_action(
                RowActivity::Idle,
                InstallState::Installed,
                HoverState::Hovered
            ),
            VersionRowAction::Uninstall
        );
        assert_eq!(
            resolve_version_row_action(
                RowActivity::Idle,
                InstallState::Installed,
                HoverState::NotHovered
            ),
            VersionRowAction::Installed
        );
    }

    #[test]
    fn row_action_falls_back_to_install_for_uninstalled_versions() {
        assert_eq!(
            resolve_version_row_action(
                RowActivity::Idle,
                InstallState::NotInstalled,
                HoverState::NotHovered
            ),
            VersionRowAction::Install
        );
    }

    #[test]
    fn badge_kinds_include_lts_and_security_when_supported() {
        assert_eq!(
            version_badge_kinds(true, false, true),
            vec![VersionBadgeKind::Lts, VersionBadgeKind::Security]
        );
    }

    #[test]
    fn badge_kinds_hide_lts_for_eol_and_keep_order() {
        assert_eq!(
            version_badge_kinds(true, true, true),
            vec![VersionBadgeKind::Eol, VersionBadgeKind::Security]
        );
        assert!(version_badge_kinds(false, false, false).is_empty());
    }

    #[test]
    fn install_progress_text_formats_expected_states() {
        assert_eq!(
            format_install_progress(Some(&InstallProgress::Downloading {
                downloaded_bytes: 5 * 1024 * 1024,
                total_bytes: 20 * 1024 * 1024,
            })),
            "5.0 MB / 20.0 MB (25%)"
        );
        assert_eq!(
            format_install_progress(Some(&InstallProgress::Extracting)),
            "Extracting..."
        );
        assert_eq!(
            format_install_progress(Some(&InstallProgress::Configuring)),
            "Configuring..."
        );
        assert_eq!(format_install_progress(None), "Installing...");
    }

    #[test]
    fn progress_numeric_helpers_round_and_clamp() {
        assert_eq!(format_mb_tenths(5 * 1024 * 1024), "5.0");
        assert_eq!(format_mb_tenths(1_572_864), "1.5");
        assert_eq!(percent_complete(25, 100), 25);
        assert_eq!(percent_complete(120, 100), 100);
        assert_eq!(percent_complete(1, 0), 0);
    }
}
