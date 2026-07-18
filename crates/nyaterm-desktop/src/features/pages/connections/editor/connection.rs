use super::*;

#[path = "connection/local.rs"]
mod local;
#[path = "connection/serial.rs"]
mod serial;
#[path = "connection/ssh.rs"]
mod ssh;
#[path = "connection/telnet.rs"]
mod telnet;

use local::*;
use serial::*;
use ssh::*;
use telnet::*;
impl NyaTermApp {
    pub(in crate::features) fn connection_editor_panel(
        &mut self,
        editor: ConnectionEditorState,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        self.connection_editor_surface(editor, false, cx)
    }

    pub(in crate::features) fn connection_editor_window_view(
        &mut self,
        editor: ConnectionEditorState,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        self.connection_editor_surface(editor, true, cx)
    }

    fn connection_editor_surface(
        &mut self,
        editor: ConnectionEditorState,
        native_window: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let palette = self.theme_palette();
        let language = self.settings.language.clone();
        let title = if editor.id.is_some() {
            self.tr("dialog.editConnection")
        } else {
            self.tr("dialog.newConnection")
        };
        let local_label = self.tr("dialog.localTerminal");
        let serial_label = self.tr("dialog.serial");
        let name_label = self.tr("dialog.connectionName");
        let description_label = self.tr("dialog.description");
        let group_title = self.tr("dialog.group");
        let more_label = self.tr("common.more");
        let cancel_label = self.tr("common.cancel");
        let save_label = self.tr("common.save");
        let none_label = self.tr("dialog.none");
        let group_label = editor
            .group_id
            .as_deref()
            .and_then(|id| {
                self.connection_groups
                    .iter()
                    .find(|group| group.id == id)
                    .map(|group| group.name.clone())
            })
            .unwrap_or_else(|| self.tr("network.ungrouped").to_string());
        let key_label = editor
            .key_id
            .as_deref()
            .and_then(|id| {
                self.connection_ssh_keys
                    .iter()
                    .find(|key| key.id == id)
                    .map(|key| key.name.clone())
            })
            .unwrap_or_else(|| none_label.to_string());
        let otp_label = editor
            .otp_id
            .as_deref()
            .and_then(|id| {
                self.connection_otp_entries
                    .iter()
                    .find(|entry| entry.id == id)
                    .map(|entry| {
                        if entry.issuer.is_empty() {
                            entry.username.clone()
                        } else if entry.username.is_empty() {
                            entry.issuer.clone()
                        } else {
                            format!("{} ({})", entry.issuer, entry.username)
                        }
                    })
            })
            .unwrap_or_else(|| self.tr("dialog.noOtp").to_string());
        let proxy_label = editor
            .proxy_id
            .as_deref()
            .and_then(|id| {
                self.proxies
                    .iter()
                    .find(|proxy| proxy.id == id)
                    .map(|proxy| {
                        if proxy.protocol == "proxycommand" {
                            let command = proxy.command.as_deref().unwrap_or("").trim();
                            if command.is_empty() {
                                format!("{} · {}", proxy.name, proxy.protocol.to_ascii_uppercase())
                            } else {
                                format!(
                                    "{} · {} {}",
                                    proxy.name,
                                    proxy.protocol.to_ascii_uppercase(),
                                    truncate_preview(command, 18)
                                )
                            }
                        } else {
                            format!(
                                "{} · {} {}:{}",
                                proxy.name,
                                proxy.protocol.to_ascii_uppercase(),
                                proxy.host,
                                proxy.port
                            )
                        }
                    })
            })
            .unwrap_or_else(|| self.tr("dialog.noProxy").to_string());
        let jump_label = editor
            .proxy_jump_id
            .as_deref()
            .and_then(|id| {
                self.connections
                    .iter()
                    .find(|connection| connection.id == id)
                    .map(|connection| connection.name.clone())
            })
            .unwrap_or_else(|| self.tr("dialog.noProxyJump").to_string());
        let password_display = if editor.password.is_empty() {
            if editor.existing_password.is_some() {
                "•••••••• (saved)".to_string()
            } else {
                String::new()
            }
        } else {
            "•".repeat(editor.password.chars().count().min(24))
        };

        let card = div()
            .id(SharedString::from("connection-editor-panel"))
            .w_full()
            .when(native_window, |this| this.size_full())
            .when(!native_window, |this| this.max_h(px(640.)))
            .p_4()
            .flex()
            .flex_col()
            .gap_3()
            .overflow_hidden()
            .track_focus(&self.connection_editor_focus)
            .on_click(cx.listener(|this, _, window, cx| {
                window.focus(&this.connection_editor_focus);
                cx.notify();
            }))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                cx.stop_propagation();
                this.handle_connection_editor_key_down(event, window, cx);
            }))
            .when(!native_window, |this| {
                this.child(
                    div()
                        .text_size(px(15.))
                        .font_weight(FontWeight(700.))
                        .text_color(rgb(palette.text))
                        .child(title),
                )
            })
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(kind_chip(
                        palette,
                        "SSH",
                        editor.kind == ConnectionKindTab::Ssh,
                        cx.listener(|this, _, _, cx| {
                            this.set_connection_editor_kind(ConnectionKindTab::Ssh, cx);
                        }),
                    ))
                    .child(kind_chip(
                        palette,
                        local_label,
                        editor.kind == ConnectionKindTab::Local,
                        cx.listener(|this, _, _, cx| {
                            this.set_connection_editor_kind(ConnectionKindTab::Local, cx);
                        }),
                    ))
                    .child(kind_chip(
                        palette,
                        "Telnet",
                        editor.kind == ConnectionKindTab::Telnet,
                        cx.listener(|this, _, _, cx| {
                            this.set_connection_editor_kind(ConnectionKindTab::Telnet, cx);
                        }),
                    ))
                    .child(kind_chip(
                        palette,
                        serial_label,
                        editor.kind == ConnectionKindTab::Serial,
                        cx.listener(|this, _, _, cx| {
                            this.set_connection_editor_kind(ConnectionKindTab::Serial, cx);
                        }),
                    )),
            )
            .child(
                div()
                    .id("connection-editor-scroll")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .pr_1()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(editor_field(
                        palette,
                        "connection-editor-name",
                        name_label,
                        editor.name.clone(),
                        editor.focused_field == ConnectionEditorField::Name,
                        cx.listener(|this, _, window, cx| {
                            this.focus_connection_editor_field(
                                ConnectionEditorField::Name,
                                window,
                                cx,
                            );
                        }),
                    ))
                    .child(editor_field(
                        palette,
                        "connection-editor-description",
                        description_label,
                        editor.description.clone(),
                        editor.focused_field == ConnectionEditorField::Description,
                        cx.listener(|this, _, window, cx| {
                            this.focus_connection_editor_field(
                                ConnectionEditorField::Description,
                                window,
                                cx,
                            );
                        }),
                    ))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_2()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(palette.text_muted))
                                    .child(format!("{group_title} · {group_label}")),
                            )
                            .child(small_button(
                                palette,
                                "connection-editor-group",
                                more_label,
                                cx.listener(|this, _, _, cx| {
                                    this.cycle_connection_editor_group(cx);
                                }),
                            )),
                    )
                    .when(editor.kind == ConnectionKindTab::Ssh, |this| {
                        this.child(connection_editor_ssh_section(
                            palette,
                            &editor,
                            password_display.clone(),
                            key_label.clone(),
                            otp_label.clone(),
                            proxy_label.clone(),
                            jump_label.clone(),
                            &language,
                            cx,
                        ))
                    })
                    .when(editor.kind == ConnectionKindTab::Local, |this| {
                        this.child(connection_editor_local_section(
                            palette, &editor, &language, cx,
                        ))
                    })
                    .when(editor.kind == ConnectionKindTab::Telnet, |this| {
                        this.child(connection_editor_telnet_section(
                            palette, &editor, &language, cx,
                        ))
                    })
                    .when(editor.kind == ConnectionKindTab::Serial, |this| {
                        this.child(connection_editor_serial_section(
                            palette, &editor, &language, cx,
                        ))
                    })
                    .when_some(editor.error.clone(), |this, error| {
                        this.child(
                            div()
                                .text_size(px(12.))
                                .text_color(rgb(palette.danger))
                                .child(error),
                        )
                    }),
            )
            .child(
                div()
                    .mt_1()
                    .pt_3()
                    .border_t_1()
                    .border_color(rgb(palette.border))
                    .flex()
                    .items_center()
                    .justify_end()
                    .gap_2()
                    .child(small_button(
                        palette,
                        "connection-editor-close",
                        cancel_label,
                        cx.listener(|this, _, _, cx| {
                            this.close_connection_editor(cx);
                        }),
                    ))
                    .child(small_button(
                        palette,
                        "connection-editor-save",
                        save_label,
                        cx.listener(|this, _, window, cx| {
                            this.save_connection_editor(window, cx);
                        }),
                    )),
            );
        if native_window {
            div()
                .size_full()
                .overflow_hidden()
                .bg(rgb(palette.bg))
                .child(card)
                .into_any_element()
        } else {
            modal_dialog_shell(palette, "connection-editor-modal", 560., card).into_any_element()
        }
    }
}
