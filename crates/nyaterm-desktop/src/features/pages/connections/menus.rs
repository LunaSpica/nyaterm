use super::*;

fn connection_menu_position(
    x: f32,
    y: f32,
    menu_width: f32,
    preferred_height: f32,
    viewport_width: f32,
    viewport_height: f32,
) -> (f32, f32, f32) {
    let margin = 8.;
    let max_height = (viewport_height - margin * 2.).max(80.);
    let height = preferred_height.min(max_height);
    let max_x = (viewport_width - menu_width - margin).max(margin);
    let max_y = (viewport_height - height - margin).max(margin);
    (
        x.clamp(margin, max_x),
        y.clamp(margin, max_y),
        max_height,
    )
}

impl NyaTermApp {
    pub(in crate::features) fn connection_context_menu_overlay(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let state = self
            .connection_context_menu
            .clone()
            .unwrap_or(ConnectionContextMenuState {
                connection_id: String::new(),
                x: px(24.),
                y: px(24.),
            });
        let connection = self
            .connections
            .iter()
            .find(|connection| connection.id == state.connection_id)
            .cloned();
        let selected_count = self.selected_connections().len();
        let connect_label = if selected_count > 1
            && connection
                .as_ref()
                .is_some_and(|conn| self.selected_connection_ids.contains(&conn.id))
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
        let (viewport_w, viewport_h) = self.last_viewport_size;
        let (menu_x, menu_y, menu_max_h) = connection_menu_position(
            f32::from(state.x),
            f32::from(state.y),
            180.,
            166.,
            viewport_w,
            viewport_h,
        );

        div()
            .id(SharedString::from("connection-context-menu-overlay"))
            .absolute()
            .top_0()
            .bottom_0()
            .left_0()
            .right_0()
            .on_click(cx.listener(|this, _, _, cx| {
                this.close_connection_context_menus(cx);
            }))
            .child(
                div()
                    .id(SharedString::from("connection-context-menu"))
                    .absolute()
                    .top(px(menu_y))
                    .left(px(menu_x))
                    .w(px(180.))
                    .max_h(px(menu_max_h))
                    .overflow_y_scroll()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(palette.border))
                    .bg(self.shell_surface_color(palette.bg))
                    .shadow_lg()
                    .py_1()
                    .on_click(|_, _, cx| cx.stop_propagation())
                    .child(menu_item_owned(
                        palette,
                        "connection-context-connect",
                        connect_label,
                        cx.listener(move |this, _, window, cx| {
                            this.close_connection_context_menus(cx);
                            let selected = this.selected_connections();
                            if selected.len() > 1
                                && selected.iter().any(|conn| conn.id == connection_id)
                            {
                                this.start_selected_saved_connections(window, cx);
                            } else if let Some(connection) = connection_for_connect.clone() {
                                this.start_saved_connection(connection, window, cx);
                            }
                        }),
                    ))
                    .child(menu_item(
                        palette,
                        "connection-context-edit",
                        self.tr("savedConnections.edit"),
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
                    .child(menu_item(
                        palette,
                        "connection-context-rename",
                        self.tr("savedConnections.rename"),
                        cx.listener(move |this, _, window, cx| {
                            this.close_connection_context_menus(cx);
                            this.rename_connection(connection_for_rename.clone(), window, cx);
                        }),
                    ))
                    .child(menu_item(
                        palette,
                        "connection-context-copy",
                        if selected_count > 1 {
                            self.tr("savedConnections.copySelected")
                        } else {
                            self.tr("savedConnections.copy")
                        },
                        cx.listener(move |this, _, _, cx| {
                            this.close_connection_context_menus(cx);
                            if this.selected_connections().len() > 1 {
                                this.copy_selected_connections(cx);
                            } else {
                                this.copy_connection_by_id(connection_for_copy.clone(), cx);
                            }
                        }),
                    ))
                    .child(menu_separator(palette))
                    .child(menu_item(
                        palette,
                        "connection-context-delete",
                        if selected_count > 1 {
                            self.tr("savedConnections.delete")
                        } else {
                            self.tr("savedConnections.delete")
                        },
                        cx.listener(move |this, _, _, cx| {
                            this.close_connection_context_menus(cx);
                            if this.selected_connections().len() > 1 {
                                this.delete_selected_connections(cx);
                            } else {
                                this.open_connection_delete_confirm(
                                    connection_for_delete.clone(),
                                    cx,
                                );
                            }
                        }),
                    )),
            )
    }

    pub(in crate::features) fn connection_group_context_menu_overlay(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let state =
            self.connection_group_context_menu
                .clone()
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
        let (viewport_w, viewport_h) = self.last_viewport_size;
        let group_menu_height = if total_in_group > 0 { 166. } else { 129. };
        let (menu_x, menu_y, menu_max_h) = connection_menu_position(
            f32::from(state.x),
            f32::from(state.y),
            180.,
            group_menu_height,
            viewport_w,
            viewport_h,
        );

        div()
            .id(SharedString::from("connection-group-context-menu-overlay"))
            .absolute()
            .top_0()
            .bottom_0()
            .left_0()
            .right_0()
            .on_click(cx.listener(|this, _, _, cx| {
                this.close_connection_context_menus(cx);
            }))
            .child(
                div()
                    .id(SharedString::from("connection-group-context-menu"))
                    .absolute()
                    .top(px(menu_y))
                    .left(px(menu_x))
                    .w(px(180.))
                    .max_h(px(menu_max_h))
                    .overflow_y_scroll()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(palette.border))
                    .bg(self.shell_surface_color(palette.bg))
                    .shadow_lg()
                    .py_1()
                    .on_click(|_, _, cx| cx.stop_propagation())
                    .child(menu_item(
                        palette,
                        "connection-group-context-new",
                        self.tr("savedConnections.newConnection"),
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
                    .child(menu_item(
                        palette,
                        "connection-group-context-folder",
                        self.tr("savedConnections.newFolder"),
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
                        this.child(menu_separator(palette)).child(menu_item(
                            palette,
                            "connection-group-context-open-all",
                            self.tr("savedConnections.openAllConnections"),
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
                    .child(menu_item(
                        palette,
                        "connection-group-context-rename",
                        self.tr("savedConnections.renameFolder"),
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
                    .child(menu_item(
                        palette,
                        "connection-group-context-delete",
                        self.tr("savedConnections.deleteFolder"),
                        cx.listener(move |this, _, _, cx| {
                            this.close_connection_context_menus(cx);
                            this.open_connection_group_delete_confirm(group_id_delete.clone(), cx);
                        }),
                    )),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::connection_menu_position;

    #[test]
    fn menu_position_stays_inside_viewport() {
        assert_eq!(
            connection_menu_position(1240., 780., 180., 166., 1280., 800.),
            (1092., 626., 784.)
        );
        assert_eq!(
            connection_menu_position(240., 180., 180., 166., 200., 120.),
            (12., 8., 104.)
        );
    }
}
