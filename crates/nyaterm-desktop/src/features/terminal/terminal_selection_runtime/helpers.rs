use super::*;
use std::ops::Range;

fn terminal_input_selection() -> UTF16Selection {
    UTF16Selection {
        range: 0..0,
        reversed: false,
    }
}

fn terminal_visible_surface_bounds(
    bounds: Bounds<Pixels>,
    content_mask: Bounds<Pixels>,
) -> Option<Bounds<Pixels>> {
    let visible = content_mask.intersect(&bounds);
    (visible.size.width > px(0.) && visible.size.height > px(0.)).then_some(visible)
}

/// Invisible canvas child that records the terminal output bounds for selection hit-testing.
pub(in crate::features) fn terminal_bounds_tracker(
    entity: gpui::Entity<NyaTermApp>,
    session_id: Option<String>,
    active: bool,
) -> impl IntoElement {
    let input_entity = entity.clone();
    let bounds_entity = input_entity.clone();
    let tracked_session_id = session_id.clone();
    gpui::canvas(
        move |bounds, window, cx| {
            let Some(bounds) =
                terminal_visible_surface_bounds(bounds, window.content_mask().bounds)
            else {
                return;
            };
            let scale_factor = window.scale_factor();
            let unchanged = bounds_entity
                .read(cx)
                .terminal_surface_bounds_for_session(session_id.as_deref())
                .is_some_and(|previous| previous == bounds);
            if unchanged && bounds_entity.read(cx).terminal_scale_factor == scale_factor {
                return;
            }
            // Defer mutation so we never re-enter the entity while layout/prepaint is running.
            let entity = entity.clone();
            let session_id = tracked_session_id.clone();
            cx.defer(move |cx| {
                let _ = entity.update(cx, |this, cx| {
                    this.remember_terminal_surface_bounds_for_session_and_sync(
                        session_id.as_deref(),
                        bounds,
                        scale_factor,
                        cx,
                    );
                });
            });
        },
        move |bounds, _state, window, cx| {
            if !active {
                return;
            }
            let focus = input_entity.read(cx).terminal_focus.clone();
            if input_entity
                .read(cx)
                .settings
                .interaction_mac_ime_compatibility
            {
                window.handle_input(&focus, ElementInputHandler::new(bounds, input_entity), cx);
            }
        },
    )
    .absolute()
    .size_full()
}

