use gpui::{
    Context, IntoElement, KeyDownEvent, ListHorizontalSizingBehavior, MouseButton, SharedString,
    div,
    prelude::{
        FluentBuilder, InteractiveElement, ParentElement, StatefulInteractiveElement, Styled,
    },
    px, rgb, svg, uniform_list,
};
use nyaterm_ui::{NyaContextMenu, NyaDropdownMenu};

use crate::features::{ConnectionDragKind, ConnectionDragPayload, NyaTermApp};
use crate::models::ConnectionSortMode;

use super::super::list::{
    ConnectionListRow, connection_sections, connection_tree_indent_px, flatten_connection_rows,
    icon_action_button, icon_action_button_styled,
};

const CONNECTION_LIST_ROW_HEIGHT_PX: f32 = 34.;

/// Index of the connection row that is most likely the widest.
///
/// `uniform_list` measures a single row to decide how far the list can scroll
/// sideways, so pointing it at row 0 would cap the scroll at whatever that row
/// happens to be. This picks the candidate by indent plus rendered name width —
/// an estimate, since the real width comes from the text system, but one that
/// only has to identify the right row rather than its exact size.
fn widest_connection_row(rows: &[ConnectionListRow]) -> Option<usize> {
    rows.iter()
        .enumerate()
        .filter_map(|(index, row)| match row {
            ConnectionListRow::Connection { connection, depth } => {
                let name_width: usize = connection
                    .name
                    .chars()
                    // CJK and other wide glyphs take about two Latin advances.
                    .map(|c| if c as u32 >= 0x1100 { 2 } else { 1 })
                    .sum();
                Some((index, *depth * 16 + name_width * 8))
            }
            _ => None,
        })
        .max_by_key(|(_, width)| *width)
        .map(|(index, _)| index)
}

