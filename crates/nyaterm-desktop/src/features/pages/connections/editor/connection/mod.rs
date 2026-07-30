mod local;
mod serial;
mod ssh;
mod telnet;

use std::collections::{HashMap, HashSet};

use gpui::{
    AnyElement, App, Context, Entity, FontWeight, IntoElement, KeyDownEvent, SharedString, div,
    prelude::{
        FluentBuilder, InteractiveElement, ParentElement, StatefulInteractiveElement, Styled,
    },
    px, rgb, rgba,
};
use nyaterm_core::{ConnectionType, Group, SavedConnection, natural_compare, truncate_preview};
use nyaterm_ui::{NyaInput, NyaSelectOption, NyaSelectState};

use self::local::connection_editor_local_section;
use self::serial::connection_editor_serial_section;
use self::ssh::{
    SshConnectionSectionLabels, SshConnectionSectionOptions, connection_editor_ssh_section,
};
use self::telnet::connection_editor_telnet_section;
use super::super::list::{
    ConnectionEditorChoice, ConnectionEditorFields, ConnectionEditorRenderContext,
    ConnectionGroupChoice, connection_editor_group_select, connection_kind_tab, editor_field,
};
use crate::features::selects::NO_SELECTION_VALUE;
use crate::features::{
    CONNECTION_ICON_OPTIONS, DEFAULT_CONNECTION_ICON, NyaTermApp, modal_dialog_shell,
    resolve_connection_icon, themed_icon,
};
use crate::models::{
    ConnectionEditorField, ConnectionEditorSelect, ConnectionEditorState, ConnectionKindTab,
};