impl EntityInputHandler for NyaTermApp {
    fn text_for_range(
        &mut self,
        range: Range<usize>,
        adjusted_range: &mut Option<Range<usize>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<String> {
        if self.multi_line_paste.is_some() && self.multi_line_paste_focus.is_focused(window) {
            let text = self.multi_line_paste_text();
            let byte_range = byte_range_from_utf16(text, &range);
            *adjusted_range = Some(utf16_range_from_bytes(text, &byte_range));
            return Some(text[byte_range].to_string());
        }
        if self.temporary_ssh_link_open
            && self.temporary_ssh_link_focus.is_focused(window)
            && !self.temporary_ssh_link_marked_text.is_empty()
        {
            let marked = &self.temporary_ssh_link_marked_text;
            let len = marked.encode_utf16().count();
            let start = range.start.min(len);
            let end = range.end.min(len).max(start);
            *adjusted_range = Some(start..end);
            return Some(marked.clone());
        }
        if self.security_unlock_prompt_open && !self.security_unlock_marked_text.is_empty() {
            let marked = &self.security_unlock_marked_text;
            let len = marked.encode_utf16().count();
            let start = range.start.min(len);
            let end = range.end.min(len).max(start);
            *adjusted_range = Some(start..end);
            return Some(marked.clone());
        }
        let quick_switch = self.quick_switch_state(cx);
        if quick_switch.is_open() && !quick_switch.marked_text().is_empty() {
            let marked = quick_switch.marked_text();
            let len = marked.encode_utf16().count();
            let start = range.start.min(len);
            let end = range.end.min(len).max(start);
            *adjusted_range = Some(start..end);
            return Some(marked.to_string());
        }
        if self.is_locked && !self.lock_password_marked_text.is_empty() {
            let marked = &self.lock_password_marked_text;
            let len = marked.encode_utf16().count();
            let start = range.start.min(len);
            let end = range.end.min(len).max(start);
            *adjusted_range = Some(start..end);
            return Some(marked.clone());
        }
        if self.sync_groups_open
            && self.sync_groups_search_focus.is_focused(window)
            && !self.sync_groups_search_marked_text.is_empty()
        {
            let marked = &self.sync_groups_search_marked_text;
            let len = marked.encode_utf16().count();
            let start = range.start.min(len);
            let end = range.end.min(len).max(start);
            *adjusted_range = Some(start..end);
            return Some(marked.clone());
        }
        if self.sync_groups_open
            && self.sync_groups_name_focus.is_focused(window)
            && !self.sync_groups_name_marked_text.is_empty()
        {
            let marked = &self.sync_groups_name_marked_text;
            let len = marked.encode_utf16().count();
            let start = range.start.min(len);
            let end = range.end.min(len).max(start);
            *adjusted_range = Some(start..end);
            return Some(marked.clone());
        }
        if self.rename_session_id.is_some()
            && self.rename_focus.is_focused(window)
            && !self.rename_marked_text.is_empty()
        {
            let marked = &self.rename_marked_text;
            let len = marked.encode_utf16().count();
            let start = range.start.min(len);
            let end = range.end.min(len).max(start);
            *adjusted_range = Some(start..end);
            return Some(marked.clone());
        }
        if self.startup_command_open
            && self.startup_command_focus.is_focused(window)
            && !self.startup_command_marked_text.is_empty()
        {
            let marked = &self.startup_command_marked_text;
            let len = marked.encode_utf16().count();
            let start = range.start.min(len);
            let end = range.end.min(len).max(start);
            *adjusted_range = Some(start..end);
            return Some(marked.clone());
        }
        if self.terminal_ime_marked_text.is_empty() {
            return None;
        }
        let len = self.terminal_ime_marked_text.encode_utf16().count();
        let start = range.start.min(len);
        let end = range.end.min(len).max(start);
        *adjusted_range = Some(start..end);
        Some(self.terminal_ime_marked_text.clone())
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        if self.security_unlock_prompt_open {
            let cursor = self.security_unlock_draft.encode_utf16().count();
            return Some(UTF16Selection {
                range: cursor..cursor,
                reversed: false,
            });
        }
        if self.temporary_ssh_link_open && self.temporary_ssh_link_focus.is_focused(window) {
            let cursor = self.temporary_ssh_link_draft.encode_utf16().count();
            return Some(UTF16Selection {
                range: cursor..cursor,
                reversed: false,
            });
        }
        let quick_switch = self.quick_switch_state(cx);
        if quick_switch.is_open() {
            let cursor = quick_switch.query().encode_utf16().count();
            return Some(UTF16Selection {
                range: cursor..cursor,
                reversed: false,
            });
        }
        if self.is_locked {
            let cursor = self.lock_password_draft.encode_utf16().count();
            return Some(UTF16Selection {
                range: cursor..cursor,
                reversed: false,
            });
        }
        if self.sync_groups_open && self.sync_groups_search_focus.is_focused(window) {
            let cursor = self.sync_groups_search_draft.encode_utf16().count();
            return Some(UTF16Selection {
                range: cursor..cursor,
                reversed: false,
            });
        }
        if self.sync_groups_open && self.sync_groups_name_focus.is_focused(window) {
            let cursor = self
                .selected_sync_group()
                .map(|group| group.name.encode_utf16().count())
                .unwrap_or_default();
            return Some(UTF16Selection {
                range: cursor..cursor,
                reversed: false,
            });
        }
        if self.multi_line_paste_focus.is_focused(window) {
            let text = self.multi_line_paste_text();
            let range = self.multi_line_paste_selected_byte_range();
            let reversed = self
                .multi_line_paste_anchor
                .is_some_and(|anchor| anchor > self.multi_line_paste_cursor);
            return Some(UTF16Selection {
                range: utf16_range_from_bytes(text, &range),
                reversed,
            });
        }
        if self.rename_session_id.is_some() && self.rename_focus.is_focused(window) {
            let cursor = self.rename_draft.encode_utf16().count();
            return Some(UTF16Selection {
                range: cursor..cursor,
                reversed: false,
            });
        }
        if self.startup_command_open && self.startup_command_focus.is_focused(window) {
            let cursor = self.startup_command_draft.encode_utf16().count();
            return Some(UTF16Selection {
                range: cursor..cursor,
                reversed: false,
            });
        }
        // GPUI's IME contract needs a valid insertion range even when there is
        // no marked text. This is also what lets CJK candidate windows anchor to
        // the terminal cursor instead of treating the surface as non-editable.
        Some(terminal_input_selection())
    }

    fn marked_text_range(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<Range<usize>> {
        if self.security_unlock_prompt_open {
            let len = self.security_unlock_marked_text.encode_utf16().count();
            return (len > 0).then_some(0..len);
        }
        let quick_switch = self.quick_switch_state(cx);
        if quick_switch.is_open() {
            let len = quick_switch.marked_text().encode_utf16().count();
            return (len > 0).then_some(0..len);
        }
        if self.is_locked {
            let len = self.lock_password_marked_text.encode_utf16().count();
            return (len > 0).then_some(0..len);
        }
        if self.sync_groups_open && self.sync_groups_search_focus.is_focused(window) {
            let len = self.sync_groups_search_marked_text.encode_utf16().count();
            return (len > 0).then_some(0..len);
        }
        if self.sync_groups_open && self.sync_groups_name_focus.is_focused(window) {
            let len = self.sync_groups_name_marked_text.encode_utf16().count();
            return (len > 0).then_some(0..len);
        }
        if self.multi_line_paste.is_some() && self.multi_line_paste_focus.is_focused(window) {
            return self
                .multi_line_paste_marked_range
                .as_ref()
                .map(|range| utf16_range_from_bytes(self.multi_line_paste_text(), range));
        }
        if self.temporary_ssh_link_open && self.temporary_ssh_link_focus.is_focused(window) {
            let len = self.temporary_ssh_link_marked_text.encode_utf16().count();
            return (len > 0).then_some(0..len);
        }
        if self.rename_session_id.is_some() && self.rename_focus.is_focused(window) {
            let len = self.rename_marked_text.encode_utf16().count();
            return (len > 0).then_some(0..len);
        }
        if self.startup_command_open && self.startup_command_focus.is_focused(window) {
            let len = self.startup_command_marked_text.encode_utf16().count();
            return (len > 0).then_some(0..len);
        }
        let len = self.terminal_ime_marked_text.encode_utf16().count();
        (len > 0).then_some(0..len)
    }

    fn unmark_text(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.security_unlock_prompt_open {
            self.security_unlock_marked_text.clear();
            return;
        }
        if self.quick_switch_open(cx) {
            self.update_quick_switch_state(cx, |store| store.clear_quick_switch_marked_text());
            return;
        }
        if self.is_locked {
            self.lock_password_marked_text.clear();
            return;
        }
        if self.sync_groups_open && self.sync_groups_search_focus.is_focused(window) {
            self.sync_groups_search_marked_text.clear();
            return;
        }
        if self.sync_groups_open && self.sync_groups_name_focus.is_focused(window) {
            self.sync_groups_name_marked_text.clear();
            return;
        }
        if self.multi_line_paste.is_some() && self.multi_line_paste_focus.is_focused(window) {
            self.multi_line_paste_marked_text.clear();
            self.multi_line_paste_marked_range = None;
            return;
        }
        if self.temporary_ssh_link_open && self.temporary_ssh_link_focus.is_focused(window) {
            self.temporary_ssh_link_marked_text.clear();
            return;
        }
        if self.rename_session_id.is_some() && self.rename_focus.is_focused(window) {
            self.rename_marked_text.clear();
            return;
        }
        if self.startup_command_open && self.startup_command_focus.is_focused(window) {
            self.startup_command_marked_text.clear();
            return;
        }
        self.terminal_ime_marked_text.clear();
    }

    fn replace_text_in_range(
        &mut self,
        range: Option<Range<usize>>,
        text: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.security_unlock_prompt_open {
            self.security_unlock_marked_text.clear();
            self.security_unlock_draft.push_str(text);
            self.security_unlock_error = None;
            cx.notify();
            return;
        }
        if self.quick_switch_open(cx) {
            self.update_quick_switch_state(cx, |store| store.replace_quick_switch_text(text));
            cx.notify();
            return;
        }
        if self.is_locked {
            self.lock_password_marked_text.clear();
            if !text.is_empty() {
                self.lock_password_draft.push_str(text);
                self.lock_status = self.tr("lockScreen.passwordPlaceholder").to_string();
            }
            cx.notify();
            return;
        }
        if self.sync_groups_open && self.sync_groups_search_focus.is_focused(window) {
            self.sync_groups_search_marked_text.clear();
            if !text.is_empty() {
                self.sync_groups_search_draft.push_str(text);
            }
            cx.notify();
            return;
        }
        if self.sync_groups_open && self.sync_groups_name_focus.is_focused(window) {
            self.sync_groups_name_marked_text.clear();
            if !text.is_empty() {
                let mut name = self
                    .selected_sync_group()
                    .map(|group| group.name.clone())
                    .unwrap_or_default();
                name.push_str(text);
                self.set_selected_sync_group_name(name, cx);
            }
            return;
        }
        if self.multi_line_paste.is_some() && self.multi_line_paste_focus.is_focused(window) {
            let range = range
                .as_ref()
                .map(|range| byte_range_from_utf16(self.multi_line_paste_text(), range))
                .or_else(|| self.multi_line_paste_marked_range.clone())
                .unwrap_or_else(|| self.multi_line_paste_selected_byte_range());
            self.replace_multi_line_paste_range(range, text, cx);
            return;
        }
        if self.temporary_ssh_link_open && self.temporary_ssh_link_focus.is_focused(window) {
            self.temporary_ssh_link_marked_text.clear();
            if !text.is_empty() {
                self.temporary_ssh_link_draft.push_str(text);
                self.temporary_ssh_link_error = None;
            }
            cx.notify();
            return;
        }
        if self.rename_session_id.is_some() && self.rename_focus.is_focused(window) {
            self.rename_marked_text.clear();
            if !text.is_empty() {
                let remaining = 64usize.saturating_sub(self.rename_draft.chars().count());
                self.rename_draft.extend(text.chars().take(remaining));
            }
            cx.notify();
            return;
        }
        if self.startup_command_open && self.startup_command_focus.is_focused(window) {
            self.startup_command_marked_text.clear();
            if !text.is_empty() {
                self.startup_command_draft.push_str(text);
            }
            cx.notify();
            return;
        }
        self.terminal_ime_marked_text.clear();
        if !text.is_empty() {
            if let Some(selected) = self.smart_cursor_selected_input_range() {
                if self.replace_smart_input_selection(selected, text, cx) {
                    return;
                }
            }
            let bytes = text.as_bytes().to_vec();
            let has_buffer_selection = self.terminal_selection.is_some()
                && self.smart_cursor_selected_input_range().is_none();
            if has_buffer_selection {
                self.send_terminal_input_without_suggestion_track(bytes, cx);
            } else {
                self.send_terminal_input(bytes, cx);
            }
        }
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range: Option<Range<usize>>,
        new_text: &str,
        new_selected_range: Option<Range<usize>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.security_unlock_prompt_open {
            self.security_unlock_marked_text = new_text.to_string();
            cx.notify();
            return;
        }
        if self.quick_switch_open(cx) {
            self.update_quick_switch_state(cx, |store| {
                store.set_quick_switch_marked_text(new_text)
            });
            cx.notify();
            return;
        }
        if self.is_locked {
            self.lock_password_marked_text = new_text.to_string();
            cx.notify();
            return;
        }
        if self.sync_groups_open && self.sync_groups_search_focus.is_focused(window) {
            self.sync_groups_search_marked_text = new_text.to_string();
            cx.notify();
            return;
        }
        if self.sync_groups_open && self.sync_groups_name_focus.is_focused(window) {
            self.sync_groups_name_marked_text = new_text.to_string();
            cx.notify();
            return;
        }
        if self.multi_line_paste.is_some() && self.multi_line_paste_focus.is_focused(window) {
            let range = range
                .as_ref()
                .map(|range| byte_range_from_utf16(self.multi_line_paste_text(), range))
                .or_else(|| self.multi_line_paste_marked_range.clone())
                .unwrap_or_else(|| self.multi_line_paste_selected_byte_range());
            let start = range.start;
            self.replace_multi_line_paste_range(range, new_text, cx);
            self.multi_line_paste_marked_text = new_text.to_string();
            self.multi_line_paste_marked_range =
                (!new_text.is_empty()).then_some(start..start + new_text.len());
            if let Some(selected) = new_selected_range {
                let selected = byte_range_from_utf16(new_text, &selected);
                self.multi_line_paste_anchor =
                    (selected.start != selected.end).then_some(start + selected.start);
                self.multi_line_paste_cursor = start + selected.end;
            }
            cx.notify();
            return;
        }
        if self.temporary_ssh_link_open && self.temporary_ssh_link_focus.is_focused(window) {
            self.temporary_ssh_link_marked_text = new_text.to_string();
            cx.notify();
            return;
        }
        if self.rename_session_id.is_some() && self.rename_focus.is_focused(window) {
            self.rename_marked_text = new_text.chars().take(64).collect();
            cx.notify();
            return;
        }
        if self.startup_command_open && self.startup_command_focus.is_focused(window) {
            self.startup_command_marked_text = new_text.to_string();
            cx.notify();
            return;
        }
        self.terminal_ime_marked_text = new_text.to_string();
        cx.notify();
    }

    fn bounds_for_range(
        &mut self,
        _range_utf16: Range<usize>,
        element_bounds: Bounds<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        if self.security_unlock_prompt_open {
            return Some(gpui::bounds(
                Point {
                    x: element_bounds.left(),
                    y: element_bounds.bottom() - px(18.),
                },
                Size {
                    width: px(1.),
                    height: px(18.),
                },
            ));
        }
        if self.quick_switch_open(cx)
            || self.is_locked
            || (self.sync_groups_open
                && (self.sync_groups_search_focus.is_focused(window)
                    || self.sync_groups_name_focus.is_focused(window)))
            || (self.multi_line_paste.is_some() && self.multi_line_paste_focus.is_focused(window))
            || (self.rename_session_id.is_some() && self.rename_focus.is_focused(window))
            || (self.startup_command_open && self.startup_command_focus.is_focused(window))
        {
            return Some(element_bounds);
        }
        let (cell_w, cell_h) = self.terminal_cell_size();
        let insets = self.terminal_content_insets();
        let gutter = self.terminal_gutter_width_px();
        let snapshot = self.terminal_snapshot_for_session(self.active_session_id.as_deref(), 0);
        let row = if snapshot.cursor.row == usize::MAX {
            snapshot.row_count().saturating_sub(1)
        } else {
            snapshot
                .cursor
                .row
                .min(snapshot.row_count().saturating_sub(1))
        };
        let col = snapshot.cursor.col.min(snapshot.cols.saturating_sub(1));
        let origin_x = f32::from(element_bounds.origin.x);
        let origin_y = f32::from(element_bounds.origin.y);
        let max_x = origin_x + f32::from(element_bounds.size.width) - cell_w.max(1.);
        let max_y = origin_y + f32::from(element_bounds.size.height) - cell_h.max(1.);
        let x = (origin_x + insets.left + gutter + col as f32 * cell_w)
            .min(max_x)
            .max(origin_x);
        let y = (origin_y + insets.top + row as f32 * cell_h)
            .min(max_y)
            .max(origin_y);
        Some(gpui::bounds(
            Point { x: px(x), y: px(y) },
            Size {
                width: px(cell_w.max(1.)),
                height: px(cell_h.max(1.)),
            },
        ))
    }

    fn character_index_for_point(
        &mut self,
        _point: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<usize> {
        if self.security_unlock_prompt_open {
            return Some(self.security_unlock_draft.encode_utf16().count());
        }
        let quick_switch = self.quick_switch_state(cx);
        if quick_switch.is_open() {
            return Some(quick_switch.query().encode_utf16().count());
        }
        if self.is_locked {
            return Some(self.lock_password_draft.encode_utf16().count());
        }
        if self.sync_groups_open && self.sync_groups_search_focus.is_focused(window) {
            return Some(self.sync_groups_search_draft.encode_utf16().count());
        }
        if self.sync_groups_open && self.sync_groups_name_focus.is_focused(window) {
            return Some(
                self.selected_sync_group()
                    .map(|group| group.name.encode_utf16().count())
                    .unwrap_or_default(),
            );
        }
        if self.multi_line_paste.is_some() && self.multi_line_paste_focus.is_focused(window) {
            return Some(utf16_offset_for_byte(
                self.multi_line_paste_text(),
                self.multi_line_paste_cursor,
            ));
        }
        if self.rename_session_id.is_some() && self.rename_focus.is_focused(window) {
            return Some(self.rename_draft.encode_utf16().count());
        }
        if self.startup_command_open && self.startup_command_focus.is_focused(window) {
            return Some(self.startup_command_draft.encode_utf16().count());
        }
        Some(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_input_selection_keeps_a_valid_insertion_point() {
        let selection = terminal_input_selection();

        assert_eq!(selection.range, 0..0);
        assert!(!selection.reversed);
    }

    #[test]
    fn terminal_surface_bounds_follow_the_visible_content_mask() {
        let bounds = Bounds::new(
            gpui::point(px(10.), px(20.)),
            gpui::size(px(600.), px(4_000.)),
        );
        let content_mask = Bounds::new(
            gpui::point(px(0.), px(0.)),
            gpui::size(px(1_280.), px(800.)),
        );

        assert_eq!(
            terminal_visible_surface_bounds(bounds, content_mask),
            Some(Bounds::new(
                gpui::point(px(10.), px(20.)),
                gpui::size(px(600.), px(780.))
            ))
        );
    }

    #[test]
    fn terminal_surface_bounds_exclude_scrollbar_column() {
        let outer_width = 610.;
        let text_width =
            outer_width - crate::features::terminal_surface::TERMINAL_SCROLLBAR_COLUMN_WIDTH;
        let bounds = Bounds::new(
            gpui::point(px(10.), px(20.)),
            gpui::size(px(text_width), px(400.)),
        );
        let content_mask = Bounds::new(
            gpui::point(px(0.), px(0.)),
            gpui::size(px(1_280.), px(800.)),
        );

        assert_eq!(
            terminal_visible_surface_bounds(bounds, content_mask),
            Some(Bounds::new(
                gpui::point(px(10.), px(20.)),
                gpui::size(px(600.), px(400.))
            ))
        );
    }

    #[test]
    fn terminal_surface_bounds_skip_fully_clipped_surfaces() {
        let bounds = Bounds::new(
            gpui::point(px(10.), px(900.)),
            gpui::size(px(600.), px(400.)),
        );
        let content_mask = Bounds::new(
            gpui::point(px(0.), px(0.)),
            gpui::size(px(1_280.), px(800.)),
        );

        assert_eq!(terminal_visible_surface_bounds(bounds, content_mask), None);
    }
}

fn byte_offset_from_utf16(text: &str, offset: usize) -> usize {
    let mut utf16_offset = 0usize;
    for (byte_offset, ch) in text.char_indices() {
        if utf16_offset >= offset {
            return byte_offset;
        }
        utf16_offset += ch.len_utf16();
        if utf16_offset >= offset {
            return byte_offset + ch.len_utf8();
        }
    }
    text.len()
}

fn byte_range_from_utf16(text: &str, range: &Range<usize>) -> Range<usize> {
    let start = byte_offset_from_utf16(text, range.start);
    let end = byte_offset_from_utf16(text, range.end).max(start);
    start..end
}

fn utf16_offset_for_byte(text: &str, offset: usize) -> usize {
    let offset = offset.min(text.len());
    text[..offset].encode_utf16().count()
}

fn utf16_range_from_bytes(text: &str, range: &Range<usize>) -> Range<usize> {
    utf16_offset_for_byte(text, range.start)..utf16_offset_for_byte(text, range.end)
}

pub(super) fn open_external_url_for_action(url: &str) -> Result<(), String> {
    let url = url.trim();
    if url.is_empty() {
        return Err("empty url".to_string());
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(url)
            .spawn()
            .map(|_| ())
            .map_err(|error| format!("failed to open url: {error}"))
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", ""])
            .arg(url)
            .spawn()
            .map(|_| ())
            .map_err(|error| format!("failed to open url: {error}"))
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        std::process::Command::new("xdg-open")
            .arg(url)
            .spawn()
            .map(|_| ())
            .map_err(|error| format!("failed to open url: {error}"))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::features) enum SmartSelectionEdge {
    Start,
    End,
}
