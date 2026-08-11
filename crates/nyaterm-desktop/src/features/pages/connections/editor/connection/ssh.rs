use gpui::{
    Context, FontWeight, IntoElement, SharedString, div,
    prelude::{
        FluentBuilder, InteractiveElement, ParentElement, StatefulInteractiveElement, Styled,
    },
    px, rgb, rgba, svg,
};

use nyaterm_core::truncate_preview;
use nyaterm_ui::{NyaTabItem, NyaTabs};

use crate::features::{ConnectionEditorToggle, NyaTermApp};
use crate::models::{
    ConnectionEditorAdvancedTab, ConnectionEditorField, ConnectionEditorPasswordSource,
    ConnectionEditorSelect,
};

use super::super::super::list::{
    ConnectionEditorChoice, ConnectionEditorRenderContext, connection_editor_select, editor_field,
    editor_stepper_field, required, toggle_chip,
};

use super::ConnectionEditorSectionContext;

pub(super) struct SshConnectionSectionLabels {
    pub(super) otp: String,
    pub(super) proxy: String,
    pub(super) jump: String,
}

pub(super) struct SshConnectionSectionOptions {
    pub(super) auth: Vec<ConnectionEditorChoice>,
}

fn ssh_advanced_content(
    palette: crate::theme::ThemePalette,
    title: &'static str,
    description: impl Into<SharedString>,
    content: impl IntoElement,
) -> impl IntoElement {
    let description = description.into();
    div()
        .rounded_md()
        .border_1()
        .border_color(rgb(palette.border))
        .bg(rgb(palette.bg))
        .p_3()
        .flex()
        .flex_col()
        .gap_3()
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .text_xs()
                        .font_weight(FontWeight(600.))
                        .text_color(rgb(palette.text))
                        .child(title),
                )
                .child(
                    div()
                        .text_size(px(10.))
                        .text_color(rgb(palette.text_muted))
                        .child(description),
                ),
        )
        .child(content)
}