#[derive(Clone, Copy)]
struct ConnectionEditorSectionContext<'a> {
    palette: crate::theme::ThemePalette,
    editor: &'a ConnectionEditorState,
    language: &'a str,
    fields: &'a ConnectionEditorFields,
}

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

    fn connection_editor_select_entity(
        &mut self,
        select_key: ConnectionEditorSelect,
        choices: &[ConnectionEditorChoice],
        placeholder: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> Entity<NyaSelectState> {
        let options = choices
            .iter()
            .map(|choice| {
                NyaSelectOption::new(
                    choice
                        .value
                        .clone()
                        .unwrap_or_else(|| NO_SELECTION_VALUE.to_string()),
                    choice.label.clone(),
                )
            })
            .collect::<Vec<_>>();
        let selected_value = choices.iter().find(|choice| choice.selected).map(|choice| {
            choice
                .value
                .clone()
                .unwrap_or_else(|| NO_SELECTION_VALUE.to_string())
        });
        let select = self.select_entity(
            connection_editor_select_id(select_key),
            options,
            selected_value,
            false,
            cx,
        );
        select.update(cx, |select, cx| select.set_placeholder(placeholder, cx));
        select
    }

    fn connection_editor_group_select_entity(
        &mut self,
        choices: &[ConnectionGroupChoice],
        cx: &mut Context<Self>,
    ) -> Entity<NyaSelectState> {
        let options = choices
            .iter()
            .map(|choice| {
                NyaSelectOption::new(
                    choice
                        .value
                        .clone()
                        .unwrap_or_else(|| NO_SELECTION_VALUE.to_string()),
                    format!("{}{}", "  ".repeat(choice.depth), choice.label),
                )
            })
            .collect::<Vec<_>>();
        let selected_value = choices.iter().find(|choice| choice.selected).map(|choice| {
            choice
                .value
                .clone()
                .unwrap_or_else(|| NO_SELECTION_VALUE.to_string())
        });
        self.select_entity(
            "connection-editor-group-select",
            options,
            selected_value,
            false,
            cx,
        )
    }

    fn connection_editor_surface(
        &mut self,
        editor: ConnectionEditorState,
        native_window: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let palette = self.theme_palette();
        let language = self.settings.summary().language.clone();
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
        let icon_auto_detect_label = self.tr("dialog.iconAutoDetect");
        let icon_auto_detect_hint = self.tr("dialog.iconAutoDetectTooltip");
        let icon_auto_detect = editor.icon_auto_detect;
        let mut group_options = vec![ConnectionGroupChoice {
            value: None,
            label: none_label.to_string(),
            depth: 0,
            selected: editor.group_id.is_none() && editor.pending_group_name.is_none(),
        }];
        group_options.extend(
            ordered_connection_groups(self.connection_state.groups())
                .into_iter()
                .map(|(group, depth)| {
                    let selected = editor.group_id.as_deref() == Some(group.id.as_str());
                    ConnectionGroupChoice {
                        value: Some(group.id),
                        label: group.name,
                        depth,
                        selected,
                    }
                }),
        );
        let group_parent_id = if editor.pending_group_name.is_some() {
            editor.pending_group_parent_id.as_deref()
        } else {
            editor.group_id.as_deref()
        };
        let group_parent_hint = group_parent_id
            .and_then(|id| connection_group_path_label(self.connection_state.groups(), id))
            .map(|path| {
                self.tr("dialog.newGroupParentHint")
                    .replace("{{group}}", &path)
            })
            .unwrap_or_else(|| self.tr("dialog.newGroupRootHint").to_string());
        let key_label = editor
            .key_id
            .as_deref()
            .and_then(|id| {
                self.security
                    .ssh_keys()
                    .iter()
                    .find(|key| key.id == id)
                    .map(|key| key.name.clone())
            })
            .unwrap_or_else(|| none_label.to_string());
        let password_label = editor
            .password_id
            .as_deref()
            .and_then(|id| {
                self.security
                    .passwords()
                    .iter()
                    .find(|password| password.id == id)
                    .map(|password| password.name.clone())
            })
            .unwrap_or_else(|| self.tr("dialog.selectPassword").to_string());
        let otp_label = editor
            .otp_id
            .as_deref()
            .and_then(|id| {
                self.security
                    .otp_entries()
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
                self.tunnel_state
                    .proxies()
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
                self.connection_state
                    .connections()
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
            self.security
                .ssh_keys()
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
        password_options.extend(self.security.passwords().iter().map(|password| {
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
        otp_options.extend(self.security.otp_entries().iter().map(|entry| {
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
        proxy_options.extend(self.tunnel_state.proxies().iter().map(|proxy| {
            ConnectionEditorChoice {
                value: Some(proxy.id.clone()),
                label: proxy.name.clone(),
                selected: editor.proxy_id.as_deref() == Some(proxy.id.as_str()),
            }
        }));
        let mut jump_options = vec![ConnectionEditorChoice {
            value: None,
            label: self.tr("dialog.noProxyJump").to_string(),
            selected: editor.proxy_jump_id.is_none(),
        }];
        jump_options.extend(
            self.connection_state
                .connections()
                .iter()
                .filter(|connection| matches!(connection.config, ConnectionType::Ssh { .. }))
                .filter(|connection| editor.id.as_deref() != Some(connection.id.as_str()))
                .filter(|connection| {
                    !connection_proxy_jump_would_cycle(
                        self.connection_state.connections(),
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
        let telnet_enter_options = [
            ("crlf", "CRLF (\\r\\n)"),
            ("cr", "CR (\\r)"),
            ("lf", "LF (\\n)"),
        ]
        .into_iter()
        .map(|(value, label)| ConnectionEditorChoice {
            value: Some(value.to_string()),
            label: label.to_string(),
            selected: editor.telnet_enter_mode == value,
        })
        .collect::<Vec<_>>();
        let mut serial_port_options = Vec::new();
        if !editor.serial_port.is_empty()
            && !self
                .connection_state
                .serial_ports()
                .contains(&editor.serial_port)
        {
            serial_port_options.push(ConnectionEditorChoice {
                value: Some(editor.serial_port.clone()),
                label: editor.serial_port.clone(),
                selected: true,
            });
        }
        serial_port_options.extend(self.connection_state.serial_ports().iter().map(|port| {
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
        let shell_label = match editor.shell_path.as_str() {
            "powershell.exe" => self.tr("dialog.shellPowerShell"),
            "cmd.exe" => self.tr("dialog.shellCmd"),
            "bash" => self.tr("dialog.shellBash"),
            "wsl.exe" => self.tr("dialog.shellWsl"),
            "wt.exe" => self.tr("dialog.shellWindowsTerminal"),
            _ => self.tr("dialog.shellCustom"),
        };
        let shell_options = [
            ("powershell.exe", self.tr("dialog.shellPowerShell")),
            ("cmd.exe", self.tr("dialog.shellCmd")),
            ("bash", self.tr("dialog.shellBash")),
            ("wsl.exe", self.tr("dialog.shellWsl")),
            ("wt.exe", self.tr("dialog.shellWindowsTerminal")),
        ]
        .into_iter()
        .map(|(value, label)| ConnectionEditorChoice {
            value: Some(value.to_string()),
            label: label.to_string(),
            selected: editor.shell_path == value,
        })
        .collect::<Vec<_>>();
        let mut selects = HashMap::new();
        selects.insert(
            ConnectionEditorSelect::Group,
            self.connection_editor_group_select_entity(&group_options, cx),
        );
        for (select_key, choices, placeholder) in [
            (
                ConnectionEditorSelect::SavedPassword,
                password_options.as_slice(),
                password_label.clone(),
            ),
            (
                ConnectionEditorSelect::SshKey,
                key_options.as_slice(),
                key_label.clone(),
            ),
            (
                ConnectionEditorSelect::Otp,
                otp_options.as_slice(),
                otp_label.clone(),
            ),
            (
                ConnectionEditorSelect::Proxy,
                proxy_options.as_slice(),
                proxy_label.clone(),
            ),
            (
                ConnectionEditorSelect::ProxyJump,
                jump_options.as_slice(),
                jump_label.clone(),
            ),
            (
                ConnectionEditorSelect::Backspace,
                backspace_options.as_slice(),
                String::new(),
            ),
            (
                ConnectionEditorSelect::TelnetEnterMode,
                telnet_enter_options.as_slice(),
                "CR (\\r)".to_string(),
            ),
            (
                ConnectionEditorSelect::Shell,
                shell_options.as_slice(),
                shell_label.to_string(),
            ),
            (
                ConnectionEditorSelect::SerialPort,
                serial_port_options.as_slice(),
                self.tr("dialog.selectSerialPort").to_string(),
            ),
            (
                ConnectionEditorSelect::BaudRate,
                baud_options.as_slice(),
                self.tr("dialog.customBaudRate").to_string(),
            ),
            (
                ConnectionEditorSelect::DataBits,
                data_bits_options.as_slice(),
                String::new(),
            ),
            (
                ConnectionEditorSelect::Parity,
                parity_options.as_slice(),
                String::new(),
            ),
            (
                ConnectionEditorSelect::StopBits,
                stop_bits_options.as_slice(),
                String::new(),
            ),
        ] {
            selects.insert(
                select_key,
                self.connection_editor_select_entity(select_key, choices, placeholder, cx),
            );
        }
        let fields =
            ConnectionEditorFields::new(self.connection_state.editor_fields().clone(), selects);
        let icon_key = editor.icon.as_deref();
        let icon_def = resolve_connection_icon(icon_key, editor.kind.label());
        let icon_picker_open = self.connection_state.editor_icon_picker_is_open();
        let icon_picker_bg = if native_window {
            rgb(palette.surface)
        } else {
            self.shell_surface_color(palette.surface)
        };
        let validation_error = self.connection_editor_validation_error(&editor);
        let save_enabled = validation_error.is_none();
        let editor_focus = self.connection_state.editor_focus_handle();
        let section_context = ConnectionEditorSectionContext {
            palette,
            editor: &editor,
            language: &language,
            fields: &fields,
        };
        let mut icon_grid = div().grid().grid_cols(7).gap_1();
        for icon_key in CONNECTION_ICON_OPTIONS.iter().copied() {
            let icon = resolve_connection_icon(Some(icon_key), editor.kind.label());
            let selected = editor.icon.as_deref().unwrap_or(DEFAULT_CONNECTION_ICON) == icon_key;
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
                    .child(themed_icon(palette, icon, false, 16.))
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
                    .child(themed_icon(palette, icon_def, false, 17.))
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
                        .bg(icon_picker_bg)
                        .shadow_lg()
                        .child(icon_grid)
                        // Only SSH reports a remote system, so the toggle would be
                        // inert on the other kinds.
                        .when(editor.kind == ConnectionKindTab::Ssh, |this| {
                            this.child(
                                div()
                                    .id("connection-editor-icon-auto-detect")
                                    .mt_2()
                                    .pt_2()
                                    .border_t_1()
                                    .border_color(rgb(palette.border))
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .gap_2()
                                    .cursor_pointer()
                                    .child(
                                        div()
                                            .min_w_0()
                                            .flex()
                                            .flex_col()
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(rgb(palette.text))
                                                    .child(icon_auto_detect_label),
                                            )
                                            .child(
                                                div()
                                                    .text_size(px(10.))
                                                    .text_color(rgb(palette.text_dimmed))
                                                    .child(icon_auto_detect_hint),
                                            ),
                                    )
                                    .child(crate::features::pages::settings::settings_switch(
                                        palette,
                                        "connection-editor-icon-auto-detect-switch",
                                        icon_auto_detect,
                                        cx.listener(move |this, _, _, cx| {
                                            this.set_connection_editor_icon_auto_detect(
                                                !icon_auto_detect,
                                                cx,
                                            );
                                        }),
                                    ))
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.set_connection_editor_icon_auto_detect(
                                            !icon_auto_detect,
                                            cx,
                                        );
                                    })),
                            )
                        }),
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
            // No blanket focus grab here: it existed to keep the old label-div
            // inputs "focused", and would now steal focus back from whichever
            // field the pointer just landed on, since click follows mouse-down.
            .track_focus(&editor_focus)
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
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .items_end()
                            .gap_3()
                            .child(icon_picker)
                            .child(div().min_w(px(192.)).flex_1().child(editor_field(
                                palette,
                                name_label,
                                ConnectionEditorField::Name,
                                &fields,
                                cx,
                            )))
                            .child(div().min_w(px(192.)).max_w(px(288.)).flex_1().child(
                                connection_editor_group_select(
                                    ConnectionEditorRenderContext {
                                        palette,
                                        fields: &fields,
                                        cx,
                                    },
                                    group_title,
                                    group_parent_hint,
                                ),
                            )),
                    )
                    .when(editor.kind == ConnectionKindTab::Ssh, |this| {
                        this.child(connection_editor_ssh_section(
                            section_context,
                            SshConnectionSectionLabels {
                                otp: otp_label.clone(),
                                proxy: proxy_label.clone(),
                                jump: jump_label.clone(),
                            },
                            SshConnectionSectionOptions { auth: auth_options },
                            cx,
                        ))
                    })
                    .when(editor.kind == ConnectionKindTab::Local, |this| {
                        this.child(connection_editor_local_section(section_context, cx))
                    })
                    .when(editor.kind == ConnectionKindTab::Telnet, |this| {
                        this.child(connection_editor_telnet_section(section_context, cx))
                    })
                    .when(editor.kind == ConnectionKindTab::Serial, |this| {
                        this.child(connection_editor_serial_section(section_context, cx))
                    })
                    .child(connection_description_field(
                        palette,
                        description_label,
                        &fields,
                        cx,
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
                    .justify_between()
                    .gap_3()
                    .child(
                        div()
                            .min_w_0()
                            .flex_1()
                            .text_size(px(10.))
                            .text_color(rgb(palette.danger))
                            .child(validation_error.unwrap_or_default()),
                    )
                    .child(
                        div()
                            .flex_none()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(connection_editor_footer_button(
                                palette,
                                "connection-editor-close",
                                cancel_label,
                                false,
                                true,
                                cx.listener(|this, _, _, cx| {
                                    this.close_connection_editor(cx);
                                }),
                            ))
                            .child(connection_editor_footer_button(
                                palette,
                                "connection-editor-save",
                                save_label,
                                true,
                                save_enabled,
                                cx.listener(|this, _, window, cx| {
                                    this.save_connection_editor(window, cx);
                                }),
                            )),
                    ),
            );
        if native_window {
            div()
                .size_full()
                .overflow_hidden()
                .bg(rgb(palette.bg))
                .child(card)
                .into_any_element()
        } else {
            modal_dialog_shell(
                palette,
                self.shell_surface_color(palette.bg),
                "connection-editor-modal",
                560.,
                card,
            )
            .into_any_element()
        }
    }
}

fn connection_editor_select_id(select: ConnectionEditorSelect) -> &'static str {
    match select {
        ConnectionEditorSelect::Authentication => {
            unreachable!("authentication uses a radio group")
        }
        ConnectionEditorSelect::Group => "connection-editor-group-select",
        ConnectionEditorSelect::SavedPassword => "connection-editor-saved-password",
        ConnectionEditorSelect::SshKey => "connection-editor-ssh-key",
        ConnectionEditorSelect::Otp => "connection-editor-otp",
        ConnectionEditorSelect::Proxy => "connection-editor-proxy",
        ConnectionEditorSelect::ProxyJump => "connection-editor-proxy-jump",
        ConnectionEditorSelect::Backspace => "connection-editor-backspace",
        ConnectionEditorSelect::TelnetEnterMode => "connection-editor-telnet-enter-mode",
        ConnectionEditorSelect::Shell => "connection-editor-shell",
        ConnectionEditorSelect::SerialPort => "connection-editor-serial-port",
        ConnectionEditorSelect::BaudRate => "connection-editor-baud-rate",
        ConnectionEditorSelect::DataBits => "connection-editor-data-bits",
        ConnectionEditorSelect::Parity => "connection-editor-parity",
        ConnectionEditorSelect::StopBits => "connection-editor-stop-bits",
    }
}

fn connection_group_path_label(groups: &[Group], group_id: &str) -> Option<String> {
    let mut parts = Vec::new();
    let mut next = Some(group_id);
    let mut seen = HashSet::new();
    while let Some(id) = next {
        if !seen.insert(id.to_string()) {
            break;
        }
        let group = groups.iter().find(|group| group.id == id)?;
        parts.push(group.name.clone());
        next = group.parent_id.as_deref();
    }
    parts.reverse();
    Some(parts.join(" / "))
}

pub(in crate::features::pages::connections) fn ordered_connection_groups(
    groups: &[Group],
) -> Vec<(Group, usize)> {
    let group_ids = groups
        .iter()
        .map(|group| group.id.clone())
        .collect::<HashSet<_>>();
    let mut children = HashMap::<Option<String>, Vec<Group>>::new();
    for group in groups {
        let parent_id = group
            .parent_id
            .clone()
            .filter(|parent_id| parent_id != &group.id && group_ids.contains(parent_id));
        children.entry(parent_id).or_default().push(group.clone());
    }
    for siblings in children.values_mut() {
        siblings.sort_by(|left, right| {
            left.sort_order
                .cmp(&right.sort_order)
                // Same rule as the panel tree, so the dropdown and the list agree.
                .then_with(|| natural_compare(&left.name, &right.name))
                .then_with(|| left.id.cmp(&right.id))
        });
    }

    fn append_group(
        group: Group,
        depth: usize,
        children: &HashMap<Option<String>, Vec<Group>>,
        visited: &mut HashSet<String>,
        ordered: &mut Vec<(Group, usize)>,
    ) {
        if !visited.insert(group.id.clone()) {
            return;
        }
        let group_id = group.id.clone();
        ordered.push((group, depth));
        for child in children.get(&Some(group_id)).cloned().unwrap_or_default() {
            append_group(child, depth + 1, children, visited, ordered);
        }
    }

    let mut ordered = Vec::with_capacity(groups.len());
    let mut visited = HashSet::new();
    for group in children.get(&None).cloned().unwrap_or_default() {
        append_group(group, 0, &children, &mut visited, &mut ordered);
    }
    let mut remaining = groups
        .iter()
        .filter(|group| !visited.contains(&group.id))
        .cloned()
        .collect::<Vec<_>>();
    remaining.sort_by(|left, right| {
        left.sort_order
            .cmp(&right.sort_order)
            .then_with(|| {
                left.name
                    .to_ascii_lowercase()
                    .cmp(&right.name.to_ascii_lowercase())
            })
            .then_with(|| left.id.cmp(&right.id))
    });
    for group in remaining {
        append_group(group, 0, &children, &mut visited, &mut ordered);
    }
    ordered
}

fn connection_description_field(
    palette: crate::theme::ThemePalette,
    label: &'static str,
    fields: &ConnectionEditorFields,
    cx: &App,
) -> impl IntoElement {
    let entity = fields.get(&ConnectionEditorField::Description);
    let handle = entity.map(|field| field.read(cx).focus_handle());
    let focused = entity.is_some_and(|field| field.read(cx).has_focus());
    div()
        .flex()
        .flex_col()
        .gap_1()
        .child(
            div()
                .text_xs()
                .text_color(rgb(palette.text_muted))
                .child(label),
        )
        .child(
            div()
                .id("connection-editor-description")
                // Fixed: in a flex column the box would otherwise shrink to
                // whatever space the form had left, cutting a row in half.
                .h(px(72.))
                .flex_none()
                .min_w_0()
                .overflow_hidden()
                .rounded_sm()
                .border_1()
                .border_color(if focused {
                    rgb(palette.primary)
                } else {
                    rgb(palette.border)
                })
                .bg(rgb(palette.input))
                .px_3()
                .py_2()
                .cursor_text()
                .when_some(handle, |this, handle| {
                    this.on_mouse_down(gpui::MouseButton::Left, move |_, window, _| {
                        window.focus(&handle);
                    })
                })
                .children(entity.map(NyaInput::new)),
        )
}

fn connection_editor_footer_button(
    palette: crate::theme::ThemePalette,
    id: &'static str,
    label: &'static str,
    primary: bool,
    enabled: bool,
    on_click: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    let background = if primary {
        palette.primary
    } else {
        palette.surface_elevated
    };
    let text = if primary {
        palette.on_primary
    } else {
        palette.text
    };
    div()
        .id(id)
        .h(px(28.))
        .px_3()
        .flex()
        .items_center()
        .rounded_sm()
        .border_1()
        .border_color(if primary {
            rgb(palette.primary)
        } else {
            rgb(palette.border)
        })
        .bg(rgb(background))
        .text_color(rgb(text))
        .text_xs()
        .opacity(if enabled { 1.0 } else { 0.45 })
        .when(enabled, |this| {
            this.cursor_pointer()
                .hover(move |this| this.opacity(0.86))
                .on_click(on_click)
        })
        .child(label)
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

#[cfg(test)]
mod tests {
    use nyaterm_core::Group;

    use super::ordered_connection_groups;

    fn group(id: &str, name: &str, parent_id: Option<&str>, sort_order: i32) -> Group {
        Group {
            id: id.to_string(),
            name: name.to_string(),
            parent_id: parent_id.map(ToOwned::to_owned),
            sort_order,
            created_at_ms: None,
            updated_at_ms: None,
        }
    }

    #[test]
    fn group_picker_orders_tree_and_keeps_orphans_visible() {
        let groups = vec![
            group("child", "Child", Some("parent"), 0),
            group("orphan", "Orphan", Some("missing"), 2),
            group("parent", "Parent", None, 1),
        ];

        let ordered = ordered_connection_groups(&groups)
            .into_iter()
            .map(|(group, depth)| (group.id, depth))
            .collect::<Vec<_>>();

        assert_eq!(
            ordered,
            vec![
                ("parent".to_string(), 0),
                ("child".to_string(), 1),
                ("orphan".to_string(), 0),
            ]
        );
    }
}
