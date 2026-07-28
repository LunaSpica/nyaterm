use gpui::prelude::*;
use gpui::{Context, FontWeight, IntoElement, div, px, rgb};

use super::super::common::{network_dialog_footer, network_modal_shell};
use super::super::tunnel::tunnel_editor_selector;
use super::helpers::proxy_protocol_label;
use crate::features::{NyaTermApp, TextInputSetup};
use crate::models::{NetworkProxyEditorField, NetworkProxyEditorState};

pub(in crate::features::pages::tunnels) fn network_proxy_editor_panel(
    palette: crate::theme::ThemePalette,
    editor: NetworkProxyEditorState,
    app: &mut NyaTermApp,
    cx: &mut Context<NyaTermApp>,
) -> impl IntoElement {
    let protocol_label = proxy_protocol_label(&editor.protocol);
    let group_label = editor
        .group_id
        .as_deref()
        .and_then(|id| {
            app.tunnel_state
                .catalog
                .proxy_groups
                .iter()
                .find(|group| group.id == id)
                .map(|group| group.name.clone())
        })
        .unwrap_or_else(|| app.tr("network.ungrouped").to_string());
    // A stored password is never shown, so the box says so in its placeholder
    // rather than putting a row of asterisks where the text would go.
    let password_placeholder = if editor.existing_password.is_some() || editor.password_id.is_some()
    {
        app.tr("network.proxyPasswordKeep")
    } else {
        ""
    };
    let name_input = proxy_editor_input(
        app,
        NetworkProxyEditorField::Name,
        app.tr("network.proxyName"),
        editor.name.clone(),
        TextInputSetup::default(),
        cx,
    );
    let is_command = editor.is_proxy_command();
    let command_input = is_command.then(|| {
        proxy_editor_input(
            app,
            NetworkProxyEditorField::Command,
            app.tr("network.proxyCommand"),
            editor.command.clone(),
            TextInputSetup::default(),
            cx,
        )
    });
    let host_input = (!is_command).then(|| {
        proxy_editor_input(
            app,
            NetworkProxyEditorField::Host,
            app.tr("dialog.host"),
            editor.host.clone(),
            TextInputSetup::default(),
            cx,
        )
    });
    let port_input = (!is_command).then(|| {
        proxy_editor_input(
            app,
            NetworkProxyEditorField::Port,
            app.tr("dialog.port"),
            editor.port.clone(),
            TextInputSetup::default(),
            cx,
        )
    });
    let username_input = (!is_command).then(|| {
        proxy_editor_input(
            app,
            NetworkProxyEditorField::Username,
            app.tr("network.proxyUsername"),
            editor.username.clone(),
            TextInputSetup::default(),
            cx,
        )
    });
    let password_input = (!is_command).then(|| {
        proxy_editor_input(
            app,
            NetworkProxyEditorField::Password,
            app.tr("network.proxyPassword"),
            editor.password.clone(),
            TextInputSetup {
                placeholder: password_placeholder.into(),
                masked: true,
                multi_line: false,
            },
            cx,
        )
    });

    let card = div()
        .p_6()
        .flex()
        .flex_col()
        .gap_4()
        .child(
            div()
                .min_w_0()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .text_size(px(15.))
                        .font_weight(FontWeight(700.))
                        .text_color(rgb(palette.text))
                        .child(if editor.id.is_some() {
                            app.tr("network.editProxy")
                        } else {
                            app.tr("network.newProxy")
                        }),
                )
                .child(
                    div()
                        .text_size(px(12.))
                        .text_color(rgb(palette.text_muted))
                        .child(app.tr("network.proxyDialogDescription")),
                ),
        )
        .child(
            div()
                .flex()
                .gap_3()
                .child(div().w(px(144.)).flex_none().child(tunnel_editor_selector(
                    palette,
                    "network-proxy-editor-protocol",
                    app.tr("network.protocol"),
                    protocol_label.to_string(),
                    cx.listener(|this, _, _, cx| {
                        this.cycle_network_proxy_protocol(cx);
                    }),
                )))
                .child(div().flex_1().min_w_0().child(name_input)),
        )
        .child(tunnel_editor_selector(
            palette,
            "network-proxy-editor-group",
            app.tr("network.group"),
            group_label,
            cx.listener(|this, _, _, cx| {
                this.cycle_network_proxy_group(cx);
            }),
        ))
        .when(editor.is_proxy_command(), |this| {
            this.children(command_input).child(
                div()
                    .text_xs()
                    .text_color(rgb(palette.text_muted))
                    .child(app.tr("network.proxyCommandHint")),
            )
        })
        .when(!editor.is_proxy_command(), |this| {
            this.child(
                div()
                    .grid()
                    .grid_cols(2)
                    .gap_2()
                    .children(host_input)
                    .children(port_input),
            )
            .child(
                div()
                    .grid()
                    .grid_cols(2)
                    .gap_2()
                    .children(username_input)
                    .children(password_input),
            )
        })
        .when_some(editor.error.clone(), |this, error| {
            this.child(div().text_xs().text_color(rgb(palette.danger)).child(error))
        })
        .child(network_dialog_footer(
            app,
            palette,
            "network-proxy-editor-cancel",
            "network-proxy-editor-save",
            app.tr("common.save"),
            cx.listener(|this, _, _, cx| {
                this.close_network_proxy_editor(cx);
            }),
            cx.listener(|this, _, _, cx| {
                this.save_network_proxy_editor(cx);
            }),
        ));

    let card = card.on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, _, cx| {
        match event.keystroke.key.as_str() {
            "escape" => {
                cx.stop_propagation();
                this.close_network_proxy_editor(cx);
            }
            "enter" => {
                cx.stop_propagation();
                this.save_network_proxy_editor(cx);
            }
            _ => {}
        }
    }));

    network_modal_shell(
        palette,
        app.shell_surface_color(palette.bg),
        "network-proxy-editor-modal",
        520.,
        card,
    )
}

pub(in crate::features::pages::tunnels) fn proxy_editor_input(
    app: &mut NyaTermApp,
    field: NetworkProxyEditorField,
    caption: &'static str,
    value: String,
    setup: TextInputSetup,
    cx: &mut Context<NyaTermApp>,
) -> gpui::AnyElement {
    app.text_input_field(
        format!("network.proxy-editor.{}", proxy_editor_field_key(field)),
        caption,
        &value,
        setup,
        cx,
    )
    .into_any_element()
}

/// The stable part of a proxy field's input id.
fn proxy_editor_field_key(field: NetworkProxyEditorField) -> &'static str {
    match field {
        NetworkProxyEditorField::Name => "name",
        NetworkProxyEditorField::Host => "host",
        NetworkProxyEditorField::Port => "port",
        NetworkProxyEditorField::Command => "command",
        NetworkProxyEditorField::Username => "username",
        NetworkProxyEditorField::Password => "password",
    }
}
