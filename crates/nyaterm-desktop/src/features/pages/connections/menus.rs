use gpui::{
    AnyElement, Context, Edges, IntoElement, Pixels, SharedString, anchored, deferred, div, point,
    prelude::{
        FluentBuilder, InteractiveElement, ParentElement, StatefulInteractiveElement, Styled,
    },
    px, rgb, svg,
};

use crate::features::NyaTermApp;
use crate::models::{
    ConnectionContextMenuState, ConnectionGroupContextMenuState, ConnectionListContextMenuState,
};

use super::editor::ordered_connection_groups;
use super::list::{menu_item_submenu_trigger, menu_item_with_icon, menu_separator};

const CONNECTION_MENU_WIDTH_PX: f32 = 180.;
const CONNECTION_MENU_MARGIN_PX: f32 = 8.;

fn connection_menu_max_height(viewport_height: f32) -> f32 {
    (viewport_height - CONNECTION_MENU_MARGIN_PX * 2.).max(80.)
}

impl NyaTermApp {
    /// Wrap a context menu so it is positioned in *window* space and painted on
    /// top of everything.
    ///
    /// The coordinates come from a `MouseDownEvent`, which reports window-relative
    /// positions, while the panel this menu is declared in is both offset from the
    /// window origin and `overflow_hidden`. Laying the menu out inline therefore
    /// put it at the wrong place and clipped whatever hung past the panel edge —
    /// which, in a sidebar this narrow, was usually the whole menu. `anchored`
    /// interprets the position against the window and keeps the menu on screen;
    /// `deferred` moves the paint out from under the panel's clip rect.
    fn connection_menu_overlay(
        &mut self,
        id: &'static str,
        x: Pixels,
        y: Pixels,
        menu: impl IntoElement,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        self.connection_menu_overlay_with_submenu(id, x, y, menu, None, cx)
    }

    /// As [`Self::connection_menu_overlay`], plus an optional flyout.
    ///
    /// The flyout is a *descendant* of the menu rather than a second overlay, so
    /// the menu's `on_mouse_down_out` does not fire while the pointer is inside it
    /// and tear the whole stack down. That is also why the scroll container moved
    /// inward: the flyout has to hang outside the menu's box without being clipped.
    fn connection_menu_overlay_with_submenu(
        &mut self,
        id: &'static str,
        x: Pixels,
        y: Pixels,
        menu: impl IntoElement,
        submenu: Option<AnyElement>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let palette = self.theme_palette();
        let (viewport_width, viewport_height) = self.last_viewport_size;
        let max_height = connection_menu_max_height(viewport_height);
        let submenu_offset = connection_submenu_offset(f32::from(x), viewport_width);
        deferred(
            anchored()
                .position(point(x, y))
                .snap_to_window_with_margin(Edges::all(px(CONNECTION_MENU_MARGIN_PX)))
                .child(
                    div()
                        .id(SharedString::from(id))
                        .occlude()
                        .relative()
                        .w(px(CONNECTION_MENU_WIDTH_PX))
                        .rounded_md()
                        .border_1()
                        .border_color(rgb(palette.border))
                        .bg(self.shell_surface_color(palette.bg))
                        .shadow_lg()
                        .py_1()
                        .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                            this.close_connection_context_menus(cx);
                        }))
                        .child(
                            div()
                                .id(SharedString::from(format!("{id}-items")))
                                .max_h(px(max_height))
                                .overflow_y_scroll()
                                .child(menu),
                        )
                        .when_some(submenu, |this, submenu| {
                            this.child(
                                anchored()
                                    .position(point(px(submenu_offset), y))
                                    .snap_to_window_with_margin(Edges::all(px(
                                        CONNECTION_MENU_MARGIN_PX,
                                    )))
                                    .child(submenu),
                            )
                        }),
                ),
        )
        .with_priority(1)
        .into_any_element()
    }
}

/// Where the flyout goes: beside the menu, flipping to its left when the window
/// edge is too close for another panel.
fn connection_submenu_offset(menu_x: f32, viewport_width: f32) -> f32 {
    let gap = 2.;
    let right = menu_x + CONNECTION_MENU_WIDTH_PX + gap;
    if right + CONNECTION_MENU_WIDTH_PX <= viewport_width - CONNECTION_MENU_MARGIN_PX {
        right
    } else {
        (menu_x - CONNECTION_MENU_WIDTH_PX - gap).max(CONNECTION_MENU_MARGIN_PX)
    }
}

