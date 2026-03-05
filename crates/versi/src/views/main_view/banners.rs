use chrono::{DateTime, Utc};
use iced::widget::{Space, button, column, container, progress_bar, row, text};
use iced::{Alignment, Color, Element, Length};
use versi_backend::InstallProgress;

use crate::message::Message;
use crate::state::{BulkItemStatus, BulkRunAction, BulkRunKind, MainState, NetworkStatus};
use crate::theme::styles;

pub(super) fn contextual_banners(state: &MainState) -> Option<Element<'_, Message>> {
    let schedule = state.available_versions.schedule.as_ref();

    let mut banners: Vec<Element<Message>> = Vec::new();

    if let Some(network_banner) = network_status_banner(state) {
        banners.push(network_banner);
    }

    if let Some(schedule_banner) = release_schedule_banner(state, schedule.is_some()) {
        banners.push(schedule_banner);
    }

    if let Some(metadata_banner) =
        metadata_banner(state, state.available_versions.metadata.is_some())
    {
        banners.push(metadata_banner);
    }

    if let Some(security_advisories_banner) = security_advisories_banner(
        state,
        state.available_versions.security_advisories.is_some(),
    ) {
        banners.push(security_advisories_banner);
    }

    if let Some(vulnerability_banner) = vulnerability_banner(state) {
        banners.push(vulnerability_banner);
    }

    if let Some(update_banner) = available_updates_banner(state) {
        banners.push(update_banner);
    }

    if let Some(eol_banner) = eol_cleanup_banner(state) {
        banners.push(eol_banner);
    }

    if banners.is_empty() {
        None
    } else {
        Some(column(banners).spacing(8).into())
    }
}

pub(super) fn bulk_operation_progress_banner(state: &MainState) -> Option<Element<'_, Message>> {
    const PREVIEW_LIMIT: usize = 5;

    let run = state.bulk_run.as_ref()?;
    if !run.is_active() {
        return None;
    }

    let snapshot = BulkProgressSnapshot::from_run(run, &state.install_progress)?;
    let title = format!(
        "{} {} of {} versions...",
        bulk_action_verb(run.kind),
        snapshot.current,
        snapshot.total
    );
    let mut lines: Vec<Element<Message>> = vec![
        bulk_progress_header_line(title, snapshot.percent(), snapshot.pending),
        progress_bar(0.0..=1.0, snapshot.progress()).into(),
        text(format!(
            "Pending: {}  Completed: {}  Failed: {}  Canceled: {}",
            snapshot.pending, snapshot.completed, snapshot.failed, snapshot.canceled
        ))
        .size(11)
        .color(crate::theme::tokens::TEXT_MUTED)
        .into(),
    ];
    lines.extend(bulk_progress_version_lines(&snapshot, PREVIEW_LIMIT));

    Some(
        container(column(lines).spacing(6))
            .style(styles::card_container)
            .padding([10, 12])
            .width(Length::Fill)
            .into(),
    )
}

fn bulk_action_verb(kind: BulkRunKind) -> &'static str {
    match kind {
        BulkRunKind::UpdateMajors => "Updating",
        BulkRunKind::UninstallEol
        | BulkRunKind::UninstallMajor
        | BulkRunKind::UninstallMajorExceptLatest => "Uninstalling",
    }
}

struct BulkProgressSnapshot {
    total: usize,
    completed: usize,
    failed: usize,
    canceled: usize,
    pending: usize,
    current: usize,
    progress_basis_points: u16,
    pending_versions: Vec<String>,
    completed_versions: Vec<String>,
    failed_versions: Vec<String>,
    canceled_versions: Vec<String>,
}

impl BulkProgressSnapshot {
    fn from_run(
        run: &crate::state::BulkRunState,
        install_progress: &std::collections::HashMap<String, InstallProgress>,
    ) -> Option<Self> {
        let total = run.total_count();
        if total == 0 {
            return None;
        }

        let completed = run.completed_count();
        let failed = run.failed_count();
        let canceled = run.canceled_count();
        let running = run.running_count();
        let pending = run.pending_count();

        let finished = completed + failed + canceled;
        let current = if running > 0 {
            (finished + 1).min(total)
        } else {
            finished.min(total)
        };

        Some(Self {
            total,
            completed,
            failed,
            canceled,
            pending,
            current,
            progress_basis_points: overall_bulk_progress_basis_points(run, install_progress),
            pending_versions: run.pending_versions(),
            completed_versions: run.completed_versions(),
            failed_versions: run.failed_versions(),
            canceled_versions: run.canceled_versions(),
        })
    }

