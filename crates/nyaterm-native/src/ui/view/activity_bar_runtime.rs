use super::*;
use gpui::{FontWeight, Render, Window, rgba};
use nyaterm_domain::truncate_preview;

#[derive(Clone, Debug)]
pub(in crate::ui::view) struct ActivityBarDragPayload {
    pub entry_id: String,
    pub zone: ActivityBarZone,
    pub index: usize,
    pub label: String,
}

pub(in crate::ui::view) struct ActivityBarDragPreview {
    payload: ActivityBarDragPayload,
    position: gpui::Point<gpui::Pixels>,
}

impl ActivityBarDragPreview {
    pub(in crate::ui::view) fn new(
        payload: ActivityBarDragPayload,
        position: gpui::Point<gpui::Pixels>,
    ) -> Self {
        Self { payload, position }
    }
}

impl Render for ActivityBarDragPreview {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .pl(self.position.x - px(72.))
            .pt(self.position.y - px(18.))
            .child(
                div()
                    .w(px(144.))
                    .h(px(36.))
                    .px_3()
                    .flex()
                    .items_center()
                    .gap_2()
                    .rounded_sm()
                    .border_1()
                    .border_color(rgb(0x334155))
                    .bg(rgba(0x151b24dd))
                    .shadow_lg()
                    .child(
                        div()
                            .text_size(px(10.))
                            .font_weight(FontWeight(800.))
                            .text_color(rgb(0x93c5fd))
                            .child("ACT"),
                    )
                    .child(
                        div()
                            .min_w_0()
                            .flex_1()
                            .text_size(px(12.))
                            .text_color(rgb(0xc9d1d9))
                            .child(truncate_preview(&self.payload.label, 16)),
                    ),
            )
    }
}

impl NyaTermApp {
    pub(in crate::ui::view) fn activity_entries_for_zone(
        &self,
        zone: ActivityBarZone,
    ) -> Vec<ActivityBarEntry> {
        self.activity_bar_layout
            .zone(zone)
            .iter()
            .filter_map(|id| ActivityBarEntry::from_persistence_id(id))
            .collect()
    }

    pub(in crate::ui::view) fn sync_activity_layout_to_settings(&mut self) {
        self.settings.ui_activity_bar_left_top = self.activity_bar_layout.left_top.clone();
        self.settings.ui_activity_bar_left_bottom = self.activity_bar_layout.left_bottom.clone();
        self.settings.ui_activity_bar_right_top = self.activity_bar_layout.right_top.clone();
        self.settings.ui_activity_bar_right_bottom = self.activity_bar_layout.right_bottom.clone();
        self.settings.ui_activity_bar_show_labels = self.activity_bar_layout.show_labels;
    }

    pub(in crate::ui::view) fn apply_activity_layout_from_settings(&mut self) {
        self.activity_bar_layout = ActivityBarLayoutState {
            left_top: self.settings.ui_activity_bar_left_top.clone(),
            left_bottom: self.settings.ui_activity_bar_left_bottom.clone(),
            right_top: self.settings.ui_activity_bar_right_top.clone(),
            right_bottom: self.settings.ui_activity_bar_right_bottom.clone(),
            show_labels: self.settings.ui_activity_bar_show_labels,
        };
        self.normalize_activity_bar_layout();
    }

    pub(in crate::ui::view) fn normalize_activity_bar_layout(&mut self) {
        let mut seen = std::collections::HashSet::new();
        for zone in ActivityBarZone::all() {
            let mut next = Vec::new();
            for id in self.activity_bar_layout.zone(zone).iter().cloned() {
                if id == "fileTransfer" {
                    continue;
                }
                if seen.insert(id.clone()) {
                    next.push(id);
                }
            }
            *self.activity_bar_layout.zone_mut(zone) = next;
        }
        // Ensure defaults exist if zones empty for critical items.
        let defaults = ActivityBarLayoutState::default();
        for zone in ActivityBarZone::all() {
            if self.activity_bar_layout.zone(zone).is_empty() {
                *self.activity_bar_layout.zone_mut(zone) = defaults.zone(zone).to_vec();
            }
        }
    }

