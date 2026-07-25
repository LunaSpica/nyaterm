use std::collections::HashSet;
use std::time::{Duration, Instant};

use nyaterm_core::Group;

use crate::features::{ConnectionDragKind, ConnectionDropPosition, ConnectionDropTarget};
use crate::models::{
    ConnectionContextMenuState, ConnectionGroupContextMenuState, ConnectionSortMode,
};

pub(super) fn remove_connection_list_references(
    selected_ids: &mut HashSet<String>,
    last_selected_id: &mut Option<String>,
    hovered_connection_id: &mut Option<String>,
    hover_pending: &mut Option<(String, Instant)>,
    context_menu: &mut Option<ConnectionContextMenuState>,
    drop_target: &mut Option<ConnectionDropTarget>,
    connection_id: &str,
) {
    selected_ids.remove(connection_id);
    if last_selected_id.as_deref() == Some(connection_id) {
        *last_selected_id = None;
    }
    if hovered_connection_id.as_deref() == Some(connection_id) {
        *hovered_connection_id = None;
    }
    if hover_pending
        .as_ref()
        .is_some_and(|(pending_id, _)| pending_id == connection_id)
    {
        *hover_pending = None;
    }
    if context_menu
        .as_ref()
        .is_some_and(|menu| menu.connection_id == connection_id)
    {
        *context_menu = None;
    }
    if drop_target.as_ref().is_some_and(|target| {
        target.kind == ConnectionDragKind::Connection && target.id.as_deref() == Some(connection_id)
    }) {
        *drop_target = None;
    }
}

pub(super) fn remove_group_list_references(
    expanded_group_ids: &mut HashSet<String>,
    hovered_group_id: &mut Option<String>,
    group_context_menu: &mut Option<ConnectionGroupContextMenuState>,
    drop_target: &mut Option<ConnectionDropTarget>,
    group_id: &str,
) {
    expanded_group_ids.remove(group_id);
    if hovered_group_id.as_deref() == Some(group_id) {
        *hovered_group_id = None;
    }
    if group_context_menu
        .as_ref()
        .is_some_and(|menu| menu.group_id == group_id)
    {
        *group_context_menu = None;
    }
    if drop_target.as_ref().is_some_and(|target| {
        target.kind == ConnectionDragKind::Group && target.id.as_deref() == Some(group_id)
    }) {
        *drop_target = None;
    }
}

pub(super) fn retain_loaded_connection_list_references(
    selected_ids: &mut HashSet<String>,
    last_selected_id: &mut Option<String>,
    hovered_connection_id: &mut Option<String>,
    hover_pending: &mut Option<(String, Instant)>,
    context_menu: &mut Option<ConnectionContextMenuState>,
    expanded_group_ids: &mut HashSet<String>,
    hovered_group_id: &mut Option<String>,
    group_context_menu: &mut Option<ConnectionGroupContextMenuState>,
    drop_target: &mut Option<ConnectionDropTarget>,
    connection_ids: &HashSet<String>,
    group_ids: &HashSet<String>,
) {
    selected_ids.retain(|id| connection_ids.contains(id));
    if last_selected_id
        .as_ref()
        .is_some_and(|id| !connection_ids.contains(id))
    {
        *last_selected_id = None;
    }
    if hovered_connection_id
        .as_ref()
        .is_some_and(|id| !connection_ids.contains(id))
    {
        *hovered_connection_id = None;
    }
    if hover_pending
        .as_ref()
        .is_some_and(|(id, _)| !connection_ids.contains(id))
    {
        *hover_pending = None;
    }
    if context_menu
        .as_ref()
        .is_some_and(|menu| !connection_ids.contains(&menu.connection_id))
    {
        *context_menu = None;
    }

    expanded_group_ids.retain(|id| group_ids.contains(id));
    if hovered_group_id
        .as_ref()
        .is_some_and(|id| !group_ids.contains(id))
    {
        *hovered_group_id = None;
    }
    if group_context_menu
        .as_ref()
        .is_some_and(|menu| !group_ids.contains(&menu.group_id))
    {
        *group_context_menu = None;
    }
    if drop_target
        .as_ref()
        .is_some_and(|target| match target.kind {
            ConnectionDragKind::Connection => target
                .id
                .as_ref()
                .is_some_and(|id| !connection_ids.contains(id)),
            ConnectionDragKind::Group => {
                target.id.as_ref().is_some_and(|id| !group_ids.contains(id))
            }
        })
    {
        *drop_target = None;
    }
}

pub(super) fn clear_selected_connection_ids(
    selected_ids: &mut HashSet<String>,
    last_selected_id: &mut Option<String>,
) {
    selected_ids.clear();
    *last_selected_id = None;
}

pub(super) fn close_connection_more_menu(more_menu_open: &mut bool) -> bool {
    let was_open = *more_menu_open;
    *more_menu_open = false;
    was_open
}

pub(super) fn apply_connection_search_key(
    search_draft: &mut String,
    key: &str,
    input: Option<&str>,
) -> bool {
    match key {
        "escape" => clear_connection_search(search_draft),
        "backspace" => {
            search_draft.pop();
            true
        }
        _ => {
            let Some(input) = input.filter(|input| !input.is_empty()) else {
                return false;
            };
            search_draft.push_str(input);
            true
        }
    }
}

pub(super) fn clear_connection_search(search_draft: &mut String) -> bool {
    search_draft.clear();
    true
}

pub(super) fn cycle_connection_sort_mode(sort_mode: &mut ConnectionSortMode) -> ConnectionSortMode {
    *sort_mode = sort_mode.next();
    *sort_mode
}