    fn progress(&self) -> f32 {
        f32::from(self.progress_basis_points) / 10_000.0
    }

    fn percent(&self) -> u32 {
        (u32::from(self.progress_basis_points) + 50) / 100
    }
}

fn bulk_progress_header_line(
    title: String,
    percent: u32,
    pending: usize,
) -> Element<'static, Message> {
    let cancel_button = {
        let button = button(text("Cancel Remaining").size(12))
            .style(styles::danger_button)
            .padding([6, 12]);

        if pending > 0 {
            button.on_press(Message::CancelBulkRun)
        } else {
            button
        }
    };

    row![
        text(title).size(13),
        Space::new().width(Length::Fill),
        text(format!("{percent}%"))
            .size(12)
            .color(crate::theme::tokens::TEXT_MUTED),
        cancel_button,
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
}

fn bulk_progress_version_lines(
    snapshot: &BulkProgressSnapshot,
    limit: usize,
) -> Vec<Element<'static, Message>> {
    [
        version_preview_line(
            "Pending versions",
            &snapshot.pending_versions,
            limit,
            crate::theme::tokens::TEXT_MUTED,
        ),
        version_preview_line(
            "Completed versions",
            &snapshot.completed_versions,
            limit,
            crate::theme::tokens::TEXT_MUTED,
        ),
        version_preview_line(
            "Failed versions",
            &snapshot.failed_versions,
            limit,
            crate::theme::tokens::DANGER,
        ),
        version_preview_line(
            "Canceled versions",
            &snapshot.canceled_versions,
            limit,
            crate::theme::tokens::TEXT_MUTED,
        ),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn version_preview_line(
    label: &str,
    versions: &[String],
    limit: usize,
    color: Color,
) -> Option<Element<'static, Message>> {
    (!versions.is_empty()).then(|| {
        text(format!(
            "{label}: {}",
            format_versions_preview(versions, limit)
        ))
        .size(11)
        .color(color)
        .into()
    })
}

fn running_item_progress_basis_points(
    action: BulkRunAction,
    progress: Option<&InstallProgress>,
) -> u16 {
    match action {
        BulkRunAction::Uninstall => 5_000,
        BulkRunAction::Install => match progress {
            Some(InstallProgress::Downloading {
                downloaded_bytes,
                total_bytes,
            }) if *total_bytes > 0 => {
                let scaled =
                    (u128::from(*downloaded_bytes) * 10_000 / u128::from(*total_bytes)).min(10_000);
                u16::try_from(scaled).unwrap_or(10_000)
            }
            Some(InstallProgress::Extracting) => 9_000,
            Some(InstallProgress::Configuring) => 9_700,
            _ => 5_000,
        },
    }
}

fn overall_bulk_progress_basis_points(
    run: &crate::state::BulkRunState,
    install_progress: &std::collections::HashMap<String, InstallProgress>,
) -> u16 {
    if run.items.is_empty() {
        return 0;
    }

    let total = u64::try_from(run.items.len()).unwrap_or(u64::MAX);
    let progressed = run
        .items
        .iter()
        .map(|item| match &item.status {
            BulkItemStatus::Pending => 0_u16,
            BulkItemStatus::Running => {
                running_item_progress_basis_points(item.action, install_progress.get(&item.version))
            }
            BulkItemStatus::Completed | BulkItemStatus::Failed(_) | BulkItemStatus::Canceled => {
                10_000_u16
            }
        })
        .map(u64::from)
        .sum::<u64>();

    let avg = (progressed / total).min(10_000);
    u16::try_from(avg).unwrap_or(10_000)
}

fn format_versions_preview(versions: &[String], limit: usize) -> String {
    let head = versions
        .iter()
        .take(limit)
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join(", ");

    if versions.len() > limit {
        format!("{head}, ... +{}", versions.len() - limit)
    } else {
        head
    }
}

