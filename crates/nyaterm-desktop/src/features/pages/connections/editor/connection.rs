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
        let cancel_label = self.tr("common.cancel");
        let save_label = self.tr("common.save");
        let none_label = self.tr("dialog.none");
        let icon_label = self.tr("dialog.icon");
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
        let mut group_options = vec![ConnectionEditorChoice {
            value: None,
            label: none_label.to_string(),
            selected: editor.group_id.is_none(),
        }];
        group_options.extend(
            self.connection_groups
                .iter()
                .map(|group| ConnectionEditorChoice {
                    value: Some(group.id.clone()),
                    label: group.name.clone(),
                    selected: editor.group_id.as_deref() == Some(group.id.as_str()),
                }),
        );
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
        let password_label = editor
            .password_id
            .as_deref()
            .and_then(|id| {
                self.connection_saved_passwords
                    .iter()
                    .find(|password| password.id == id)
                    .map(|password| password.name.clone())
            })
            .unwrap_or_else(|| self.tr("dialog.selectPassword").to_string());
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
        let auth_options = [
            ("none", self.tr("dialog.noAuthentication")),
            ("password", self.tr("dialog.password")),
            ("key", self.tr("dialog.privateKey")),
        ]
        .into_iter()
        .map(|(value, label)| ConnectionEditorChoice {
            value: Some(value.to_string()),
            label: label.to_string(),
            selected: editor.auth_mode == value,
        })
        .collect::<Vec<_>>();
        let mut key_options = vec![ConnectionEditorChoice {
            value: None,
            label: none_label.to_string(),
            selected: editor.key_id.is_none(),
        }];
        key_options.extend(
            self.connection_ssh_keys
                .iter()
                .map(|key| ConnectionEditorChoice {
                    value: Some(key.id.clone()),
                    label: key.name.clone(),
                    selected: editor.key_id.as_deref() == Some(key.id.as_str()),
                }),
        );
        let mut password_options = vec![ConnectionEditorChoice {
            value: None,
            label: none_label.to_string(),
            selected: editor.password_id.is_none(),
        }];
        password_options.extend(self.connection_saved_passwords.iter().map(|password| {
            ConnectionEditorChoice {
                value: Some(password.id.clone()),
                label: password.name.clone(),
                selected: editor.password_id.as_deref() == Some(password.id.as_str()),
            }
        }));
        let mut otp_options = vec![ConnectionEditorChoice {
            value: None,
            label: self.tr("dialog.noOtp").to_string(),
            selected: editor.otp_id.is_none(),
        }];
        otp_options.extend(self.connection_otp_entries.iter().map(|entry| {
            let label = if entry.issuer.is_empty() {
                entry.username.clone()
            } else if entry.username.is_empty() {
                entry.issuer.clone()
            } else {
                format!("{} ({})", entry.issuer, entry.username)
            };
            ConnectionEditorChoice {
                value: Some(entry.id.clone()),
                label,
                selected: editor.otp_id.as_deref() == Some(entry.id.as_str()),
            }
        }));
        let mut proxy_options = vec![ConnectionEditorChoice {
            value: None,
            label: self.tr("dialog.noProxy").to_string(),
            selected: editor.proxy_id.is_none(),
        }];
        proxy_options.extend(self.proxies.iter().map(|proxy| ConnectionEditorChoice {
            value: Some(proxy.id.clone()),
            label: proxy.name.clone(),
            selected: editor.proxy_id.as_deref() == Some(proxy.id.as_str()),
        }));
        let mut jump_options = vec![ConnectionEditorChoice {
            value: None,
            label: self.tr("dialog.noProxyJump").to_string(),
            selected: editor.proxy_jump_id.is_none(),
        }];
        jump_options.extend(
            self.connections
                .iter()
                .filter(|connection| matches!(connection.config, ConnectionType::Ssh { .. }))
                .filter(|connection| editor.id.as_deref() != Some(connection.id.as_str()))
                .filter(|connection| {
                    !connection_proxy_jump_would_cycle(
                        &self.connections,
                        editor.id.as_deref(),
                        connection,
                    )
                })
                .map(|connection| ConnectionEditorChoice {
                    value: Some(connection.id.clone()),
                    label: connection.name.clone(),
                    selected: editor.proxy_jump_id.as_deref() == Some(connection.id.as_str()),
                }),
        );
        let backspace_options = [
            ("del", self.tr("dialog.backspaceDel")),
            ("ctrl-h", self.tr("dialog.backspaceCtrlH")),
        ]
        .into_iter()
        .map(|(value, label)| ConnectionEditorChoice {
            value: Some(value.to_string()),
            label: label.to_string(),
            selected: match value {
                "ctrl-h" => matches!(editor.backspace_mode.as_str(), "ctrl-h" | "bs" | "ctrl_h"),
                _ => !matches!(editor.backspace_mode.as_str(), "ctrl-h" | "bs" | "ctrl_h"),
            },
        })
        .collect::<Vec<_>>();
        let mut serial_port_options = Vec::new();
        if !editor.serial_port.is_empty()
            && !self.connection_serial_ports.contains(&editor.serial_port)
        {
            serial_port_options.push(ConnectionEditorChoice {
                value: Some(editor.serial_port.clone()),
                label: editor.serial_port.clone(),
                selected: true,
            });
        }
        serial_port_options.extend(self.connection_serial_ports.iter().map(|port| {
            ConnectionEditorChoice {
                value: Some(port.clone()),
                label: port.clone(),
                selected: editor.serial_port == *port,
            }
        }));
        let baud_options = [
            "9600", "19200", "38400", "57600", "115200", "230400", "460800", "921600",
        ]
        .into_iter()
        .map(|value| ConnectionEditorChoice {
            value: Some(value.to_string()),
            label: value.to_string(),
            selected: editor.baud_rate == value,
        })
        .collect::<Vec<_>>();
        let data_bits_options = ["5", "6", "7", "8"]
            .into_iter()
            .map(|value| ConnectionEditorChoice {
                value: Some(value.to_string()),
                label: value.to_string(),
                selected: editor.data_bits == value,
            })
            .collect::<Vec<_>>();
        let parity_options = [
            ("none", self.tr("dialog.parityNone")),
            ("odd", self.tr("dialog.parityOdd")),
            ("even", self.tr("dialog.parityEven")),
            ("mark", self.tr("dialog.parityMark")),
            ("space", self.tr("dialog.paritySpace")),
        ]
        .into_iter()
        .map(|(value, label)| ConnectionEditorChoice {
            value: Some(value.to_string()),
            label: label.to_string(),
            selected: editor.parity == value,
        })
        .collect::<Vec<_>>();
        let stop_bits_options = ["1", "1.5", "2"]
            .into_iter()
            .map(|value| ConnectionEditorChoice {
                value: Some(value.to_string()),
                label: value.to_string(),
                selected: editor.stop_bits == value,
            })
            .collect::<Vec<_>>();
        let password_display = if editor.password.is_empty() {
            if editor.existing_password.is_some() {
                self.tr("dialog.passwordAlreadySet").to_string()
            } else {
                String::new()
            }
        } else {
            "•".repeat(editor.password.chars().count().min(24))
        };
        let icon_key = editor.icon.as_deref();
        let icon_def = resolve_connection_icon(icon_key, editor.kind.label());
        let icon_picker_open = self.connection_icon_picker_open;
        let mut icon_grid = div().grid().grid_cols(7).gap_1();
        for icon_key in CONNECTION_ICON_OPTIONS {
            let icon = resolve_connection_icon(Some(icon_key), editor.kind.label());
            let selected = editor.icon.as_deref().unwrap_or("server") == *icon_key;
            icon_grid = icon_grid.child(
                div()
                    .id(SharedString::from(format!("connection-icon-{icon_key}")))
                    .size(px(28.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_sm()
                    .cursor_pointer()
                    .bg(if selected {
                        rgba((palette.primary << 8) | 0x26)
                    } else {
                        rgba(0x00000000)
                    })
                    .border_1()
                    .border_color(if selected {
                        rgb(palette.primary)
                    } else {
                        rgba(0x00000000)
                    })
                    .hover(|this| this.bg(rgb(palette.hover)))
                    .child(
                        svg()
                            .size(px(16.))
                            .path(icon.path)
                            .text_color(rgb(icon.color)),
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.set_connection_editor_icon(Some(icon_key), cx);
                    })),
            );
        }
        let icon_picker = div()
            .relative()
            .flex_none()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(palette.text_muted))
                    .child(icon_label),
            )
            .child(
                div()
                    .id("connection-editor-icon-trigger")
                    .size(px(32.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_sm()
                    .border_1()
                    .border_color(if icon_picker_open {
                        rgb(palette.primary)
                    } else {
                        rgb(palette.border)
                    })
                    .bg(rgb(palette.input))
                    .cursor_pointer()
                    .hover(|this| this.bg(rgb(palette.hover)))
                    .child(
                        svg()
                            .size(px(17.))
                            .path(icon_def.path)
                            .text_color(rgb(icon_def.color)),
                    )
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.toggle_connection_icon_picker(cx);
                    })),
            )
            .when(icon_picker_open, |this| {
                this.child(
                    div()
                        .absolute()
                        .left_0()
                        .top(px(52.))
                        .w(px(232.))
                        .p_2()
                        .rounded_md()
                        .border_1()
                        .border_color(rgb(palette.border))
                        .bg(rgb(palette.surface_elevated))
                        .shadow_lg()
                        .child(icon_grid),
                )
            });

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
                    .h(px(32.))
                    .p_1()
                    .flex()
                    .items_center()
                    .gap_1()
                    .rounded_md()
                    .bg(rgb(palette.surface_elevated))
                    .child(connection_kind_tab(
                        palette,
                        "SSH",
                        editor.kind == ConnectionKindTab::Ssh,
                        cx.listener(|this, _, _, cx| {
                            this.set_connection_editor_kind(ConnectionKindTab::Ssh, cx);
                        }),
                    ))
                    .child(connection_kind_tab(
                        palette,
                        local_label,
                        editor.kind == ConnectionKindTab::Local,
                        cx.listener(|this, _, _, cx| {
                            this.set_connection_editor_kind(ConnectionKindTab::Local, cx);
                        }),
                    ))
                    .child(connection_kind_tab(
                        palette,
                        "Telnet",
                        editor.kind == ConnectionKindTab::Telnet,
                        cx.listener(|this, _, _, cx| {
                            this.set_connection_editor_kind(ConnectionKindTab::Telnet, cx);
                        }),
                    ))
                    .child(connection_kind_tab(
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
                    .child(div().flex().items_end().gap_2().child(icon_picker).child(
                        div().min_w_0().flex_1().child(editor_field(
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
                        )),
                    ))
                    .child(connection_editor_select(
                        palette,
                        "connection-editor-group",
                        group_title,
                        group_label,
                        ConnectionEditorMenu::Group,
                        self.connection_editor_menu == Some(ConnectionEditorMenu::Group),
                        group_options,
                        cx,
                    ))
                    .when(editor.kind == ConnectionKindTab::Ssh, |this| {
                        this.child(connection_editor_ssh_section(
                            palette,
                            &editor,
                            password_display.clone(),
                            password_label.clone(),
                            key_label.clone(),
                            otp_label.clone(),
                            proxy_label.clone(),
                            jump_label.clone(),
                            auth_options,
                            password_options,
                            key_options,
                            otp_options,
                            proxy_options,
                            jump_options,
                            backspace_options.clone(),
                            self.connection_editor_menu,
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
                            palette,
                            &editor,
                            backspace_options.clone(),
                            self.connection_editor_menu,
                            &language,
                            cx,
                        ))
                    })
                    .when(editor.kind == ConnectionKindTab::Serial, |this| {
                        this.child(connection_editor_serial_section(
                            palette,
                            &editor,
                            serial_port_options,
                            baud_options,
                            data_bits_options,
                            parity_options,
                            stop_bits_options,
                            backspace_options,
                            self.connection_editor_menu,
                            &language,
                            cx,
                        ))
                    })
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

fn connection_proxy_jump_would_cycle(
    connections: &[SavedConnection],
    current_id: Option<&str>,
    candidate: &SavedConnection,
) -> bool {
    let Some(current_id) = current_id else {
        return false;
    };
    let mut seen = HashSet::new();
    let mut next_id = Some(candidate.id.clone());
    while let Some(id) = next_id {
        if id == current_id || !seen.insert(id.clone()) {
            return true;
        }
        next_id = connections
            .iter()
            .find(|connection| connection.id == id)
            .and_then(|connection| connection.network.as_ref())
            .and_then(|network| network.proxy_jump_id.clone());
    }
    false
}
