use gpui::{
    Context, FontWeight, IntoElement, SharedString, div,
    prelude::{
        FluentBuilder, InteractiveElement, ParentElement, StatefulInteractiveElement, Styled,
    },
    px, rgb, rgba, svg,
};

use nyaterm_core::truncate_preview;

use crate::features::{ConnectionEditorToggle, NyaTermApp};
use crate::models::{
    ConnectionEditorAdvancedTab, ConnectionEditorField, ConnectionEditorMenu,
    ConnectionEditorPasswordSource, ConnectionEditorState,
};

use super::super::super::list::{
    ConnectionEditorChoice, connection_editor_select, editor_field, toggle_chip,
};

fn ssh_segment_tab(
    palette: crate::theme::ThemePalette,
    id: &'static str,
    label: &'static str,
    selected: bool,
    on_click: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .h(px(28.))
        .min_w_0()
        .flex_1()
        .flex()
        .items_center()
        .justify_center()
        .rounded_sm()
        .border_1()
        .border_color(if selected {
            rgba((palette.primary << 8) | 0x66)
        } else {
            rgba(0x00000000)
        })
        .bg(if selected {
            rgba((palette.primary << 8) | 0x18)
        } else {
            rgba(0x00000000)
        })
        .text_xs()
        .font_weight(FontWeight(600.))
        .text_color(if selected {
            rgb(palette.primary)
        } else {
            rgb(palette.text_muted)
        })
        .cursor_pointer()
        .hover(|this| this.bg(rgb(palette.hover)).text_color(rgb(palette.text)))
        .child(label)
        .on_click(on_click)
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
        .bg(rgba((palette.accent << 8) | 0x18))
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
    palette: crate::theme::ThemePalette,
    editor: &ConnectionEditorState,
    password_display: String,
    password_label: String,
    key_label: String,
    otp_label: String,
    proxy_label: String,
    jump_label: String,
    auth_options: Vec<ConnectionEditorChoice>,
    password_options: Vec<ConnectionEditorChoice>,
    key_options: Vec<ConnectionEditorChoice>,
    otp_options: Vec<ConnectionEditorChoice>,
    proxy_options: Vec<ConnectionEditorChoice>,
    jump_options: Vec<ConnectionEditorChoice>,
    backspace_options: Vec<ConnectionEditorChoice>,
    open_menu: Option<ConnectionEditorMenu>,
    language: &str,
    cx: &mut Context<NyaTermApp>,
) -> gpui::Div {
    let tr = |key: &'static str| crate::i18n::text(language, key);
    let backspace_value = match editor.backspace_mode.as_str() {
        "ctrl-h" | "bs" | "ctrl_h" => tr("dialog.backspaceCtrlH"),
        _ => tr("dialog.backspaceDel"),
    };

    let mut auth_tabs = div()
        .h(px(32.))
        .p_1()
        .flex()
        .items_center()
        .gap_1()
        .rounded_md()
        .bg(rgb(palette.surface_elevated));
    for (index, option) in auth_options.into_iter().enumerate() {
        let option_value = option.value.clone();
        auth_tabs = auth_tabs.child(
            div()
                .id(SharedString::from(format!("connection-auth-tab-{index}")))
                .h(px(24.))
                .min_w_0()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .rounded_sm()
                .border_1()
                .border_color(if option.selected {
                    rgba((palette.primary << 8) | 0x66)
                } else {
                    rgba(0x00000000)
                })
                .bg(if option.selected {
                    rgba((palette.primary << 8) | 0x18)
                } else {
                    rgba(0x00000000)
                })
                .text_xs()
                .font_weight(FontWeight(600.))
                .text_color(if option.selected {
                    rgb(palette.primary)
                } else {
                    rgb(palette.text_muted)
                })
                .cursor_pointer()
                .hover(|this| this.bg(rgb(palette.hover)).text_color(rgb(palette.text)))
                .child(option.label)
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.set_connection_editor_menu_value(
                        ConnectionEditorMenu::Authentication,
                        option_value.as_deref(),
                        cx,
                    );
                })),
        );
    }

    let advanced_tabs = div()
        .h(px(32.))
        .p_1()
        .flex()
        .gap_1()
        .rounded_md()
        .bg(rgb(palette.surface_elevated))
        .child(ssh_segment_tab(
            palette,
            "connection-advanced-proxy-tab",
            tr("dialog.proxySelect"),
            editor.advanced_network_tab == ConnectionEditorAdvancedTab::Proxy,
            cx.listener(|this, _, _, cx| {
                this.set_connection_editor_advanced_tab(ConnectionEditorAdvancedTab::Proxy, cx);
            }),
        ))
        .child(ssh_segment_tab(
            palette,
            "connection-advanced-jump-tab",
            tr("dialog.proxyJump"),
            editor.advanced_network_tab == ConnectionEditorAdvancedTab::JumpHost,
            cx.listener(|this, _, _, cx| {
                this.set_connection_editor_advanced_tab(ConnectionEditorAdvancedTab::JumpHost, cx);
            }),
        ))
        .child(ssh_segment_tab(
            palette,
            "connection-advanced-otp-tab",
            tr("dialog.twoFactorAuth"),
            editor.advanced_network_tab == ConnectionEditorAdvancedTab::TwoFactor,
            cx.listener(|this, _, _, cx| {
                this.set_connection_editor_advanced_tab(ConnectionEditorAdvancedTab::TwoFactor, cx);
            }),
        ));

    let behavior_tabs = div()
        .h(px(32.))
        .p_1()
        .flex()
        .gap_1()
        .rounded_md()
        .bg(rgb(palette.surface_elevated))
        .child(ssh_segment_tab(
            palette,
            "connection-advanced-command-tab",
            tr("dialog.commandExecution"),
            editor.advanced_behavior_tab == ConnectionEditorAdvancedTab::PostLogin,
            cx.listener(|this, _, _, cx| {
                this.set_connection_editor_advanced_tab(ConnectionEditorAdvancedTab::PostLogin, cx);
            }),
        ))
        .child(ssh_segment_tab(
            palette,
            "connection-advanced-x11-tab",
            tr("dialog.x11Forwarding"),
            editor.advanced_behavior_tab == ConnectionEditorAdvancedTab::X11,
            cx.listener(|this, _, _, cx| {
                this.set_connection_editor_advanced_tab(ConnectionEditorAdvancedTab::X11, cx);
            }),
        ))
        .child(ssh_segment_tab(
            palette,
            "connection-advanced-backspace-tab",
            tr("dialog.backspaceMode"),
            editor.advanced_behavior_tab == ConnectionEditorAdvancedTab::Backspace,
            cx.listener(|this, _, _, cx| {
                this.set_connection_editor_advanced_tab(ConnectionEditorAdvancedTab::Backspace, cx);
            }),
        ));

    let password_source_tabs = div()
        .h(px(32.))
        .p_1()
        .flex()
        .gap_1()
        .rounded_md()
        .bg(rgb(palette.surface_elevated))
        .child(ssh_segment_tab(
            palette,
            "connection-password-source-ask",
            tr("dialog.askWhenConnecting"),
            editor.password_source == ConnectionEditorPasswordSource::Ask,
            cx.listener(|this, _, _, cx| {
                this.set_connection_editor_password_source(ConnectionEditorPasswordSource::Ask, cx);
            }),
        ))
        .child(ssh_segment_tab(
            palette,
            "connection-password-source-direct",
            tr("dialog.directPassword"),
            editor.password_source == ConnectionEditorPasswordSource::Direct,
            cx.listener(|this, _, _, cx| {
                this.set_connection_editor_password_source(
                    ConnectionEditorPasswordSource::Direct,
                    cx,
                );
            }),
        ))
        .child(ssh_segment_tab(
            palette,
            "connection-password-source-saved",
            tr("dialog.savedPassword"),
            editor.password_source == ConnectionEditorPasswordSource::Saved,
            cx.listener(|this, _, _, cx| {
                this.set_connection_editor_password_source(
                    ConnectionEditorPasswordSource::Saved,
                    cx,
                );
            }),
        ));

    div()
        .flex()
        .flex_col()
        .gap_3()
        .child(
            div()
                .grid()
                .grid_cols(2)
                .gap_2()
                .child(editor_field(
                    palette,
                    "connection-editor-host",
                    tr("dialog.host"),
                    editor.host.clone(),
                    editor.focused_field == ConnectionEditorField::Host,
                    cx.listener(|this, _, window, cx| {
                        this.focus_connection_editor_field(ConnectionEditorField::Host, window, cx);
                    }),
                ))
                .child(editor_field(
                    palette,
                    "connection-editor-port",
                    tr("dialog.port"),
                    editor.port.clone(),
                    editor.focused_field == ConnectionEditorField::Port,
                    cx.listener(|this, _, window, cx| {
                        this.focus_connection_editor_field(ConnectionEditorField::Port, window, cx);
                    }),
                )),
        )
        .child(editor_field(
            palette,
            "connection-editor-username",
            tr("dialog.username"),
            editor.username.clone(),
            editor.focused_field == ConnectionEditorField::Username,
            cx.listener(|this, _, window, cx| {
                this.focus_connection_editor_field(ConnectionEditorField::Username, window, cx);
            }),
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
                                "connection-editor-password",
                                tr("dialog.password"),
                                password_display.clone(),
                                editor.focused_field == ConnectionEditorField::Password,
                                cx.listener(|this, _, window, cx| {
                                    this.focus_connection_editor_field(
                                        ConnectionEditorField::Password,
                                        window,
                                        cx,
                                    );
                                }),
                            ))
                        },
                    )
                    .when(
                        editor.password_source == ConnectionEditorPasswordSource::Saved,
                        |this| {
                            this.child(connection_editor_select(
                                palette,
                                "connection-editor-saved-password",
                                tr("dialog.savedPassword"),
                                truncate_preview(&password_label, 36),
                                ConnectionEditorMenu::SavedPassword,
                                open_menu == Some(ConnectionEditorMenu::SavedPassword),
                                password_options,
                                cx,
                            ))
                        },
                    ),
            )
        })
        .when(
            editor.auth_mode == "key" || editor.auth_mode == "certificate",
            |this| {
                this.child(connection_editor_select(
                    palette,
                    "connection-editor-key",
                    tr("dialog.privateKey"),
                    truncate_preview(&key_label, 36),
                    ConnectionEditorMenu::SshKey,
                    open_menu == Some(ConnectionEditorMenu::SshKey),
                    key_options,
                    cx,
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
                                    palette,
                                    "connection-editor-proxy",
                                    tr("dialog.proxySelect"),
                                    truncate_preview(&proxy_label, 48),
                                    ConnectionEditorMenu::Proxy,
                                    open_menu == Some(ConnectionEditorMenu::Proxy),
                                    proxy_options,
                                    cx,
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
                                    palette,
                                    "connection-editor-jump",
                                    tr("dialog.selectProxyJump"),
                                    truncate_preview(&jump_label, 48),
                                    ConnectionEditorMenu::ProxyJump,
                                    open_menu == Some(ConnectionEditorMenu::ProxyJump),
                                    jump_options,
                                    cx,
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
                                        palette,
                                        "connection-editor-otp",
                                        tr("dialog.selectOtp"),
                                        truncate_preview(&otp_label, 36),
                                        ConnectionEditorMenu::Otp,
                                        open_menu == Some(ConnectionEditorMenu::Otp),
                                        otp_options,
                                        cx,
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
                                            "connection-editor-post-login-command",
                                            tr("dialog.postLoginCommandContent"),
                                            editor.post_login_command.clone(),
                                            editor.focused_field
                                                == ConnectionEditorField::PostLoginCommand,
                                            cx.listener(|this, _, window, cx| {
                                                this.focus_connection_editor_field(
                                                    ConnectionEditorField::PostLoginCommand,
                                                    window,
                                                    cx,
                                                );
                                            }),
                                        ))
                                        .child(
                                            editor_field(
                                                palette,
                                                "connection-editor-post-login-delay",
                                                tr("dialog.postLoginDelay"),
                                                editor.post_login_delay_ms.clone(),
                                                editor.focused_field
                                                    == ConnectionEditorField::PostLoginDelay,
                                                cx.listener(|this, _, window, cx| {
                                                    this.focus_connection_editor_field(
                                                        ConnectionEditorField::PostLoginDelay,
                                                        window,
                                                        cx,
                                                    );
                                                }),
                                            ),
                                        )
                                    }),
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
                                    palette,
                                    "connection-editor-backspace",
                                    tr("dialog.backspaceMode"),
                                    backspace_value,
                                    ConnectionEditorMenu::Backspace,
                                    open_menu == Some(ConnectionEditorMenu::Backspace),
                                    backspace_options,
                                    cx,
                                ),
                            ))
                        },
                    ),
            )
        })
}