fn network_status_banner(state: &MainState) -> Option<Element<'_, Message>> {
    match state.available_versions.network_status() {
        NetworkStatus::Offline => Some(simple_retry_banner(
            "Could not load available versions".to_string(),
            Message::FetchRemoteVersions,
        )),
        NetworkStatus::Fetching | NetworkStatus::Online => None,
        NetworkStatus::Stale => {
            let age_text = state
                .available_versions
                .disk_cached_at
                .map(|timestamp| format!(" (cached {})", format_relative_time(timestamp)))
                .unwrap_or_default();
            Some(simple_retry_banner(
                format!("Using cached data{age_text} \u{2014} could not refresh from network"),
                Message::FetchRemoteVersions,
            ))
        }
    }
}

fn release_schedule_banner(state: &MainState, has_schedule: bool) -> Option<Element<'_, Message>> {
    fetch_error_banner(
        state.available_versions.schedule_fetch.error.is_some(),
        has_schedule,
        "Release schedule unavailable \u{2014} EOL detection may be inaccurate",
        Message::FetchReleaseSchedule,
    )
}

fn metadata_banner(state: &MainState, has_metadata: bool) -> Option<Element<'_, Message>> {
    fetch_error_banner(
        state.available_versions.metadata_fetch.error.is_some(),
        has_metadata,
        "Version metadata unavailable \u{2014} release details may be incomplete",
        Message::FetchVersionMetadata,
    )
}

fn security_advisories_banner(
    state: &MainState,
    has_security_advisories: bool,
) -> Option<Element<'_, Message>> {
    fetch_error_banner(
        state.available_versions.security_fetch.error.is_some(),
        has_security_advisories,
        "Security advisories unavailable \u{2014} vulnerability warnings may be incomplete",
        Message::FetchSecurityAdvisories,
    )
}

fn fetch_error_banner(
    has_error: bool,
    has_cached_data: bool,
    label: &str,
    retry_message: Message,
) -> Option<Element<'static, Message>> {
    (has_error && !has_cached_data).then(|| simple_retry_banner(label.to_string(), retry_message))
}

fn vulnerability_banner(state: &MainState) -> Option<Element<'_, Message>> {
    let vulnerable_count = state.banner_stats.vulnerable_installed;
    if vulnerable_count == 0 {
        return None;
    }

    let advisory_count = state.banner_stats.vulnerable_advisory;
    let eol_count = state.banner_stats.vulnerable_eol;
    let detail = match (advisory_count, eol_count) {
        (0, eol) => format!("{eol} end-of-life version(s) are considered vulnerable"),
        (advisory, 0) => format!("{advisory} version(s) match known advisories"),
        (advisory, eol) => {
            format!("{advisory} with advisory matches; {eol} marked vulnerable due to EOL")
        }
    };

    let title = format!(
        "{vulnerable_count} installed {} has security vulnerabilities",
        if vulnerable_count == 1 {
            "version"
        } else {
            "versions"
        }
    );

    Some(
        container(
            column![
                text(title).size(13).color(crate::theme::tokens::DANGER),
                text(detail)
                    .size(11)
                    .color(crate::theme::tokens::TEXT_MUTED),
            ]
            .spacing(4),
        )
        .style(styles::card_container)
        .padding([10, 12])
        .width(Length::Fill)
        .into(),
    )
}

fn available_updates_banner(state: &MainState) -> Option<Element<'_, Message>> {
    let update_count = state.banner_stats.updatable_majors;

    if update_count == 0 {
        return None;
    }

    let has_active_ops = !state.operation_queue.active_installs.is_empty()
        || !state.operation_queue.pending.is_empty();
    let label = format!(
        "{} major {} with updates available",
        update_count,
        if update_count == 1 {
            "version"
        } else {
            "versions"
        }
    );

    let button = button(
        row![
            text(label).size(13),
            Space::new().width(Length::Fill),
            text(if has_active_ops {
                "Updating..."
            } else {
                "Update All"
            })
            .size(13),
        ]
        .align_y(Alignment::Center),
    )
    .style(styles::banner_button_info)
    .padding([12, 16])
    .width(Length::Fill);

    Some(if has_active_ops {
        button.into()
    } else {
        button.on_press(Message::RequestBulkUpdateMajors).into()
    })
}

fn eol_cleanup_banner(state: &MainState) -> Option<Element<'_, Message>> {
    if !state.backend.capabilities().supports_uninstall {
        return None;
    }

    let eol_count = state.banner_stats.eol_installed;

    if eol_count == 0 {
        return None;
    }

    Some(
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
    )
}