    pub(in crate::ui::view) fn toggle_activity_bar_labels(&mut self, cx: &mut Context<Self>) {
        self.activity_bar_layout.show_labels = !self.activity_bar_layout.show_labels;
        self.terminal_status = if self.activity_bar_layout.show_labels {
            "activity labels shown".to_string()
        } else {
            "activity labels hidden".to_string()
        };
        self.persist_ui_layout();
        cx.notify();
    }

    pub(in crate::ui::view) fn open_activity_bar_context_menu(
        &mut self,
        entry_id: String,
        zone: ActivityBarZone,
        index: usize,
        cx: &mut Context<Self>,
    ) {
        self.title_menu_open = None;
        self.activity_bar_context_menu = Some(ActivityBarContextMenuState {
            entry_id,
            zone,
            index,
        });
        cx.notify();
    }

    pub(in crate::ui::view) fn close_activity_bar_context_menu(&mut self, cx: &mut Context<Self>) {
        self.activity_bar_context_menu = None;
        cx.notify();
    }

    pub(in crate::ui::view) fn move_activity_entry(
        &mut self,
        entry_id: String,
        target_zone: ActivityBarZone,
        target_index: Option<usize>,
        cx: &mut Context<Self>,
    ) {
        let Some((source_zone, source_index)) = self.activity_bar_layout.find_entry(&entry_id)
        else {
            self.terminal_status = "activity item not found".to_string();
            cx.notify();
            return;
        };

        // Same entry dropped on itself — no-op.
        if source_zone == target_zone {
            let len = self.activity_bar_layout.zone(target_zone).len();
            let mut insert_at = target_index.unwrap_or(len);
            if source_index < insert_at {
                insert_at = insert_at.saturating_sub(1);
            }
            insert_at = insert_at.min(len.saturating_sub(1));
            if insert_at == source_index {
                self.activity_bar_context_menu = None;
                cx.notify();
                return;
            }
        }

        // Remove from source.
        let removed = self
            .activity_bar_layout
            .zone_mut(source_zone)
            .remove(source_index);
        let mut insert_at = target_index.unwrap_or_else(|| self.activity_bar_layout.zone(target_zone).len());
        if source_zone == target_zone && source_index < insert_at {
            insert_at = insert_at.saturating_sub(1);
        }
        insert_at = insert_at.min(self.activity_bar_layout.zone(target_zone).len());
        self.activity_bar_layout
            .zone_mut(target_zone)
            .insert(insert_at, removed);

        // Mirror Tauri: when an open panel moves across left/right, clear that side's open state.
        let source_side = Self::activity_zone_side(source_zone);
        let target_side = Self::activity_zone_side(target_zone);
        if source_side != target_side {
            self.clear_activity_entry_from_side(&entry_id, source_side);
        }

        self.activity_bar_context_menu = None;
        self.terminal_status = format!(
            "moved {} to {}",
            entry_id,
            target_zone.label().to_lowercase()
        );
        self.persist_ui_layout();
        cx.notify();
    }

    fn activity_zone_side(zone: ActivityBarZone) -> PanelSide {
        match zone {
            ActivityBarZone::LeftTop | ActivityBarZone::LeftBottom => PanelSide::Left,
            ActivityBarZone::RightTop | ActivityBarZone::RightBottom => PanelSide::Right,
        }
    }

    fn clear_activity_entry_from_side(&mut self, entry_id: &str, side: PanelSide) {
        match side {
            PanelSide::Left => {
                self.left_open_panels.retain(|id| id != entry_id);
                if self
                    .active_left_panel
                    .is_some_and(|item| item.persistence_id() == entry_id)
                {
                    self.active_left_panel = None;
                }
                if self.left_open_panels.is_empty() && self.active_left_panel.is_none() {
                    self.left_sidebar_collapsed = true;
                }
            }
            PanelSide::Right => {
                self.right_open_panels.retain(|id| id != entry_id);
                if self
                    .active_right_panel
                    .is_some_and(|item| item.persistence_id() == entry_id)
                {
                    self.active_right_panel = None;
                }
                if self.right_open_panels.is_empty() && self.active_right_panel.is_none() {
                    self.right_inspector_collapsed = true;
                }
            }
        }
    }

