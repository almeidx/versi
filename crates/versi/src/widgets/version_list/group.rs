use std::collections::HashSet;

use iced::widget::{Space, button, column, container, row, text};
use iced::{Alignment, Element, Length};

use versi_backend::{InstalledVersion, NodeVersion, VersionGroup};

use crate::icon;
use crate::message::Message;
use crate::state::SearchFilter;
use crate::theme::styles;

use super::VersionListContext;
use super::filter_version;
use super::item::version_item_view;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HeaderBadgeKind {
    Lts,
    Eol,
    Default,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GroupExpansion {
    Expanded,
    Collapsed,
}

fn group_header_badges(
    has_lts: bool,
    has_default: bool,
    is_eol: bool,
    expansion: GroupExpansion,
) -> Vec<HeaderBadgeKind> {
    let mut badges = Vec::new();
    if has_lts {
        badges.push(HeaderBadgeKind::Lts);
    }
    if is_eol {
        badges.push(HeaderBadgeKind::Eol);
    }
    if has_default && matches!(expansion, GroupExpansion::Collapsed) {
        badges.push(HeaderBadgeKind::Default);
    }
    badges
}

fn show_bulk_actions(is_expanded: bool, version_count: usize, supports_uninstall: bool) -> bool {
    supports_uninstall && is_expanded && version_count > 1
}

pub(super) fn version_group_view<'a>(
    group: &'a VersionGroup,
    default: Option<&'a versi_backend::NodeVersion>,
    search_query: &'a str,
    query_lower: &str,
    update_available: Option<NodeVersion>,
    active_filters: &'a HashSet<SearchFilter>,
    ctx: &VersionListContext<'a>,
) -> Element<'a, Message> {
    let has_lts = group.versions.iter().any(|v| v.lts_codename.is_some());
    let has_default = group
        .versions
        .iter()
        .any(|v| default.is_some_and(|d| d == &v.version));
    let is_eol = ctx.schedule.is_some_and(|s| !s.is_active(group.major));

    let header_button = button(group_header_row(group, has_lts, has_default, is_eol))
        .on_press(Message::VersionGroupToggled { major: group.major })
        .style(|theme, status| {
            let mut style = iced::widget::button::text(theme, status);
            style.text_color = theme.palette().text;
            style
        })
        .padding([8, 12]);

    let header: Element<Message> = row![
        header_button,
        Space::new().width(Length::Fill),
        group_header_actions(group, update_available, ctx.supports_uninstall),
    ]
    .align_y(Alignment::Center)
    .into();

    if group.is_expanded {
        expanded_group_view(
            group,
            default,
            search_query,
            query_lower,
            active_filters,
            ctx,
            header,
        )
    } else {
        container(header)
            .style(styles::card_container)
            .padding(12)
            .width(Length::Fill)
            .into()
    }
}

fn group_header_row(
    group: &VersionGroup,
    has_lts: bool,
    has_default: bool,
    is_eol: bool,
) -> iced::widget::Row<'_, Message> {
    let chevron = if group.is_expanded {
        icon::chevron_down(12.0)
    } else {
        icon::chevron_right(12.0)
    };

    let mut header_row = row![
        chevron,
        text(format!("Node {}.x", group.major)).size(16),
        text(format!("({} installed)", group.versions.len())).size(12),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let expansion = if group.is_expanded {
        GroupExpansion::Expanded
    } else {
        GroupExpansion::Collapsed
    };
    for badge in group_header_badges(has_lts, has_default, is_eol, expansion) {
        header_row = match badge {
            HeaderBadgeKind::Lts => header_row.push(
                container(text("LTS").size(10))
                    .padding([2, 6])
                    .style(styles::badge_lts),
            ),
            HeaderBadgeKind::Eol => header_row.push(
                container(text("End-of-Life").size(10))
                    .padding([2, 6])
                    .style(styles::badge_eol),
            ),
            HeaderBadgeKind::Default => header_row.push(
                container(text("default").size(10))
                    .padding([2, 6])
                    .style(styles::badge_default),
            ),
        };
    }
    header_row
}

fn group_header_actions(
    group: &VersionGroup,
    update_available: Option<NodeVersion>,
    supports_uninstall: bool,
) -> Element<'_, Message> {
    let mut actions = row![].spacing(8).align_y(Alignment::Center);

    if let Some(new_version) = update_available {
        actions = actions.push(
            button(container(text(format!("{new_version} available")).size(10)).padding([2, 6]))
                .on_press(Message::StartInstall(new_version))
                .style(styles::update_badge_button)
                .padding([0, 4]),
        );
    }

    if show_bulk_actions(group.is_expanded, group.versions.len(), supports_uninstall) {
        actions = actions.push(
            button(text("Keep Latest").size(10))
                .on_press(Message::RequestBulkUninstallMajorExceptLatest { major: group.major })
                .style(styles::ghost_button)
                .padding([4, 8]),
        );
        actions = actions.push(
            button(text("Uninstall All").size(10))
                .on_press(Message::RequestBulkUninstallMajor { major: group.major })
                .style(styles::ghost_button)
                .padding([4, 8]),
        );
    }

    actions.into()
}

fn expanded_group_view<'a>(
    group: &'a VersionGroup,
    default: Option<&'a versi_backend::NodeVersion>,
    search_query: &'a str,
    query_lower: &str,
    active_filters: &'a HashSet<SearchFilter>,
    ctx: &VersionListContext<'a>,
    header: Element<'a, Message>,
) -> Element<'a, Message> {
    let mut version_text = String::with_capacity(16);
    let filtered_versions: Vec<&InstalledVersion> = group
        .versions
        .iter()
        .filter(|v| {
            filter_version(
                v,
                search_query,
                query_lower,
                active_filters,
                ctx.schedule,
                &mut version_text,
            )
        })
        .collect();

    let items: Vec<Element<Message>> = filtered_versions
        .iter()
        .map(|version| version_item_view(version, default, ctx))
        .collect();

    container(
        column![
            header,
            container(column(items).spacing(2)).padding(iced::Padding {
                top: 0.0,
                right: 0.0,
                bottom: 0.0,
                left: crate::theme::tokens::GROUP_INDENT,
            }),
        ]
        .spacing(4),
    )
    .style(styles::card_container)
    .padding(12)
    .into()
}

#[cfg(test)]
mod tests {
    use super::{GroupExpansion, HeaderBadgeKind, group_header_badges, show_bulk_actions};

    #[test]
    fn group_header_badges_include_default_only_when_collapsed() {
        assert_eq!(
            group_header_badges(true, true, false, GroupExpansion::Collapsed),
            vec![HeaderBadgeKind::Lts, HeaderBadgeKind::Default]
        );
        assert_eq!(
            group_header_badges(true, true, false, GroupExpansion::Expanded),
            vec![HeaderBadgeKind::Lts]
        );
    }

    #[test]
    fn group_header_badges_keep_lts_eol_order() {
        assert_eq!(
            group_header_badges(true, false, true, GroupExpansion::Collapsed),
            vec![HeaderBadgeKind::Lts, HeaderBadgeKind::Eol]
        );
        assert_eq!(
            group_header_badges(false, false, false, GroupExpansion::Collapsed),
            vec![]
        );
    }

    #[test]
    fn bulk_actions_require_expanded_group_with_multiple_versions() {
        assert!(!show_bulk_actions(false, 5, true));
        assert!(!show_bulk_actions(true, 1, true));
        assert!(!show_bulk_actions(true, 2, false));
        assert!(show_bulk_actions(true, 2, true));
    }
}