fn simple_retry_banner(label: String, retry_message: Message) -> Element<'static, Message> {
    button(
        row![
            text(label).size(13),
            Space::new().width(Length::Fill),
            text("Retry").size(13),
        ]
        .align_y(Alignment::Center),
    )
    .on_press(retry_message)
    .style(styles::banner_button_warning)
    .padding([12, 16])
    .width(Length::Fill)
    .into()
}

fn format_relative_time(timestamp: DateTime<Utc>) -> String {
    let delta = Utc::now().signed_duration_since(timestamp);
    let minutes = delta.num_minutes();
    if minutes < 1 {
        "just now".to_string()
    } else if minutes < 60 {
        format!("{minutes}m ago")
    } else {
        let hours = delta.num_hours();
        if hours < 24 {
            format!("{hours}h ago")
        } else {
            let days = delta.num_days();
            format!("{days}d ago")
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::Arc;

    use versi_backend::InstallProgress;
    use versi_backend::{BackendDetection, BackendProvider};
    use versi_platform::EnvironmentId;

    use super::{
        bulk_operation_progress_banner, contextual_banners, format_versions_preview,
        metadata_banner, overall_bulk_progress_basis_points, security_advisories_banner,
        vulnerability_banner,
    };
    use crate::backend_kind::BackendKind;
    use crate::error::AppError;
    use crate::state::{
        BulkItemStatus, BulkRunAction, BulkRunItem, BulkRunKind, BulkRunState, EnvironmentState,
        MainState,
    };

    fn main_state_for_banners() -> MainState {
        let provider: Arc<dyn BackendProvider> = Arc::new(versi_fnm::FnmProvider::new());
        let backend = provider.create_manager(&BackendDetection {
            found: true,
            path: Some(PathBuf::from("fnm")),
            version: None,
            in_path: true,
            data_dir: None,
        });

        let mut environment = EnvironmentState::new(EnvironmentId::Native, BackendKind::Fnm, None);
        environment.loading = false;

        MainState::new_with_environments(backend, vec![environment], BackendKind::Fnm)
    }

    #[test]
    fn metadata_banner_shows_when_error_exists_without_cached_metadata() {
        let mut state = main_state_for_banners();
        state.available_versions.metadata_fetch.error = Some(AppError::version_fetch_failed(
            "Version metadata",
            "network timeout",
        ));

        assert!(metadata_banner(&state, false).is_some());
        assert!(contextual_banners(&state).is_some());
    }

    #[test]
    fn metadata_banner_hides_when_metadata_exists() {
        let mut state = main_state_for_banners();
        state.available_versions.metadata = Some(std::collections::HashMap::new());
        state.available_versions.metadata_fetch.error = Some(AppError::version_fetch_failed(
            "Version metadata",
            "network timeout",
        ));

        assert!(metadata_banner(&state, true).is_none());
    }

    #[test]
    fn metadata_banner_hides_without_error() {
        let state = main_state_for_banners();
        assert!(metadata_banner(&state, false).is_none());
    }

    #[test]
    fn security_advisories_banner_shows_when_error_exists_without_cached_data() {
        let mut state = main_state_for_banners();
        state.available_versions.security_fetch.error = Some(AppError::version_fetch_failed(
            "Security advisories",
            "network timeout",
        ));

        assert!(security_advisories_banner(&state, false).is_some());
    }

    #[test]
    fn security_advisories_banner_hides_when_cached_data_exists() {
        let mut state = main_state_for_banners();
        state.available_versions.security_fetch.error = Some(AppError::version_fetch_failed(
            "Security advisories",
            "network timeout",
        ));
        state.available_versions.security_advisories = Some(std::collections::HashMap::new());

        assert!(security_advisories_banner(&state, true).is_none());
    }

    #[test]
    fn vulnerability_banner_shows_with_non_zero_count() {
        let mut state = main_state_for_banners();
        state.banner_stats.vulnerable_installed = 2;
        state.banner_stats.vulnerable_advisory = 1;
        state.banner_stats.vulnerable_eol = 1;

        assert!(vulnerability_banner(&state).is_some());
    }

    #[test]
    fn bulk_progress_banner_shows_for_active_bulk_run() {
        let mut state = main_state_for_banners();
        state.bulk_run = Some(BulkRunState::new(
            BulkRunKind::UpdateMajors,
            vec![
                BulkRunItem {
                    version: "v22.1.0".to_string(),
                    action: BulkRunAction::Install,
                    status: BulkItemStatus::Running,
                },
                BulkRunItem {
                    version: "v20.11.1".to_string(),
                    action: BulkRunAction::Install,
                    status: BulkItemStatus::Pending,
                },
            ],
        ));
        state.install_progress.insert(
            "v22.1.0".to_string(),
            InstallProgress::Downloading {
                downloaded_bytes: 50,
                total_bytes: 100,
            },
        );

        assert!(bulk_operation_progress_banner(&state).is_some());
    }

    #[test]
    fn format_versions_preview_limits_output() {
        let versions = vec![
            "v22.1.0".to_string(),
            "v20.11.1".to_string(),
            "v18.20.0".to_string(),
        ];

        assert_eq!(
            format_versions_preview(&versions, 2),
            "v22.1.0, v20.11.1, ... +1"
        );
        assert_eq!(
            format_versions_preview(&versions, 5),
            "v22.1.0, v20.11.1, v18.20.0"
        );
    }

    #[test]
    fn overall_bulk_progress_counts_running_and_finished_items() {
        let run = BulkRunState::new(
            BulkRunKind::UpdateMajors,
            vec![
                BulkRunItem {
                    version: "v22.1.0".to_string(),
                    action: BulkRunAction::Install,
                    status: BulkItemStatus::Running,
                },
                BulkRunItem {
                    version: "v20.11.1".to_string(),
                    action: BulkRunAction::Install,
                    status: BulkItemStatus::Completed,
                },
                BulkRunItem {
                    version: "v18.20.0".to_string(),
                    action: BulkRunAction::Install,
                    status: BulkItemStatus::Pending,
                },
            ],
        );

        let mut install_progress = std::collections::HashMap::new();
        install_progress.insert(
            "v22.1.0".to_string(),
            InstallProgress::Downloading {
                downloaded_bytes: 50,
                total_bytes: 100,
            },
        );

        let progress = overall_bulk_progress_basis_points(&run, &install_progress);
        assert_eq!(progress, 5_000);
    }

    mod format_relative_time_tests {
        use chrono::{Duration, Utc};

        use super::super::format_relative_time;

        #[test]
        fn less_than_60_seconds() {
            let timestamp = Utc::now() - Duration::seconds(30);
            assert_eq!(format_relative_time(timestamp), "just now");
        }

        #[test]
        fn zero_seconds_ago() {
            assert_eq!(format_relative_time(Utc::now()), "just now");
        }

        #[test]
        fn boundary_59_seconds() {
            let timestamp = Utc::now() - Duration::seconds(59);
            assert_eq!(format_relative_time(timestamp), "just now");
        }

        #[test]
        fn exactly_60_seconds() {
            let timestamp = Utc::now() - Duration::seconds(60);
            assert_eq!(format_relative_time(timestamp), "1m ago");
        }

        #[test]
        fn five_minutes() {
            let timestamp = Utc::now() - Duration::minutes(5);
            assert_eq!(format_relative_time(timestamp), "5m ago");
        }

        #[test]
        fn boundary_59_minutes() {
            let timestamp = Utc::now() - Duration::seconds(3599);
            assert_eq!(format_relative_time(timestamp), "59m ago");
        }

        #[test]
        fn exactly_one_hour() {
            let timestamp = Utc::now() - Duration::hours(1);
            assert_eq!(format_relative_time(timestamp), "1h ago");
        }

        #[test]
        fn multiple_hours() {
            let timestamp = Utc::now() - Duration::hours(12);
            assert_eq!(format_relative_time(timestamp), "12h ago");
        }

        #[test]
        fn boundary_23_hours() {
            let timestamp = Utc::now() - Duration::hours(23);
            assert_eq!(format_relative_time(timestamp), "23h ago");
        }

        #[test]
        fn exactly_one_day() {
            let timestamp = Utc::now() - Duration::days(1);
            assert_eq!(format_relative_time(timestamp), "1d ago");
        }

        #[test]
        fn multiple_days() {
            let timestamp = Utc::now() - Duration::days(7);
            assert_eq!(format_relative_time(timestamp), "7d ago");
        }
    }
}