pub(super) fn connection_editor_ssh_section(
    section: ConnectionEditorSectionContext<'_>,
    labels: SshConnectionSectionLabels,
    options: SshConnectionSectionOptions,
    cx: &mut Context<NyaTermApp>,
) -> gpui::Div {
    let ConnectionEditorSectionContext {
        palette,
        editor,
        language,
        fields,
    } = section;
    let SshConnectionSectionLabels {
        otp: otp_label,
        proxy: proxy_label,
        jump: jump_label,
    } = labels;
    let SshConnectionSectionOptions { auth: auth_options } = options;
    let tr = |key: &'static str| crate::i18n::text(language, key);

    let auth_tab_options = auth_options
        .iter()
        .filter_map(|option| option.value.as_ref().map(|value| (value, option)))
        .collect::<Vec<_>>();
    let auth_tab_values: Vec<String> = auth_tab_options
        .iter()
        .map(|(value, _)| (*value).clone())
        .collect::<Vec<_>>();
    let auth_tabs = NyaTabs::new("connection-authentication-tabs")
        .items(
            auth_tab_options
                .iter()
                .map(|(_, option)| NyaTabItem::new(option.label.clone())),
        )
        .selected_index_if_visible(
            auth_tab_options
                .iter()
                .position(|(_, option)| option.selected),
        )
        .on_select(cx.listener(move |this, index: &usize, _, cx| {
            let Some(value) = auth_tab_values.get(*index) else {
                return;
            };
            this.set_connection_editor_select_value(
                ConnectionEditorSelect::Authentication,
                Some(value.as_str()),
                cx,
            );
        }));

    let advanced_tabs = NyaTabs::new("connection-advanced-network-tabs")
        .items([
            NyaTabItem::new(tr("dialog.proxySelect")),
            NyaTabItem::new(tr("dialog.proxyJump")),
            NyaTabItem::new(tr("dialog.twoFactorAuth")),
        ])
        .selected_index(match editor.advanced_network_tab {
            ConnectionEditorAdvancedTab::Proxy => 0,
            ConnectionEditorAdvancedTab::JumpHost => 1,
            ConnectionEditorAdvancedTab::TwoFactor => 2,
            _ => 0,
        })
        .on_select(cx.listener(|this, index, _, cx| {
            let tab = match *index {
                0 => ConnectionEditorAdvancedTab::Proxy,
                1 => ConnectionEditorAdvancedTab::JumpHost,
                _ => ConnectionEditorAdvancedTab::TwoFactor,
            };
            this.set_connection_editor_advanced_tab(tab, cx);
        }));

    let behavior_tabs = NyaTabs::new("connection-advanced-behavior-tabs")
        .items([
            NyaTabItem::new(tr("dialog.commandExecution")),
            NyaTabItem::new(tr("dialog.encodingSettings")),
            NyaTabItem::new("SFTP"),
            NyaTabItem::new(tr("dialog.x11Forwarding")),
            NyaTabItem::new(tr("dialog.backspaceMode")),
            NyaTabItem::new(tr("dialog.sshAlgorithms")),
        ])
        .selected_index(match editor.advanced_behavior_tab {
            ConnectionEditorAdvancedTab::PostLogin => 0,
            ConnectionEditorAdvancedTab::Terminal => 1,
            ConnectionEditorAdvancedTab::Sftp => 2,
            ConnectionEditorAdvancedTab::X11 => 3,
            ConnectionEditorAdvancedTab::Backspace => 4,
            ConnectionEditorAdvancedTab::Algorithms => 5,
            _ => 0,
        })
        .on_select(cx.listener(|this, index, _, cx| {
            let tab = match *index {
                0 => ConnectionEditorAdvancedTab::PostLogin,
                1 => ConnectionEditorAdvancedTab::Terminal,
                2 => ConnectionEditorAdvancedTab::Sftp,
                3 => ConnectionEditorAdvancedTab::X11,
                4 => ConnectionEditorAdvancedTab::Backspace,
                _ => ConnectionEditorAdvancedTab::Algorithms,
            };
            this.set_connection_editor_advanced_tab(tab, cx);
        }));

    let password_source_tabs = NyaTabs::new("connection-password-source-tabs")
        .items([
            NyaTabItem::new(tr("dialog.askWhenConnecting")),
            NyaTabItem::new(tr("dialog.directPassword")),
            NyaTabItem::new(tr("dialog.savedPassword")),
        ])
        .selected_index(match editor.password_source {
            ConnectionEditorPasswordSource::Ask => 0,
            ConnectionEditorPasswordSource::Direct => 1,
            ConnectionEditorPasswordSource::Saved => 2,
        })
        .on_select(cx.listener(|this, index, _, cx| {
            let source = match *index {
                0 => ConnectionEditorPasswordSource::Ask,
                1 => ConnectionEditorPasswordSource::Direct,
                _ => ConnectionEditorPasswordSource::Saved,
            };
            this.set_connection_editor_password_source(source, cx);
        }));

    div()
        .flex()
        .flex_col()
        .gap_3()
        .child(editor_field(
            palette,
            required(tr("dialog.host")),
            ConnectionEditorField::Host,
            fields,
            cx,
        ))
        .child(editor_stepper_field(
            palette,
            required(tr("dialog.port")),
            ConnectionEditorField::Port,
            fields,
            cx,
        ))
        .child(editor_field(
            palette,
            required(tr("dialog.username")),
            ConnectionEditorField::Username,
            fields,
            cx,
        ))
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(palette.text_muted))
                        .child(tr("dialog.authentication")),
                )
                .child(auth_tabs),
        )
        .when(editor.auth_mode == "none", |this| {
            this.child(
                div()
                    .px_3()
                    .py_2()
                    .rounded_md()
                    .bg(rgba((palette.accent << 8) | 0x18))
                    .text_size(px(10.))
                    .text_color(rgb(palette.text_muted))
                    .child(tr("dialog.noAuthenticationDescription")),
            )
        })
        .when(editor.auth_mode == "password", |this| {
            this.child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(palette.text_muted))
                            .child(tr("dialog.passwordSource")),
                    )
                    .child(password_source_tabs)
                    .when(
                        editor.password_source == ConnectionEditorPasswordSource::Ask,
                        |this| {
                            this.child(
                                div()
                                    .px_3()
                                    .py_2()
                                    .rounded_md()
                                    .bg(rgba((palette.accent << 8) | 0x18))
                                    .text_size(px(10.))
                                    .text_color(rgb(palette.text_muted))
                                    .child(tr("dialog.askPasswordDescription")),
                            )
                        },
                    )
                    .when(
                        editor.password_source == ConnectionEditorPasswordSource::Direct,
                        |this| {
                            this.child(editor_field(
                                palette,
                                tr("dialog.password"),
                                ConnectionEditorField::Password,
                                fields,
                                cx,
                            ))
                        },
                    )
                    .when(
                        editor.password_source == ConnectionEditorPasswordSource::Saved,
                        |this| {
                            this.child(connection_editor_select(
                                ConnectionEditorRenderContext {
                                    palette,
                                    fields,
                                    cx,
                                },
                                "connection-editor-saved-password",
                                tr("dialog.savedPassword"),
                                ConnectionEditorSelect::SavedPassword,
                            ))
                        },
                    ),
            )
        })
        .when(
            editor.auth_mode == "key" || editor.auth_mode == "certificate",
            |this| {
                this.child(connection_editor_select(
                    ConnectionEditorRenderContext {
                        palette,
                        fields,
                        cx,
                    },
                    "connection-editor-key",
                    tr("dialog.privateKey"),
                    ConnectionEditorSelect::SshKey,
                ))
            },
        )
        .child(
            div()
                .id("connection-editor-advanced-toggle")
                .h(px(28.))
                .flex()
                .items_center()
                .gap_2()
                .text_xs()
                .text_color(rgb(palette.text_muted))
                .cursor_pointer()
                .hover(|this| this.text_color(rgb(palette.text)))
                .child(
                    svg()
                        .size(px(14.))
                        .flex_none()
                        .path(if editor.advanced_open {
                            "icons/chevron-down.svg"
                        } else {
                            "icons/fe/forward.svg"
                        })
                        .text_color(rgb(palette.text_muted)),
                )
                .child(tr("dialog.advancedConfig"))
                .on_click(cx.listener(|this, _, _, cx| {
                    this.toggle_connection_editor_flag(ConnectionEditorToggle::Advanced, cx);
                })),
        )
        .when(editor.advanced_open, |this| {
            this.child(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(advanced_tabs)
                    .when(
                        editor.advanced_network_tab == ConnectionEditorAdvancedTab::Proxy,
                        |this| {
                            this.child(ssh_advanced_content(
                                palette,
                                tr("dialog.proxySelect"),
                                truncate_preview(&proxy_label, 48),
                                connection_editor_select(
                                    ConnectionEditorRenderContext {
                                        palette,
                                        fields,
                                        cx,
                                    },
                                    "connection-editor-proxy",
                                    tr("dialog.proxySelect"),
                                    ConnectionEditorSelect::Proxy,
                                ),
                            ))
                        },
                    )
                    .when(
                        editor.advanced_network_tab == ConnectionEditorAdvancedTab::JumpHost,
                        |this| {
                            this.child(ssh_advanced_content(
                                palette,
                                tr("dialog.proxyJump"),
                                truncate_preview(&jump_label, 48),
                                connection_editor_select(
                                    ConnectionEditorRenderContext {
                                        palette,
                                        fields,
                                        cx,
                                    },
                                    "connection-editor-jump",
                                    tr("dialog.selectProxyJump"),
                                    ConnectionEditorSelect::ProxyJump,
                                ),
                            ))
                        },
                    )
                    .when(
                        editor.advanced_network_tab == ConnectionEditorAdvancedTab::TwoFactor,
                        |this| {
                            this.child(ssh_advanced_content(
                                palette,
                                tr("dialog.twoFactorAuth"),
                                truncate_preview(&otp_label, 36),
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_2()
                                    .child(connection_editor_select(
                                        ConnectionEditorRenderContext {
                                            palette,
                                            fields,
                                            cx,
                                        },
                                        "connection-editor-otp",
                                        tr("dialog.selectOtp"),
                                        ConnectionEditorSelect::Otp,
                                    ))
                                    .child(toggle_chip(
                                        palette,
                                        tr("dialog.autoFillOtp"),
                                        editor.auto_fill_otp,
                                        cx.listener(|this, _, _, cx| {
                                            this.toggle_connection_editor_flag(
                                                ConnectionEditorToggle::AutoFillOtp,
                                                cx,
                                            );
                                        }),
                                    )),
                            ))
                        },
                    )
                    .child(behavior_tabs)
                    .when(
                        editor.advanced_behavior_tab == ConnectionEditorAdvancedTab::PostLogin,
                        |this| {
                            this.child(ssh_advanced_content(
                                palette,
                                tr("dialog.postLoginCommand"),
                                tr("dialog.postLoginCommandDesc"),
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_2()
                                    .child(toggle_chip(
                                        palette,
                                        tr("dialog.enabled"),
                                        editor.post_login_enabled,
                                        cx.listener(|this, _, _, cx| {
                                            this.toggle_connection_editor_flag(
                                                ConnectionEditorToggle::PostLogin,
                                                cx,
                                            );
                                        }),
                                    ))
                                    .when(editor.post_login_enabled, |this| {
                                        this.child(editor_field(
                                            palette,
                                            tr("dialog.postLoginCommandContent"),
                                            ConnectionEditorField::PostLoginCommand,
                                            fields,
                                            cx,
                                        ))
                                        .child(
                                            editor_field(
                                                palette,
                                                tr("dialog.postLoginDelay"),
                                                ConnectionEditorField::PostLoginDelay,
                                                fields,
                                                cx,
                                            ),
                                        )
                                    }),
                            ))
                        },
                    )
                    .when(
                        editor.advanced_behavior_tab == ConnectionEditorAdvancedTab::Terminal,
                        |this| {
                            this.child(ssh_advanced_content(
                                palette,
                                tr("dialog.encodingSettings"),
                                tr("connection.encodingFollowGlobal"),
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_3()
                                    .child(connection_editor_select(
                                        ConnectionEditorRenderContext {
                                            palette,
                                            fields,
                                            cx,
                                        },
                                        "connection-editor-ssh-profile",
                                        tr("dialog.sshProfile"),
                                        ConnectionEditorSelect::SshProfile,
                                    ))
                                    .child(connection_editor_select(
                                        ConnectionEditorRenderContext {
                                            palette,
                                            fields,
                                            cx,
                                        },
                                        "connection-editor-ssh-terminal-type",
                                        tr("dialog.sshTerminalType"),
                                        ConnectionEditorSelect::SshTerminalType,
                                    ))
                                    .child(connection_editor_select(
                                        ConnectionEditorRenderContext {
                                            palette,
                                            fields,
                                            cx,
                                        },
                                        "connection-editor-ssh-encoding",
                                        tr("connection.encoding"),
                                        ConnectionEditorSelect::Encoding,
                                    ))
                                    .when(
                                        editor.ssh_profile
                                            == nyaterm_core::SshProfile::NetworkDevice,
                                        |this| {
                                            this.child(
                                                div()
                                                    .rounded_md()
                                                    .border_1()
                                                    .border_color(rgb(palette.warning))
                                                    .bg(rgba((palette.warning << 8) | 0x18))
                                                    .p_2()
                                                    .text_size(px(10.))
                                                    .text_color(rgb(palette.text_muted))
                                                    .child(tr(
                                                        "dialog.sshNetworkDeviceLimitations",
                                                    )),
                                            )
                                        },
                                    ),
                            ))
                        },
                    )
                    .when(
                        editor.advanced_behavior_tab == ConnectionEditorAdvancedTab::Sftp,
                        |this| {
                            this.child(ssh_advanced_content(
                                palette,
                                tr("dialog.sftpAdvanced"),
                                tr("dialog.sftpAdvancedDesc"),
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_2()
                                    .child(toggle_chip(
                                        palette,
                                        tr("dialog.enabled"),
                                        editor.sftp_enabled,
                                        cx.listener(|this, _, _, cx| {
                                            this.toggle_connection_editor_flag(
                                                ConnectionEditorToggle::SftpEnabled,
                                                cx,
                                            );
                                        }),
                                    ))
                                    .child(connection_editor_select(
                                        ConnectionEditorRenderContext {
                                            palette,
                                            fields,
                                            cx,
                                        },
                                        "connection-editor-sftp-cwd",
                                        tr("dialog.sftpCwdFollowMode"),
                                        ConnectionEditorSelect::SftpCwdFollowMode,
                                    ))
                                    .child(editor_field(
                                        palette,
                                        tr("dialog.sftpShellDetectionTimeout"),
                                        ConnectionEditorField::SftpShellDetectionTimeout,
                                        fields,
                                        cx,
                                    ))
                                    .child(connection_editor_select(
                                        ConnectionEditorRenderContext {
                                            palette,
                                            fields,
                                            cx,
                                        },
                                        "connection-editor-sftp-filename-encoding",
                                        tr("dialog.sftpFilenameEncoding"),
                                        ConnectionEditorSelect::SftpFilenameEncoding,
                                    )),
                            ))
                        },
                    )
                    .when(
                        editor.advanced_behavior_tab == ConnectionEditorAdvancedTab::X11,
                        |this| {
                            this.child(ssh_advanced_content(
                                palette,
                                tr("dialog.x11Forwarding"),
                                tr("dialog.x11ForwardingDesc"),
                                toggle_chip(
                                    palette,
                                    tr("dialog.enabled"),
                                    editor.x11_forwarding,
                                    cx.listener(|this, _, _, cx| {
                                        this.toggle_connection_editor_flag(
                                            ConnectionEditorToggle::X11,
                                            cx,
                                        );
                                    }),
                                ),
                            ))
                        },
                    )
                    .when(
                        editor.advanced_behavior_tab == ConnectionEditorAdvancedTab::Backspace,
                        |this| {
                            this.child(ssh_advanced_content(
                                palette,
                                tr("dialog.backspaceMode"),
                                tr("dialog.sshBackspaceModeDesc"),
                                connection_editor_select(
                                    ConnectionEditorRenderContext {
                                        palette,
                                        fields,
                                        cx,
                                    },
                                    "connection-editor-backspace",
                                    tr("dialog.backspaceMode"),
                                    ConnectionEditorSelect::Backspace,
                                ),
                            ))
                        },
                    )
                    .when(
                        editor.advanced_behavior_tab == ConnectionEditorAdvancedTab::Algorithms,
                        |this| {
                            this.child(ssh_advanced_content(
                                palette,
                                tr("dialog.sshAlgorithms"),
                                tr("dialog.sshAlgorithmsDesc"),
                                connection_editor_select(
                                    ConnectionEditorRenderContext {
                                        palette,
                                        fields,
                                        cx,
                                    },
                                    "connection-editor-ssh-algorithm-mode",
                                    tr("dialog.algorithmMode"),
                                    ConnectionEditorSelect::SshAlgorithmMode,
                                ),
                            ))
                        },
                    ),
            )
        })
}
