use gpui::{
    AnyElement, App, Bounds, Context, Element, Entity, GlobalElementId, InspectorElementId,
    IntoElement, KeyDownEvent, LayoutId, MouseButton, MouseDownEvent, Pixels, ScrollDelta,
    ScrollWheelEvent, SharedString, Window, div, prelude::*, px, rgb, svg,
};
use nyaterm_core::truncate_preview;
use nyaterm_transport::SftpFileType;
use nyaterm_ui::NyaInput;

use crate::features::{NyaTermApp, TextInputSetup, format_file_size};
use crate::models::TransferBrowserSortColumn;

use super::super::{
    TransferBrowserEntryRowPresentation, TransferBrowserSortHeaderState,
    normalized_transfer_browser_path, sort_header_cell, transfer_browser_entry_row,
    transfer_browser_parent_entry_row, transfer_browser_table_width,
};
use super::helpers::{
    compact_transfer_footer_button, compact_transfer_footer_button_active,
    compact_transfer_toolbar_button, compact_transfer_toolbar_button_active,
    compact_transfer_toolbar_button_enabled, compact_transfer_upload_menu_button,
    transfer_toolbar_divider,
};

const FILE_ROW_PX: f32 = 30.;
const FILE_HEADER_PX: f32 = 28.;
const FILE_OVERSCAN: usize = 8;

fn transfer_browser_viewport_rows(
    viewport_height: f32,
    queue_height: f32,
    measured_table_height: f32,
) -> usize {
    // The first frame uses an estimate so rows can render before prepaint. All
    // later frames use the actual bounds of the table viewport.
    let rows_height = if measured_table_height > 0. {
        (measured_table_height - FILE_HEADER_PX).max(FILE_ROW_PX)
    } else {
        (viewport_height - queue_height.clamp(60., 600.) - 132.).max(FILE_ROW_PX)
    };
    (rows_height / FILE_ROW_PX).floor().max(1.) as usize
}

struct TransferBrowserViewportElement {
    app: Entity<NyaTermApp>,
    child: AnyElement,
}