pub(super) fn dismiss_connection_hover(
    hovered_connection_id: &mut Option<String>,
    hover_pending: &mut Option<(String, Instant)>,
) -> bool {
    let had_hovered = hovered_connection_id.take().is_some();
    let had_pending = hover_pending.take().is_some();
    had_hovered || had_pending
}

pub(super) fn connection_hover_intent_ready(
    started_at: Instant,
    now: Instant,
    delay: Duration,
) -> bool {
    now.saturating_duration_since(started_at) >= delay
}

pub(super) fn poll_connection_hover_intent(
    hovered_connection_id: &mut Option<String>,
    hover_pending: &mut Option<(String, Instant)>,
    now: Instant,
    delay: Duration,
) -> bool {
    let Some((connection_id, started_at)) = hover_pending.clone() else {
        return false;
    };
    if !connection_hover_intent_ready(started_at, now, delay) {
        return false;
    }
    *hover_pending = None;
    if hovered_connection_id.as_deref() == Some(connection_id.as_str()) {
        return false;
    }
    *hovered_connection_id = Some(connection_id);
    true
}

pub(super) fn begin_connection_hover_intent(
    hovered_connection_id: &Option<String>,
    hover_pending: &mut Option<(String, Instant)>,
    connection_id: String,
    started_at: Instant,
) -> bool {
    if hovered_connection_id.as_deref() == Some(connection_id.as_str())
        || hover_pending
            .as_ref()
            .is_some_and(|(pending_id, _)| pending_id == &connection_id)
    {
        return false;
    }
    *hover_pending = Some((connection_id, started_at));
    true
}

pub(super) fn clear_connection_hover_intent(
    hovered_connection_id: &mut Option<String>,
    hover_pending: &mut Option<(String, Instant)>,
    connection_id: &str,
) -> bool {
    let mut changed = false;
    if hover_pending
        .as_ref()
        .is_some_and(|(pending_id, _)| pending_id == connection_id)
    {
        *hover_pending = None;
    }
    if hovered_connection_id.as_deref() == Some(connection_id) {
        *hovered_connection_id = None;
        changed = true;
    }
    changed
}

pub(super) fn set_connection_group_hover(
    hovered_group_id: &mut Option<String>,
    group_id: String,
    hovered: bool,
) -> bool {
    if hovered {
        if hovered_group_id.as_deref() == Some(group_id.as_str()) {
            return false;
        }
        *hovered_group_id = Some(group_id);
        return true;
    }
    if hovered_group_id.as_deref() == Some(group_id.as_str()) {
        *hovered_group_id = None;
        return true;
    }
    false
}

pub(super) fn set_connection_drop_target_if_changed(
    drop_target: &mut Option<ConnectionDropTarget>,
    target: ConnectionDropTarget,
) -> bool {
    if drop_target.as_ref() == Some(&target) {
        return false;
    }
    *drop_target = Some(target);
    true
}

pub(super) fn connection_drop_position_for_target(
    drop_target: &Option<ConnectionDropTarget>,
    target_id: &str,
    fallback: ConnectionDropPosition,
) -> ConnectionDropPosition {
    drop_target
        .as_ref()
        .filter(|target| target.id.as_deref() == Some(target_id))
        .map(|target| target.position)
        .unwrap_or(fallback)
}

pub(super) fn clear_connection_list_runtime_state(
    selected_ids: &mut HashSet<String>,
    last_selected_id: &mut Option<String>,
    expanded_group_ids: &mut HashSet<String>,
    context_menu: &mut Option<ConnectionContextMenuState>,
    group_context_menu: &mut Option<ConnectionGroupContextMenuState>,
    hovered_connection_id: &mut Option<String>,
    hover_pending: &mut Option<(String, Instant)>,
    drop_target: &mut Option<ConnectionDropTarget>,
    hovered_group_id: &mut Option<String>,
) {
    clear_selected_connection_ids(selected_ids, last_selected_id);
    expanded_group_ids.clear();
    *context_menu = None;
    *group_context_menu = None;
    *hovered_connection_id = None;
    *hover_pending = None;
    *drop_target = None;
    *hovered_group_id = None;
}

pub(super) fn select_connection_ids(
    selected_ids: &mut HashSet<String>,
    last_selected_id: &mut Option<String>,
    connection_id: String,
    visible_ids: &[String],
    additive: bool,
    range: bool,
) -> usize {
    if range {
        let anchor = last_selected_id
            .clone()
            .unwrap_or_else(|| connection_id.clone());
        let mut next = if additive {
            selected_ids.clone()
        } else {
            HashSet::new()
        };
        if let (Some(start), Some(end)) = (
            visible_ids.iter().position(|id| id == &anchor),
            visible_ids.iter().position(|id| id == &connection_id),
        ) {
            let (lo, hi) = if start <= end {
                (start, end)
            } else {
                (end, start)
            };
            for id in &visible_ids[lo..=hi] {
                next.insert(id.clone());
            }
        } else {
            next.insert(connection_id.clone());
        }
        *selected_ids = next;
    } else if additive {
        if selected_ids.contains(&connection_id) {
            selected_ids.remove(&connection_id);
        } else {
            selected_ids.insert(connection_id.clone());
        }
    } else {
        selected_ids.clear();
        selected_ids.insert(connection_id.clone());
    }
    *last_selected_id = Some(connection_id);
    selected_ids.len()
}

pub(super) fn expanded_group_ids(groups: &[Group]) -> HashSet<String> {
    groups.iter().map(|group| group.id.clone()).collect()
}
