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
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let title = if editor.id.is_some() {
            "Edit Connection"
        } else {
            "New Connection"
        };
        let group_label = editor
            .group_id
            .as_deref()
            .and_then(|id| {
                self.connection_groups
                    .iter()
                    .find(|group| group.id == id)
                    .map(|group| group.name.clone())
            })
            .unwrap_or_else(|| "Ungrouped".to_string());
        let key_label = editor
            .key_id
            .as_deref()
            .and_then(|id| {
                self.connection_ssh_keys
                    .iter()
                    .find(|key| key.id == id)
                    .map(|key| key.name.clone())
            })
            .unwrap_or_else(|| "No key".to_string());
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
            .unwrap_or_else(|| "No OTP".to_string());
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
            .unwrap_or_else(|| "No Proxy".to_string());
        let jump_label = editor
            .proxy_jump_id
            .as_deref()
            .and_then(|id| {
                self.connections
                    .iter()
                    .find(|connection| connection.id == id)
                    .map(|connection| connection.name.clone())
            })
            .unwrap_or_else(|| "No Jump Host".to_string());
        let password_display = if editor.password.is_empty() {
            if editor.existing_password.is_some() {
                "•••••••• (saved)".to_string()
            } else {
                String::new()
            }
        } else {
            "•".repeat(editor.password.chars().count().min(24))
        };

        let save_label = if editor.connect_after_save {
            "Save+Open"
        } else {
            "Save"
        };
        let card = div()
            .id(SharedString::from("connection-editor-panel"))
            .p_4()
            .flex()
            .flex_col()
            .gap_3()
            .max_h(px(640.))
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
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .text_size(px(15.))
                            .font_weight(FontWeight(700.))
                            .text_color(rgb(palette.text))
                            .child(title),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(rgb(palette.text_muted))
                            .child("Create or edit a saved session profile."),
                    ),
            )
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
                        "Local",
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
                        "Serial",
                        editor.kind == ConnectionKindTab::Serial,
                        cx.listener(|this, _, _, cx| {
                            this.set_connection_editor_kind(ConnectionKindTab::Serial, cx);
                        }),
                    )),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(editor_field(
                        palette,
                        "connection-editor-name",
                        "Name",
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
                        "Description",
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
                                    .child(format!("Group · {group_label}")),
                            )
                            .child(small_button(
                                palette,
                                "connection-editor-group",
                                "Cycle",
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
                            cx,
                        ))
                    })
                    .when(editor.kind == ConnectionKindTab::Local, |this| {
                        this.child(connection_editor_local_section(palette, &editor, cx))
                    })
                    .when(editor.kind == ConnectionKindTab::Telnet, |this| {
                        this.child(connection_editor_telnet_section(palette, &editor, cx))
                    })
                    .when(editor.kind == ConnectionKindTab::Serial, |this| {
                        this.child(connection_editor_serial_section(palette, &editor, cx))
                    }),
            )
            .when_some(editor.error.clone(), |this, error| {
                this.child(
                    div()
                        .text_size(px(12.))
                        .text_color(rgb(palette.danger))
                        .child(error),
                )
            })
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
                        "Cancel",
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
        modal_dialog_shell(palette, "connection-editor-modal", 560., card)
    }
}