impl IntoElement for TransferBrowserViewportElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for TransferBrowserViewportElement {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<gpui::ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        (self.child.request_layout(window, cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _state: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let height = f32::from(bounds.size.height).max(0.);
        let app = self.app.clone();
        app.update(cx, |this, cx| {
            if this.transfer.set_browser_viewport_height(height) {
                cx.notify();
            }
        });
        self.child.prepaint(window, cx);
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _state: &mut Self::RequestLayoutState,
        _prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.child.paint(window, cx);
    }
}

impl NyaTermApp {
    pub(in crate::features) fn transfer_browser_view(
        &mut self,
        can_transfer: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let transparent_surface = self.shell_transparent_color(palette.surface);
        let section_header = self.shell_transparent_color(palette.section_header);
        let _selected = self
            .transfer
            .browser_view()
            .selected_remote_path
            .as_deref()
            .map(|path| truncate_preview(path, 56))
            .unwrap_or_else(|| "none".to_string());
        let visible_entries = self.visible_transfer_browser_entries();
        let column_widths = self.transfer.browser_view().column_widths;
        let table_width = transfer_browser_table_width(column_widths);
        let resizing_column = self
            .transfer
            .browser_view()
            .column_resize
            .map(|state| state.column);
        let selected_entries = self.selected_transfer_entries();
        let selected_count = selected_entries.len();
        let total_count = self.transfer.browser_view().entries.len();
        let files_total_size: u64 = self
            .transfer
            .browser_view()
            .entries
            .iter()
            .filter(|entry| entry.file_type != SftpFileType::Directory)
            .map(|entry| entry.size.unwrap_or(0))
            .sum();
        let search_active = !self.transfer.browser_view().search.trim().is_empty();
        let search_expanded = self.transfer.browser_view().search_expanded || search_active;
        let show_hidden_files = self.settings.summary().ui_file_explorer_show_hidden_files;
        let search_input = search_expanded.then(|| {
            let field = self.text_input(
                "transfer.browser.search",
                &self.transfer.browser_view().search.clone(),
                TextInputSetup::placeholder(self.tr("fileExplorer.searchPlaceholder")),
                cx,
            );
            let focus = field.read(cx).focus_handle();
            div()
                .id(SharedString::from("transfer-browser-search"))
                .h_full()
                .flex_1()
                .min_w_0()
                .px_1()
                .flex()
                .items_center()
                .cursor_text()
                .on_mouse_down(MouseButton::Left, move |_, window, _| {
                    window.focus(&focus);
                })
                .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                    if event.keystroke.key == "escape" {
                        cx.stop_propagation();
                        this.clear_or_close_transfer_browser_search(window, cx);
                    }
                }))
                .child(
                    div()
                        .min_w_0()
                        .flex_1()
                        .font_family(crate::features::gpui_code_font_family())
                        .text_size(px(12.))
                        .text_color(rgb(palette.text))
                        .child(NyaInput::new(&field)),
                )
                .into_any_element()
        });
        let current_browser_path =
            normalized_transfer_browser_path(self.transfer.browser_view().path);
        let has_parent_entry =
            can_transfer && current_browser_path != "/" && current_browser_path != ".";
        let auto_sync_cwd = self.transfer_browser_auto_sync_cwd_enabled();
        let cwd_tracking_available = self.active_transfer_browser_connection_id().is_some();
        let mut rows = div().flex().flex_col();
        if !can_transfer {
            rows = rows.child(
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .px_4()
                    .py_8()
                    .gap_1()
                    .child(
                        svg()
                            .size(px(28.))
                            .flex_none()
                            .path("icons/conn/folder.svg")
                            .text_color(rgb(palette.border)),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(rgb(palette.text_muted))
                            .child(if self.session.active_id().is_some() {
                                self.tr("fileExplorer.unsupportedSession")
                            } else {
                                self.tr("fileExplorer.connectToSession")
                            }),
                    )
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(rgb(palette.text_dimmed))
                            .child(if self.session.active_id().is_some() {
                                self.tr("fileExplorer.unsupportedSessionDesc")
                            } else {
                                self.tr("fileExplorer.connectToSession")
                            }),
                    ),
            );
        } else if self.transfer.browser_view().loading {
            rows = rows.child(
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .px_4()
                    .py_8()
                    .text_size(px(12.))
                    .text_color(rgb(palette.text_dimmed))
                    .child(self.tr("fileExplorer.loading")),
            );
        } else if self.transfer.browser_view().entries.is_empty() {
            if has_parent_entry && self.transfer.browser_view().error.is_none() {
                rows = rows.child(transfer_browser_parent_entry_row(
                    palette,
                    current_browser_path.clone(),
                    column_widths,
                    cx,
                ));
            } else {
                rows = rows.child(
                    div()
                        .flex()
                        .flex_col()
                        .items_center()
                        .justify_center()
                        .px_4()
                        .py_8()
                        .gap_1()
                        .child(
                            if let Some(error) = self.transfer.browser_view().error.as_deref() {
                                div()
                                    .text_size(px(12.))
                                    .text_color(rgb(palette.danger))
                                    .child(truncate_preview(error, 120))
                            } else {
                                div()
                                    .text_size(px(12.))
                                    .text_color(rgb(palette.text_muted))
                                    .child(self.tr("fileExplorer.emptyDirectory"))
                            },
                        ),
                );
            }
        } else if visible_entries.is_empty() {
            rows = rows.child(
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .px_4()
                    .py_8()
                    .text_size(px(11.))
                    .text_color(rgb(palette.text_dimmed))
                    .child(self.tr("fileExplorer.noSearchResults")),
            );
        } else {
            // Tauri File Explorer virtual list (30px rows and overscan).
            let viewport_rows = transfer_browser_viewport_rows(
                self.shell.viewport_size().1,
                self.transfer.panel_height(),
                self.transfer.browser_view().viewport_height,
            );
            let parent_count = usize::from(has_parent_entry);
            let total_entries = visible_entries.len() + parent_count;
            let window_capacity = viewport_rows + FILE_OVERSCAN * 2;
            let max_offset = total_entries.saturating_sub(viewport_rows.min(total_entries));
            if self.transfer.browser_view().list_offset > max_offset {
                self.transfer.set_browser_list_offset(max_offset);
            }
            let scroll_row = self.transfer.browser_view().list_offset.min(max_offset);
            // This panel uses a manual wheel offset and clips vertically, so the
            // virtual window must be laid out at the top of the viewport. Spacer
            // padding would only work with a real scroll container.
            let window_start = scroll_row;
            let window_end = (window_start + window_capacity).min(total_entries);
            // Only one row renames at a time, so its box is built once here: the
            // loop below borrows `self` and cannot create one.
            let renaming = self.transfer.rename_dialog().cloned();
            let mut rename_input = renaming.as_ref().map(|state| {
                self.text_input_box(
                    format!("transfer.rename.{}", state.old_path),
                    &state.value,
                    TextInputSetup::placeholder("Remote name"),
                    cx,
                )
                .into_any_element()
            });
            for index in window_start..window_end {
                if has_parent_entry && index == 0 {
                    rows = rows.child(transfer_browser_parent_entry_row(
                        palette,
                        current_browser_path.clone(),
                        column_widths,
                        cx,
                    ));
                } else if let Some(entry) = visible_entries.get(index.saturating_sub(parent_count))
                {
                    rows = rows.child(transfer_browser_entry_row(
                        TransferBrowserEntryRowPresentation {
                            palette,
                            entry: entry.clone(),
                            selected_remote_path: self
                                .transfer
                                .browser_view()
                                .selected_remote_path
                                .clone(),
                            selected_remote_paths: self
                                .transfer
                                .browser_view()
                                .selected_remote_paths,
                            column_widths,
                            rename_state: renaming.clone(),
                            rename_input: (renaming.as_ref().map(|state| state.old_path.as_str())
                                == Some(entry.path.as_str()))
                            .then(|| rename_input.take())
                            .flatten(),
                        },
                        cx,
                    ));
                }
            }
        }

        div()
            .id(SharedString::from("transfer-browser-panel"))
            .size_full()
            .flex()
            .flex_col()
            .overflow_hidden()
            .bg(transparent_surface)
            .track_focus(self.transfer.browser_view().focus)
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                this.handle_transfer_browser_key_down(event, window, cx);
            }))
            .when(can_transfer, |this| {
                this.child(
                    div()
                        .relative()
                        .h(px(36.))
                        .px_1()
                        .border_b_1()
                        .border_color(rgb(palette.border))
                        .bg(section_header)
                        .flex()
                        .items_center()
                        .gap(px(2.))
                        .child(compact_transfer_toolbar_button(
                            palette,
                            "transfer-browser-new-file",
                            "icons/fe/new-file.svg",
                            self.tr("fileExplorer.newFile"),
                            cx.listener(|this, _, window, cx| {
                                this.open_transfer_new_file_dialog(window, cx);
                            }),
                        ))
                        .child(compact_transfer_toolbar_button(
                            palette,
                            "transfer-browser-new-folder",
                            "icons/fe/new-folder.svg",
                            self.tr("fileExplorer.newFolder"),
                            cx.listener(|this, _, window, cx| {
                                this.open_transfer_new_folder_dialog(window, cx);
                            }),
                        ))
                        .child(transfer_toolbar_divider(palette))
                        .child(compact_transfer_upload_menu_button(
                            palette,
                            self.tr("fileExplorer.upload"),
                            cx,
                        ))
                        .child(compact_transfer_toolbar_button_enabled(
                            palette,
                            "transfer-browser-download-selected",
                            "icons/fe/download.svg",
                            self.tr("fileExplorer.downloadSelected"),
                            selected_count > 0,
                            cx.listener(|this, _, window, cx| {
                                this.start_selected_sftp_download_jobs(window, cx);
                            }),
                        ))
                        .child(compact_transfer_toolbar_button_enabled(
                            palette,
                            "transfer-browser-delete-selected",
                            "icons/fe/delete.svg",
                            self.tr("fileExplorer.delete"),
                            selected_count > 0,
                            cx.listener(|this, _, window, cx| {
                                this.open_selected_transfer_delete_dialog(window, cx);
                            }),
                        ))
                        .child(transfer_toolbar_divider(palette))
                        .child(compact_transfer_toolbar_button(
                            palette,
                            "transfer-browser-go-up",
                            "icons/fe/up.svg",
                            self.tr("fileExplorer.goUp"),
                            cx.listener(|this, _, window, cx| {
                                this.open_transfer_parent_directory(window, cx);
                            }),
                        ))
                        .child(compact_transfer_toolbar_button(
                            palette,
                            "transfer-browser-refresh",
                            "icons/fe/refresh.svg",
                            self.tr("fileExplorer.refresh"),
                            cx.listener(|this, _, window, cx| {
                                this.refresh_transfer_browser(window, cx);
                            }),
                        ))
                        .child(div().flex_1())
                        .child(compact_transfer_toolbar_button_active(
                            palette,
                            "transfer-browser-expand-search",
                            "icons/fe/search.svg",
                            self.tr("fileExplorer.search"),
                            search_active || search_expanded,
                            cx.listener(|this, _, window, cx| {
                                this.focus_transfer_browser_search(None, window, cx);
                            }),
                        ))
                        .child(compact_transfer_toolbar_button_active(
                            palette,
                            "transfer-browser-toggle-hidden-files",
                            "icons/eye.svg",
                            if show_hidden_files {
                                self.tr("fileExplorer.hideHiddenFiles")
                            } else {
                                self.tr("fileExplorer.showHiddenFiles")
                            },
                            show_hidden_files,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_transfer_browser_hidden_files(cx);
                            }),
                        ))
                        .when(search_expanded, |toolbar| {
                            toolbar.child(
                                div()
                                    .id(SharedString::from("transfer-browser-search-overlay"))
                                    .absolute()
                                    .top(px(2.))
                                    .bottom(px(2.))
                                    .left(px(4.))
                                    .right(px(4.))
                                    .rounded_md()
                                    .border_1()
                                    .border_color(rgb(0x388bfd))
                                    .bg(transparent_surface)
                                    .px_1()
                                    .flex()
                                    .items_center()
                                    .gap_1()
                                    .child(
                                        svg()
                                            .size(px(16.))
                                            .flex_none()
                                            .path("icons/fe/search.svg")
                                            .text_color(rgb(palette.link)),
                                    )
                                    .children(search_input)
                                    .child(
                                        div()
                                            .id(SharedString::from("transfer-browser-clear-search"))
                                            .size(px(20.))
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .rounded_sm()
                                            .text_size(px(12.))
                                            .text_color(rgb(palette.text_muted))
                                            .cursor_pointer()
                                            .hover(|this| {
                                                this.bg(rgb(palette.surface_elevated))
                                                    .text_color(rgb(palette.text))
                                            })
                                            .child(
                                                svg()
                                                    .size(px(13.))
                                                    .path("icons/window/close.svg")
                                                    .text_color(rgb(palette.text_muted)),
                                            )
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                this.clear_or_close_transfer_browser_search(
                                                    window, cx,
                                                );
                                            })),
                                    ),
                            )
                        }),
                )
                .child(self.transfer_browser_path_row(current_browser_path.clone(), cx))
            })
            .child(TransferBrowserViewportElement {
                app: cx.entity(),
                child: div()
                    .id(SharedString::from("transfer-browser-table-scroll"))
                    .flex_1()
                    .min_h_0()
                    .min_w_0()
                    .overflow_x_scroll()
                    .overflow_y_hidden()
                    .scrollbar_width(px(8.))
                    .on_scroll_wheel(cx.listener(|this, event: &ScrollWheelEvent, _, cx| {
                        let current_path =
                            normalized_transfer_browser_path(this.transfer.browser_view().path);
                        let parent_count = usize::from(current_path != "/" && current_path != ".");
                        let total = this.visible_transfer_browser_entries().len() + parent_count;
                        let viewport_rows = transfer_browser_viewport_rows(
                            this.shell.viewport_size().1,
                            this.transfer.panel_height(),
                            this.transfer.browser_view().viewport_height,
                        );
                        let max_offset = total.saturating_sub(viewport_rows.min(total));
                        if max_offset == 0 {
                            return;
                        }
                        let delta_rows = match event.delta {
                            ScrollDelta::Lines(delta) => delta.y,
                            ScrollDelta::Pixels(delta) => f32::from(delta.y) / FILE_ROW_PX,
                        };
                        let next = (this.transfer.browser_view().list_offset as f32 - delta_rows)
                            .round()
                            .clamp(0., max_offset as f32)
                            as usize;
                        if next != this.transfer.browser_view().list_offset {
                            this.transfer.set_browser_list_offset(next);
                            cx.stop_propagation();
                            cx.notify();
                        }
                    }))
                    .on_mouse_down(
                        MouseButton::Right,
                        cx.listener(|this, event: &MouseDownEvent, window, cx| {
                            this.open_transfer_browser_current_context_menu(event, window, cx);
                        }),
                    )
                    .child(
                        div()
                            .min_w(table_width)
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_0()
                                    .child(sort_header_cell(
                                        palette,
                                        TransferBrowserSortColumn::Name,
                                        column_widths.name,
                                        TransferBrowserSortHeaderState {
                                            header_bg: section_header,
                                            active_column: self.transfer.browser_view().sort_column,
                                            direction: self.transfer.browser_view().sort_direction,
                                            resizing_column,
                                        },
                                        cx,
                                    ))
                                    .child(sort_header_cell(
                                        palette,
                                        TransferBrowserSortColumn::Modified,
                                        column_widths.modified,
                                        TransferBrowserSortHeaderState {
                                            header_bg: section_header,
                                            active_column: self.transfer.browser_view().sort_column,
                                            direction: self.transfer.browser_view().sort_direction,
                                            resizing_column,
                                        },
                                        cx,
                                    ))
                                    .child(sort_header_cell(
                                        palette,
                                        TransferBrowserSortColumn::Size,
                                        column_widths.size,
                                        TransferBrowserSortHeaderState {
                                            header_bg: section_header,
                                            active_column: self.transfer.browser_view().sort_column,
                                            direction: self.transfer.browser_view().sort_direction,
                                            resizing_column,
                                        },
                                        cx,
                                    ))
                                    .child(sort_header_cell(
                                        palette,
                                        TransferBrowserSortColumn::Permissions,
                                        column_widths.permissions,
                                        TransferBrowserSortHeaderState {
                                            header_bg: section_header,
                                            active_column: self.transfer.browser_view().sort_column,
                                            direction: self.transfer.browser_view().sort_direction,
                                            resizing_column,
                                        },
                                        cx,
                                    ))
                                    .child(sort_header_cell(
                                        palette,
                                        TransferBrowserSortColumn::Owner,
                                        column_widths.owner,
                                        TransferBrowserSortHeaderState {
                                            header_bg: section_header,
                                            active_column: self.transfer.browser_view().sort_column,
                                            direction: self.transfer.browser_view().sort_direction,
                                            resizing_column,
                                        },
                                        cx,
                                    ))
                                    .child(sort_header_cell(
                                        palette,
                                        TransferBrowserSortColumn::Group,
                                        column_widths.group,
                                        TransferBrowserSortHeaderState {
                                            header_bg: section_header,
                                            active_column: self.transfer.browser_view().sort_column,
                                            direction: self.transfer.browser_view().sort_direction,
                                            resizing_column,
                                        },
                                        cx,
                                    )),
                            )
                            .child(rows),
                    )
                    .into_any_element(),
            })
            // Tauri FileExplorer footer: totals left, cwd sync / send icons right.
            .child(
                div()
                    .h(px(28.))
                    .flex_none()
                    .px_2()
                    .border_t_1()
                    .border_color(rgb(palette.border))
                    .bg(transparent_surface)
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .child(
                        div()
                            .min_w_0()
                            .flex()
                            .items_center()
                            .gap_3()
                            .text_size(px(11.))
                            .text_color(rgb(palette.text_muted))
                            .when(
                                !self.transfer.browser_view().loading
                                    && self.transfer.browser_view().error.is_none()
                                    && total_count > 0,
                                |this| {
                                    this.child(
                                        self.tr("fileExplorer.totalItems")
                                            .replace("{{count}}", &total_count.to_string()),
                                    )
                                    .when(files_total_size > 0, |this| {
                                        this.child(format_file_size(Some(files_total_size)))
                                    })
                                },
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_0()
                            .child(compact_transfer_footer_button(
                                palette,
                                "transfer-browser-footer-sync-cwd",
                                "icons/fe/sync.svg",
                                if cwd_tracking_available {
                                    self.tr("fileExplorer.syncTerminalPath")
                                } else {
                                    self.tr("fileExplorer.cwdTrackingUnavailable")
                                },
                                cwd_tracking_available,
                                cx.listener(|this, _, window, cx| {
                                    this.start_transfer_sync_cwd_job(window, cx);
                                }),
                            ))
                            .child(compact_transfer_footer_button_active(
                                palette,
                                "transfer-browser-footer-auto-sync",
                                "icons/fe/sync.svg",
                                if cwd_tracking_available {
                                    self.tr("fileExplorer.autoSyncTerminalPath")
                                } else {
                                    self.tr("fileExplorer.cwdTrackingUnavailable")
                                },
                                auto_sync_cwd,
                                cwd_tracking_available,
                                cx.listener(|this, _, window, cx| {
                                    this.toggle_transfer_browser_auto_sync_cwd(window, cx);
                                }),
                            ))
                            .child(compact_transfer_footer_button(
                                palette,
                                "transfer-browser-footer-send-path",
                                "icons/fe/paste.svg",
                                self.tr("fileExplorer.sendToTerminal"),
                                true,
                                cx.listener(|this, _, _, cx| {
                                    this.send_current_transfer_browser_path_to_terminal(cx);
                                }),
                            )),
                    ),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::transfer_browser_viewport_rows;

    #[test]
    fn viewport_rows_follow_window_and_queue_height() {
        assert_eq!(transfer_browser_viewport_rows(800., 240., 0.), 14);
        assert_eq!(transfer_browser_viewport_rows(1080., 240., 0.), 23);
        assert_eq!(transfer_browser_viewport_rows(800., 60., 0.), 20);
    }

    #[test]
    fn viewport_rows_keep_one_row_when_queue_consumes_the_panel() {
        assert_eq!(transfer_browser_viewport_rows(400., 600., 0.), 1);
    }

    #[test]
    fn viewport_rows_prefer_measured_table_height() {
        assert_eq!(transfer_browser_viewport_rows(800., 240., 444.), 13);
        assert_eq!(transfer_browser_viewport_rows(800., 240., 84.), 1);
    }
}