    pub(in crate::ui::view) fn reorder_activity_entry(
        &mut self,
        entry_id: String,
        delta: isize,
        cx: &mut Context<Self>,
    ) {
        let Some((zone, index)) = self.activity_bar_layout.find_entry(&entry_id) else {
            return;
        };
        let len = self.activity_bar_layout.zone(zone).len();
        if len == 0 {
            return;
        }
        let next = if delta < 0 {
            index.saturating_sub(1)
        } else {
            (index + 1).min(len.saturating_sub(1))
        };
        if next == index {
            return;
        }
        self.activity_bar_layout.zone_mut(zone).swap(index, next);
        self.activity_bar_context_menu = None;
        self.terminal_status = format!("reordered {entry_id}");
        self.persist_ui_layout();
        cx.notify();
    }

    pub(in crate::ui::view) fn activate_activity_entry(
        &mut self,
        entry: ActivityBarEntry,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match entry {
            ActivityBarEntry::Panel(NavItem::Settings) => self.open_page(NavItem::Settings, cx),
            ActivityBarEntry::Panel(item) => self.open_panel(item, cx),
            ActivityBarEntry::QuickCommands => {
                self.bottom_panel = if self.bottom_panel == BottomPanelMode::QuickCommands {
                    BottomPanelMode::Hidden
                } else {
                    BottomPanelMode::QuickCommands
                };
                cx.notify();
            }
            ActivityBarEntry::CommandSend => {
                self.bottom_panel = if self.bottom_panel == BottomPanelMode::CommandSend {
                    BottomPanelMode::Hidden
                } else {
                    BottomPanelMode::CommandSend
                };
                cx.notify();
            }
            ActivityBarEntry::Recording => self.open_panel(NavItem::Recording, cx),
            ActivityBarEntry::Lock => self.lock_app(window, cx),
        }
    }

    pub(in crate::ui::view) fn activity_entry_selected(&self, entry: ActivityBarEntry) -> bool {
        match entry {
            ActivityBarEntry::Panel(NavItem::Settings) => self.main_mode == MainMode::Page,
            ActivityBarEntry::Panel(item) if item.is_left_panel() => {
                if self.panel_multi_open {
                    let id = item.persistence_id();
                    self.side_open_panel_ids(PanelSide::Left)
                        .iter()
                        .any(|open| open == id)
                        || self.side_overlay_panel(PanelSide::Left) == Some(item)
                        || self.active_left_panel == Some(item)
                } else {
                    self.current_left_panel() == Some(item)
                }
            }
            ActivityBarEntry::Panel(item) if item.is_right_panel() => {
                if self.panel_multi_open {
                    let id = item.persistence_id();
                    self.side_open_panel_ids(PanelSide::Right)
                        .iter()
                        .any(|open| open == id)
                        || self.side_overlay_panel(PanelSide::Right) == Some(item)
                        || self.active_right_panel == Some(item)
                } else {
                    self.current_right_panel() == Some(item)
                }
            }
            ActivityBarEntry::Panel(_) => false,
            ActivityBarEntry::QuickCommands => self.bottom_panel == BottomPanelMode::QuickCommands,
            ActivityBarEntry::CommandSend => self.bottom_panel == BottomPanelMode::CommandSend,
            ActivityBarEntry::Recording => {
                self.current_right_panel() == Some(NavItem::Recording)
                    || !self.recording_manager.list_recording_sessions().is_empty()
            }
            ActivityBarEntry::Lock => self.is_locked,
        }
    }
}
