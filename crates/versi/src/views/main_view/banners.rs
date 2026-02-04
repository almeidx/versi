use chrono::{DateTime, Utc};
use iced::widget::{Space, button, column, row, text};
use iced::{Alignment, Element, Length};

use crate::message::Message;
use crate::state::{MainState, NetworkStatus};
use crate::theme::styles;

pub(super) fn contextual_banners<'a>(state: &'a MainState) -> Option<Element<'a, Message>> {
    let env = state.active_environment();
    let schedule = state.available_versions.schedule.as_ref();

    let mut banners: Vec<Element<Message>> = Vec::new();

    match state.available_versions.network_status() {
        NetworkStatus::Offline => {
            banners.push(
                button(
                    row![
                        text("Could not load available versions").size(13),
                        Space::new().width(Length::Fill),
                        text("Retry").size(13),
                    ]
                    .align_y(Alignment::Center),
                )
                .on_press(Message::FetchRemoteVersions)
                .style(styles::banner_button_warning)
                .padding([12, 16])
                .width(Length::Fill)
                .into(),
            );
        }
        NetworkStatus::Stale => {
            let age_text = state
                .available_versions
                .disk_cached_at
                .map(|t| format!(" (cached {})", format_relative_time(t)))
                .unwrap_or_default();
            banners.push(
                button(
                    row![
                        text(format!(
                            "Using cached data{} \u{2014} could not refresh from network",
                            age_text
                        ))
                        .size(13),
                        Space::new().width(Length::Fill),
                        text("Retry").size(13),
                    ]
                    .align_y(Alignment::Center),
                )
                .on_press(Message::FetchRemoteVersions)
                .style(styles::banner_button_warning)
                .padding([12, 16])
                .width(Length::Fill)
                .into(),
            );
        }
        _ => {}
    }

    if state.available_versions.schedule_error.is_some() && schedule.is_none() {
        banners.push(
            button(
                row![
                    text("Release schedule unavailable \u{2014} EOL detection may be inaccurate")
                        .size(13),
                    Space::new().width(Length::Fill),
                    text("Retry").size(13),
                ]
                .align_y(Alignment::Center),
            )
            .on_press(Message::FetchReleaseSchedule)
            .style(styles::banner_button_warning)
            .padding([12, 16])
            .width(Length::Fill)
            .into(),
        );
    }

    let latest_by_major = &state.available_versions.latest_by_major;

    let update_count = env
        .version_groups
        .iter()
        .filter(|group| {
            let installed_latest = group.versions.iter().map(|v| &v.version).max();
            latest_by_major
                .get(&group.major)
                .is_some_and(|latest| installed_latest.is_some_and(|installed| latest > installed))
        })
        .count();

    if update_count > 0 {
        banners.push(
            button(
                row![
                    text(format!(
                        "{} major {} with updates available",
                        update_count,
                        if update_count == 1 {
                            "version"
                        } else {
                            "versions"
                        }
                    ))
                    .size(13),
                    Space::new().width(Length::Fill),
                    text("Update All").size(13),
                ]
                .align_y(Alignment::Center),
            )
            .on_press(Message::RequestBulkUpdateMajors)
            .style(styles::banner_button_info)
            .padding([12, 16])
            .width(Length::Fill)
            .into(),
        );
    }

    let eol_count = schedule
        .map(|s| {
            env.version_groups
                .iter()
                .filter(|g| !s.is_active(g.major))
                .map(|g| g.versions.len())
                .sum::<usize>()
        })
        .unwrap_or(0);

    if eol_count > 0 {
        banners.push(
            button(
                row![
                    text(format!(
                        "{} end-of-life {} installed",
                        eol_count,
                        if eol_count == 1 {
                            "version"
                        } else {
                            "versions"
                        }
                    ))
                    .size(13),
                    Space::new().width(Length::Fill),
                    text("Clean Up").size(13),
                ]
                .align_y(Alignment::Center),
            )
            .on_press(Message::RequestBulkUninstallEOL)
            .style(styles::banner_button_warning)
            .padding([12, 16])
            .width(Length::Fill)
            .into(),
        );
    }

    if banners.is_empty() {
        None
    } else {
        Some(column(banners).spacing(8).into())
    }
}

fn format_relative_time(timestamp: DateTime<Utc>) -> String {
    let delta = Utc::now().signed_duration_since(timestamp);
    let minutes = delta.num_minutes();
    if minutes < 1 {
        "just now".to_string()
    } else if minutes < 60 {
        format!("{}m ago", minutes)
    } else {
        let hours = delta.num_hours();
        if hours < 24 {
            format!("{}h ago", hours)
        } else {
            let days = delta.num_days();
            format!("{}d ago", days)
        }
    }
}
