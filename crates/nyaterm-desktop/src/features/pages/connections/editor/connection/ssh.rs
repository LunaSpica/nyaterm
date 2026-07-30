use gpui::{
    Context, FontWeight, IntoElement, SharedString, div,
    prelude::{
        FluentBuilder, InteractiveElement, ParentElement, StatefulInteractiveElement, Styled,
    },
    px, rgb, rgba, svg,
};

use nyaterm_core::truncate_preview;
use nyaterm_ui::NyaRadioGroup;

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

    let auth_values = auth_options
        .iter()
        .map(|option| option.value.clone())
        .collect::<Vec<_>>();
    let auth_tabs = div().h(px(34.)).child(
        NyaRadioGroup::new("connection-authentication")
            .items(auth_options.iter().map(|option| option.label.clone()))
            .selected_index(auth_options.iter().position(|option| option.selected))
            .horizontal()
            .on_select(cx.listener(move |this, index: &usize, _, cx| {
                let Some(value) = auth_values.get(*index) else {
                    return;
                };
                this.set_connection_editor_select_value(
                    ConnectionEditorSelect::Authentication,
                    value.as_deref(),
                    cx,
                );
            })),
    );

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
            // Tauri gives the host the room and pins the port at a fixed width,
            // rather than splitting the row down the middle: a host is long and
            // a port is never more than five digits.
            div()
                .flex()
                .gap_3()
                .child(div().min_w_0().flex_1().child(editor_field(
                    palette,
                    required(tr("dialog.host")),
                    ConnectionEditorField::Host,
                    fields,
                    cx,
                )))
                .child(div().w(px(150.)).flex_none().child(editor_stepper_field(
                    palette,
                    required(tr("dialog.port")),
                    ConnectionEditorField::Port,
                    fields,
                    cx,
                ))),
        )
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
                    ),
            )
        })
}