impl NyaTermApp {
    pub(in crate::features) fn connections_view(
        &mut self,
        window: &mut gpui::Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let query = self.connection_state.list_search_query();
        let sections = connection_sections(
            self.connection_state.connections(),
            self.connection_state.groups(),
            &query,
            self.connection_state.list_sort_mode(),
        );
        // Folders start closed, so a filter would otherwise match into a tree the
        // user cannot see. Open the folders that still have hits, and put the tree
        // back once the filter clears.
        self.connection_state.sync_list_search_expansion(
            &query,
            sections
                .iter()
                .filter_map(|section| section.group_id.clone()),
        );
        let empty_connections_label = self.tr("savedConnections.empty");
        let empty_connections_hint = self.tr("savedConnections.emptyHint");
        let no_results_label = self.tr("savedConnections.noResults");
        let empty_group_label = self.tr("savedConnections.emptyGroup");

        // Keep the flattened model cheap to rebuild, then let GPUI instantiate only
        // the rows intersecting the scroll viewport.
        let flat_rows =
            flatten_connection_rows(&sections, self.connection_state.list_expanded_group_ids());
        // A folder is worth showing even before anything is filed under it, so the
        // empty state waits until there are no folders either. Otherwise a freshly
        // created folder is swallowed by "no saved connections".
        let store_is_empty = self.connection_state.connections().is_empty()
            && self.connection_state.groups().is_empty();
        let nothing_matched = flat_rows.is_empty();
        let palette = self.theme_palette();

        let mut list = div()
            .id(SharedString::from("connections-list-scroll"))
            .flex_1()
            .min_h_0()
            .p(px(6.))
            .flex()
            .flex_col()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    // Click empty background clears multi-select (Tauri list onMouseDown).
                    if this.connection_state.list_has_selection() {
                        this.clear_selected_connections(cx);
                    }
                }),
            )
            .on_drop(cx.listener(|this, payload: &ConnectionDragPayload, _, cx| {
                this.connection_state.clear_list_drop_target();
                match payload.kind {
                    ConnectionDragKind::Connection => {
                        this.move_connection_into_group(payload.id.clone(), None, cx);
                    }
                    ConnectionDragKind::Group => {
                        this.move_group_into_group(payload.id.clone(), None, cx);
                    }
                }
            }));
        if store_is_empty {
            list = list.child(
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .px_4()
                    .py_8()
                    .gap_2()
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(rgb(palette.text_muted))
                            .child(empty_connections_label),
                    )
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(rgb(palette.text_dimmed))
                            .child(empty_connections_hint),
                    ),
            );
        } else if nothing_matched {
            list = list.child(
                div()
                    .px_4()
                    .py_8()
                    .text_size(px(11.))
                    .text_color(rgb(palette.text_dimmed))
                    .child(no_results_label),
            );
        } else {
            let row_count = flat_rows.len();
            // `uniform_list` derives its scrollable width from one measured row, so
            // point it at the row most likely to be the widest or long names would
            // still be unreachable.
            let widest_row = widest_connection_row(&flat_rows);
            list = list.child(
                uniform_list(
                    "connections-list-rows",
                    row_count,
                    cx.processor(move |this, range: std::ops::Range<usize>, _, cx| {
                        let mut items = Vec::with_capacity(range.len());
                        for index in range {
                            let Some(row) = flat_rows.get(index).cloned() else {
                                continue;
                            };
                            // A definite width here, so the rows inside can resolve
                            // their `min_w(relative(1.))` and still overflow it when
                            // the name is long. It tracks the horizontal scroll.
                            let item = div()
                                .h(px(CONNECTION_LIST_ROW_HEIGHT_PX))
                                .w_full()
                                .flex_none()
                                .flex()
                                .items_center();
                            items.push(match row {
                                ConnectionListRow::Separator => item
                                    .child(div().mx_2().h(px(1.)).w_full().bg(rgb(palette.border))),
                                ConnectionListRow::GroupHeader(section) => item.child(
                                    div()
                                        .w_full()
                                        .child(this.connection_section(section, true, cx)),
                                ),
                                ConnectionListRow::EmptyGroup { depth } => item.child(
                                    div()
                                        .w_full()
                                        .px_2()
                                        .pl(px(connection_tree_indent_px(depth)))
                                        .h(px(28.))
                                        .flex()
                                        .items_center()
                                        .text_size(px(11.))
                                        .text_color(rgb(palette.text_dimmed))
                                        .child(empty_group_label),
                                ),
                                ConnectionListRow::Connection { connection, depth } => {
                                    item.child(div().w_full().child(this.saved_connection_row(
                                        *connection,
                                        depth,
                                        cx,
                                    )))
                                }
                            });
                        }
                        items
                    }),
                )
                .with_horizontal_sizing_behavior(ListHorizontalSizingBehavior::Unconstrained)
                .with_width_from_item(widest_row)
                .flex_1()
                .min_h_0(),
            );
        }

        let list = NyaContextMenu::new(list, self.connection_list_context_menu_items(cx));

        // Tauri: PanelHeader (shared stack) + search/action strip + flat tree list.
        // Count is shown in the shared panel header via meta; strip hosts search + icons.
        div()
            .relative()
            .flex()
            .flex_col()
            .size_full()
            .overflow_hidden()
            .bg(self.shell_transparent_color(palette.surface))
            .child(self.connections_search_bar(window, cx))
            .child(list)
    }

    pub(in crate::features) fn connections_search_bar(
        &mut self,
        window: &mut gpui::Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let search_empty = self.connection_state.list_search_is_empty();
        let search_field = self.connection_state.list_search_field();
        let search_focus = search_field.read(cx).focus_handle();
        let search_focused = search_focus.is_focused(window);
        // Tauri swaps the glyph, flips it for Z-A and tints it while a name sort is
        // active, so the current mode is readable without hovering for the tooltip.
        let sort_mode = self.connection_state.list_sort_mode();
        let sort_label = match sort_mode {
            ConnectionSortMode::Default => "icons/conn/sort.svg",
            ConnectionSortMode::NameAsc | ConnectionSortMode::NameDesc => {
                "icons/conn/sort-alpha.svg"
            }
        };
        let sort_tint = (sort_mode != ConnectionSortMode::Default).then_some(palette.primary);
        let sort_flipped = sort_mode == ConnectionSortMode::NameDesc;
        let sort_tooltip = self.tr(match sort_mode {
            ConnectionSortMode::Default => "savedConnections.sortDefault",
            ConnectionSortMode::NameAsc => "savedConnections.sortNameAsc",
            ConnectionSortMode::NameDesc => "savedConnections.sortNameDesc",
        });
        let more_menu = NyaDropdownMenu::new("connections-more")
            .icon("icons/conn/more.svg")
            .icon_size(px(14.))
            .tooltip(self.tr("common.more"))
            .min_width(px(180.))
            .items(self.connection_more_menu_items(cx))
            .on_trigger(|_, _, cx| cx.stop_propagation());

        // Tauri search strip: px-2 py-1.5, input h-7.
        div()
            .h(px(36.))
            .px_2()
            .flex()
            .items_center()
            .gap_1()
            .border_b_1()
            .border_color(rgb(palette.border))
            .bg(self.shell_transparent_color(palette.section_header))
            .child(
                div()
                    .id(SharedString::from("connection-search-input"))
                    .h(px(28.))
                    .flex_1()
                    .min_w_0()
                    .relative()
                    .rounded_md()
                    .border_1()
                    // Tauri gives the focused field a primary ring; without it the
                    // box looked identical whether or not it had focus.
                    .border_color(rgb(if search_focused {
                        palette.primary
                    } else {
                        palette.border
                    }))
                    .bg(rgb(palette.hover))
                    .px_2()
                    .flex()
                    .items_center()
                    .gap_2()
                    .cursor_text()
                    .on_click(cx.listener(|this, _, window, cx| {
                        let field = this.connection_state.list_search_field();
                        window.focus(&field.read(cx).focus_handle(), cx);
                        cx.notify();
                    }))
                    // Result navigation stays here: the field leaves the arrows
                    // and enter unconsumed precisely so the list can claim them.
                    .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                        this.handle_connection_search_key_down(event, window, cx);
                    }))
                    .child(
                        svg()
                            .size(px(14.))
                            .flex_none()
                            .path("icons/fe/search.svg")
                            .text_color(rgb(palette.text_dimmed)),
                    )
                    .child(
                        div()
                            .min_w_0()
                            .flex_1()
                            .text_size(px(12.))
                            .child(search_field.clone()),
                    )
                    .when(!search_empty, |this| {
                        this.child(
                            div()
                                .id(SharedString::from("connection-search-clear"))
                                .size(px(18.))
                                .flex()
                                .items_center()
                                .justify_center()
                                .rounded_sm()
                                .text_size(px(10.))
                                .text_color(rgb(palette.text_muted))
                                .cursor_pointer()
                                .hover(move |this| {
                                    this.bg(rgb(palette.surface_elevated))
                                        .text_color(rgb(palette.text))
                                })
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.clear_connection_search(window, cx);
                                }))
                                .child(
                                    svg()
                                        .size(px(13.))
                                        .path("icons/window/close.svg")
                                        .text_color(rgb(palette.text_muted)),
                                ),
                        )
                    }),
            )
            // Count lives in PanelHeader (Tauri).
            .child(icon_action_button_styled(
                palette,
                "connections-sort",
                sort_label,
                sort_tooltip,
                sort_tint,
                sort_flipped,
                cx.listener(|this, _, _, cx| {
                    this.cycle_connection_sort_mode(cx);
                }),
            ))
            .child(icon_action_button(
                palette,
                "connections-temp-ssh",
                "icons/conn/flash.svg",
                self.tr("temporarySsh.title"),
                cx.listener(|this, _, window, cx| {
                    this.open_temporary_ssh_link_dialog(window, cx);
                }),
            ))
            .child(icon_action_button(
                palette,
                "connections-new-group",
                "icons/conn/folder.svg",
                self.tr("savedConnections.newFolder"),
                cx.listener(|this, _, window, cx| {
                    this.open_connection_group_editor(None, None, window, cx);
                }),
            ))
            .child(icon_action_button(
                palette,
                "connections-new",
                "icons/conn/add.svg",
                self.tr("savedConnections.newConnection"),
                cx.listener(|this, _, window, cx| {
                    this.open_connection_editor(None, None, false, window, cx);
                }),
            ))
            .child(more_menu)
    }
}