impl NyaTermApp {
    /// The "move to group" flyout: Ungrouped, then every folder indented by depth.
    ///
    /// The old UI nested one hover-cascade level per folder. A single indented
    /// panel is the same choice in one step, matches the group dropdown in the
    /// connection editor, and keeps the hover-path state out of a clipped panel.
    pub(in crate::features::pages::connections) fn connection_move_to_group_submenu(
        &mut self,
        id: &'static str,
        move_ids: Vec<String>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let palette = self.theme_palette();
        let max_height = connection_menu_max_height(self.last_viewport_size.1);
        let ungrouped_ids = move_ids.clone();
        let mut panel = div()
            .id(SharedString::from(id))
            .occlude()
            .w(px(CONNECTION_MENU_WIDTH_PX))
            .max_h(px(max_height))
            .overflow_y_scroll()
            .rounded_md()
            .border_1()
            .border_color(rgb(palette.border))
            .bg(self.shell_surface_color(palette.bg))
            .shadow_lg()
            .py_1()
            .child(menu_item_with_icon(
                palette,
                "connection-move-to-root",
                "icons/conn/connect.svg",
                self.tr("savedConnections.ungroupedConnections"),
                false,
                cx.listener(move |this, _, _, cx| {
                    this.close_connection_context_menus(cx);
                    this.move_connections_into_group(ungrouped_ids.clone(), None, cx);
                }),
            ));

        let groups = ordered_connection_groups(&self.connection_groups);
        if !groups.is_empty() {
            panel = panel.child(menu_separator(palette));
        }
        for (group, depth) in groups {
            let group_id = group.id.clone();
            let ids = move_ids.clone();
            panel = panel.child(
                div()
                    .id(SharedString::from(format!("connection-move-to-{group_id}")))
                    .h(px(28.))
                    .pr_3()
                    .pl(px(12. + depth as f32 * 14.))
                    .flex()
                    .items_center()
                    .gap_2()
                    .text_size(px(12.))
                    .text_color(rgb(palette.text))
                    .cursor_pointer()
                    .hover(|this| this.bg(rgb(palette.surface_elevated)))
                    .child(
                        svg()
                            .size(px(13.))
                            .flex_none()
                            .path("icons/conn/folder.svg")
                            .text_color(rgb(palette.text_muted)),
                    )
                    .child(div().min_w_0().flex_1().child(group.name.clone()))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.close_connection_context_menus(cx);
                        this.move_connections_into_group(ids.clone(), Some(group_id.clone()), cx);
                    })),
            );
        }
        panel.into_any_element()
    }

    pub(in crate::features) fn connection_context_menu_overlay(
        &mut self,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let palette = self.theme_palette();
        let state = self.connection_state.list.active_context_menu().unwrap_or(
            ConnectionContextMenuState {
                connection_id: String::new(),
                x: px(24.),
                y: px(24.),
            },
        );
        let connection = self
            .connections
            .iter()
            .find(|connection| connection.id == state.connection_id)
            .cloned();
        let selected_count = self.selected_connections().len();
        let connect_label = if selected_count > 1
            && connection
                .as_ref()
                .is_some_and(|conn| self.connection_state.list.contains_selected_id(&conn.id))
        {
            format!(
                "{} ({selected_count})",
                self.tr("savedConnections.connectSelected")
            )
        } else {
            self.tr("savedConnections.connect").to_string()
        };
        let connection_id = state.connection_id.clone();
        let connection_for_connect = connection.clone();
        let connection_for_edit = connection_id.clone();
        let connection_for_rename = connection_id.clone();
        let connection_for_copy = connection_id.clone();
        let connection_for_delete = connection_id.clone();
        let move_submenu_open = self.connection_state.list.move_submenu_is_open();
        // A row that belongs to a multi-selection moves the whole selection, the
        // same rule the connect entry above already uses.
        let move_ids = if selected_count > 1
            && self
                .connection_state
                .list
                .contains_selected_id(&state.connection_id)
        {
            self.selected_connections()
                .into_iter()
                .map(|connection| connection.id)
                .collect()
        } else {
            vec![state.connection_id.clone()]
        };

        let menu = div()
            .flex()
            .flex_col()
            .child(menu_item_with_icon(
                palette,
                "connection-context-connect",
                "icons/conn/connect.svg",
                connect_label,
                false,
                cx.listener(move |this, _, window, cx| {
                    this.close_connection_context_menus(cx);
                    let selected = this.selected_connections();
                    if selected.len() > 1 && selected.iter().any(|conn| conn.id == connection_id) {
                        this.start_selected_saved_connections(window, cx);
                    } else if let Some(connection) = connection_for_connect.clone() {
                        this.start_saved_connection(connection, window, cx);
                    }
                }),
            ))
            .child(menu_item_with_icon(
                palette,
                "connection-context-edit",
                "icons/net/edit.svg",
                self.tr("savedConnections.edit"),
                false,
                cx.listener(move |this, _, window, cx| {
                    this.close_connection_context_menus(cx);
                    this.open_connection_editor(
                        Some(connection_for_edit.clone()),
                        None,
                        false,
                        window,
                        cx,
                    );
                }),
            ))
            .child(menu_separator(palette))
            .child(menu_item_with_icon(
                palette,
                "connection-context-rename",
                "icons/session/rename.svg",
                self.tr("savedConnections.rename"),
                false,
                cx.listener(move |this, _, window, cx| {
                    this.close_connection_context_menus(cx);
                    this.rename_connection(connection_for_rename.clone(), window, cx);
                }),
            ))
            .child(menu_item_with_icon(
                palette,
                "connection-context-copy",
                "icons/copy.svg",
                if selected_count > 1 {
                    self.tr("savedConnections.copySelected")
                } else {
                    self.tr("savedConnections.copy")
                },
                false,
                cx.listener(move |this, _, _, cx| {
                    this.close_connection_context_menus(cx);
                    if this.selected_connections().len() > 1 {
                        this.copy_selected_connections(cx);
                    } else {
                        this.copy_connection_by_id(connection_for_copy.clone(), cx);
                    }
                }),
            ))
            .child(menu_item_submenu_trigger(
                palette,
                "connection-context-move",
                "icons/net/move.svg",
                self.tr("savedConnections.moveToGroup"),
                move_submenu_open,
                cx.listener(|this, _, _, cx| {
                    this.connection_state.list.toggle_move_submenu();
                    cx.notify();
                }),
            ))
            .child(menu_separator(palette))
            .child(menu_item_with_icon(
                palette,
                "connection-context-delete",
                "icons/net/delete.svg",
                if selected_count > 1 {
                    self.tr("savedConnections.delete")
                } else {
                    self.tr("savedConnections.delete")
                },
                true,
                cx.listener(move |this, _, _, cx| {
                    this.close_connection_context_menus(cx);
                    if this.selected_connections().len() > 1 {
                        this.delete_selected_connections(cx);
                    } else {
                        this.open_connection_delete_confirm(connection_for_delete.clone(), cx);
                    }
                }),
            ));

        let submenu = move_submenu_open.then(|| {
            self.connection_move_to_group_submenu("connection-context-move-menu", move_ids, cx)
        });
        self.connection_menu_overlay_with_submenu(
            "connection-context-menu",
            state.x,
            state.y,
            menu,
            submenu,
            cx,
        )
    }

    pub(in crate::features) fn connection_group_context_menu_overlay(
        &mut self,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let palette = self.theme_palette();
        let state = self
            .connection_state
            .list
            .active_group_context_menu()
            .unwrap_or(ConnectionGroupContextMenuState {
                group_id: String::new(),
                x: px(24.),
                y: px(24.),
            });
        let group_id = state.group_id.clone();
        let group_id_new = group_id.clone();
        let group_id_folder = group_id.clone();
        let group_id_open = group_id.clone();
        let group_id_edit = group_id.clone();
        let group_id_delete = group_id.clone();
        let total_in_group = {
            let mut group_ids = std::collections::HashSet::from([group_id.clone()]);
            let mut changed = true;
            while changed {
                changed = false;
                for group in &self.connection_groups {
                    if let Some(parent) = group.parent_id.as_ref() {
                        if group_ids.contains(parent) && group_ids.insert(group.id.clone()) {
                            changed = true;
                        }
                    }
                }
            }
            self.connections
                .iter()
                .filter(|connection| {
                    connection
                        .group_id
                        .as_ref()
                        .is_some_and(|id| group_ids.contains(id))
                })
                .count()
        };
        let menu = div()
            .flex()
            .flex_col()
            .child(menu_item_with_icon(
                palette,
                "connection-group-context-new",
                "icons/conn/add.svg",
                self.tr("savedConnections.newConnection"),
                false,
                cx.listener(move |this, _, window, cx| {
                    this.close_connection_context_menus(cx);
                    this.open_connection_editor(
                        None,
                        Some(group_id_new.clone()),
                        false,
                        window,
                        cx,
                    );
                }),
            ))
            .child(menu_item_with_icon(
                palette,
                "connection-group-context-folder",
                "icons/fe/new-folder.svg",
                self.tr("savedConnections.newFolder"),
                false,
                cx.listener(move |this, _, window, cx| {
                    this.close_connection_context_menus(cx);
                    this.open_connection_group_editor(
                        None,
                        Some(group_id_folder.clone()),
                        window,
                        cx,
                    );
                }),
            ))
            .when(total_in_group > 0, |this| {
                this.child(menu_separator(palette))
                    .child(menu_item_with_icon(
                        palette,
                        "connection-group-context-open-all",
                        "icons/fe/forward.svg",
                        self.tr("savedConnections.openAllConnections"),
                        false,
                        cx.listener(move |this, _, window, cx| {
                            this.close_connection_context_menus(cx);
                            this.open_connection_group_open_confirm(
                                group_id_open.clone(),
                                window,
                                cx,
                            );
                        }),
                    ))
            })
            .child(menu_separator(palette))
            .child(menu_item_with_icon(
                palette,
                "connection-group-context-rename",
                "icons/session/rename.svg",
                self.tr("savedConnections.renameFolder"),
                false,
                cx.listener(move |this, _, window, cx| {
                    this.close_connection_context_menus(cx);
                    this.open_connection_group_editor(
                        Some(group_id_edit.clone()),
                        None,
                        window,
                        cx,
                    );
                }),
            ))
            .child(menu_item_with_icon(
                palette,
                "connection-group-context-delete",
                "icons/net/delete.svg",
                self.tr("savedConnections.deleteFolder"),
                true,
                cx.listener(move |this, _, _, cx| {
                    this.close_connection_context_menus(cx);
                    this.open_connection_group_delete_confirm(group_id_delete.clone(), cx);
                }),
            ));

        self.connection_menu_overlay("connection-group-context-menu", state.x, state.y, menu, cx)
    }

    /// Right-click on the list background.
    ///
    /// Tauri hangs the multi-selection actions off this menu (and off the header
    /// `⋮`), which is why the panel needs no always-visible selection toolbar.
    pub(in crate::features) fn connection_list_context_menu_overlay(
        &mut self,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let palette = self.theme_palette();
        let state = self
            .connection_state
            .list
            .active_list_context_menu()
            .unwrap_or(ConnectionListContextMenuState {
                x: px(24.),
                y: px(24.),
            });
        let selected = self.selected_connections();
        let selected_count = selected.len();
        let move_submenu_open = self.connection_state.list.move_submenu_is_open();
        let move_ids = selected
            .into_iter()
            .map(|connection| connection.id)
            .collect::<Vec<_>>();
        let connect_label = if selected_count > 1 {
            format!(
                "{} ({selected_count})",
                self.tr("savedConnections.connectSelected")
            )
        } else {
            self.tr("savedConnections.connect").to_string()
        };

        let menu = div()
            .flex()
            .flex_col()
            .when(selected_count > 0, |this| {
                this.child(menu_item_with_icon(
                    palette,
                    "connection-list-context-connect",
                    "icons/conn/connect.svg",
                    connect_label,
                    false,
                    cx.listener(|this, _, window, cx| {
                        this.close_connection_context_menus(cx);
                        this.start_selected_saved_connections(window, cx);
                    }),
                ))
                .child(menu_item_with_icon(
                    palette,
                    "connection-list-context-copy",
                    "icons/copy.svg",
                    self.tr("savedConnections.copySelected"),
                    false,
                    cx.listener(|this, _, _, cx| {
                        this.close_connection_context_menus(cx);
                        this.copy_selected_connections(cx);
                    }),
                ))
                .child(menu_item_submenu_trigger(
                    palette,
                    "connection-list-context-move",
                    "icons/net/move.svg",
                    self.tr("savedConnections.moveToGroup"),
                    move_submenu_open,
                    cx.listener(|this, _, _, cx| {
                        this.connection_state.list.toggle_move_submenu();
                        cx.notify();
                    }),
                ))
                .child(menu_item_with_icon(
                    palette,
                    "connection-list-context-delete",
                    "icons/net/delete.svg",
                    self.tr("savedConnections.delete"),
                    true,
                    cx.listener(|this, _, _, cx| {
                        this.close_connection_context_menus(cx);
                        this.delete_selected_connections(cx);
                    }),
                ))
                .child(menu_separator(palette))
            })
            .child(menu_item_with_icon(
                palette,
                "connection-list-context-new",
                "icons/conn/add.svg",
                self.tr("savedConnections.newConnection"),
                false,
                cx.listener(|this, _, window, cx| {
                    this.close_connection_context_menus(cx);
                    this.open_connection_editor(None, None, false, window, cx);
                }),
            ))
            .child(menu_item_with_icon(
                palette,
                "connection-list-context-new-folder",
                "icons/fe/new-folder.svg",
                self.tr("savedConnections.newFolder"),
                false,
                cx.listener(|this, _, window, cx| {
                    this.close_connection_context_menus(cx);
                    this.open_connection_group_editor(None, None, window, cx);
                }),
            ))
            .child(menu_separator(palette))
            .child(menu_item_with_icon(
                palette,
                "connection-list-context-import",
                "icons/import.svg",
                self.tr("settings.importConfig"),
                false,
                cx.listener(|this, _, window, cx| {
                    this.close_connection_context_menus(cx);
                    this.open_connection_import_dialog(window, cx);
                }),
            ));

        let submenu = (move_submenu_open && selected_count > 0).then(|| {
            self.connection_move_to_group_submenu("connection-list-context-move-menu", move_ids, cx)
        });
        self.connection_menu_overlay_with_submenu(
            "connection-list-context-menu",
            state.x,
            state.y,
            menu,
            submenu,
            cx,
        )
    }
}
