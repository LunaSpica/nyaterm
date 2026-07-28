use gpui::prelude::*;
use gpui::{App, ClickEvent, Context, FontWeight, IntoElement, Window, div, px, rgb};

use super::super::common::{network_dialog_footer, network_modal_shell};
use crate::features::{NyaTermApp, TextInputSetup};
use crate::models::{NetworkTunnelEditorField, NetworkTunnelEditorState};
use nyaterm_core::truncate_preview;

pub(in crate::features::pages::tunnels) fn network_tunnel_editor_panel(
    palette: crate::theme::ThemePalette,
    editor: NetworkTunnelEditorState,
    app: &mut NyaTermApp,
    cx: &mut Context<NyaTermApp>,
) -> impl IntoElement {
    let connection_label = editor
        .connection_id
        .as_deref()
        .and_then(|id| {
            app.connection_catalog
                .connections()
                .iter()
                .find(|connection| connection.id == id)
                .map(|connection| connection.name.clone())
        })
        .unwrap_or_else(|| app.tr("network.connectionPickerPlaceholder").to_string());
    let group_label = editor
        .group_id
        .as_deref()
        .and_then(|id| {
            app.tunnel_state
                .tunnel_groups()
                .iter()
                .find(|group| group.id == id)
                .map(|group| group.name.clone())
        })
        .unwrap_or_else(|| app.tr("network.ungrouped").to_string());
    let mode_label = match editor.tunnel_type.as_str() {
        "remote" => app.tr("network.remoteTunnel"),
        "dynamic" => app.tr("network.dynamicTunnel"),
        _ => app.tr("network.localTunnel"),
    };
    let preview = tunnel_editor_preview(&editor);
    // Built up front: the card is one long builder chain that only reads `app`,
    // and creating an input needs it mutably.
    let name_input = tunnel_editor_input(
        app,
        NetworkTunnelEditorField::Name,
        app.tr("network.tunnelName"),
        editor.name.clone(),
        cx,
    );
    let listen_port_input = tunnel_editor_input(
        app,
        NetworkTunnelEditorField::ListenPort,
        match editor.tunnel_type.as_str() {
            "remote" => app.tr("network.listenPortRemote"),
            "dynamic" => app.tr("network.listenPortDynamic"),
            _ => app.tr("network.listenPortLocal"),
        },
        editor.listen_port.clone(),
        cx,
    );
    let dynamic = editor.is_dynamic();
    let target_port_input = (!dynamic).then(|| {
        tunnel_editor_input(
            app,
            NetworkTunnelEditorField::TargetPort,
            match editor.tunnel_type.as_str() {
                "remote" => app.tr("network.targetPortRemote"),
                _ => app.tr("network.targetPortLocal"),
            },
            editor.target_port.clone(),
            cx,
        )
    });
    let target_host_input = (!dynamic).then(|| {
        tunnel_editor_input(
            app,
            NetworkTunnelEditorField::TargetHost,
            match editor.tunnel_type.as_str() {
                "remote" => app.tr("network.targetHostRemote"),
                _ => app.tr("network.targetHostLocal"),
            },
            editor.target_host.clone(),
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
                            app.tr("network.editTunnel")
                        } else {
                            app.tr("network.newTunnel")
                        }),
                )
                .child(
                    div()
                        .text_size(px(12.))
                        .text_color(rgb(palette.text_muted))
                        .child(app.tr("network.tunnelDialogDescription")),
                ),
        )
        .child(
            div()
                .grid()
                .grid_cols(3)
                .gap_2()
                .child(name_input)
                .child(tunnel_editor_selector(
                    palette,
                    "network-tunnel-editor-type",
                    app.tr("network.tunnelType"),
                    mode_label.to_string(),
                    cx.listener(|this, _, _, cx| {
                        this.cycle_network_tunnel_type(cx);
                    }),
                ))
                .child(tunnel_editor_selector(
                    palette,
                    "network-tunnel-editor-group",
                    app.tr("network.group"),
                    group_label,
                    cx.listener(|this, _, _, cx| {
                        this.cycle_network_tunnel_group(cx);
                    }),
                )),
        )
        .child(tunnel_editor_selector(
            palette,
            "network-tunnel-editor-connection",
            app.tr("network.savedConnection"),
            connection_label,
            cx.listener(|this, _, _, cx| {
                this.cycle_network_tunnel_connection(cx);
            }),
        ))
        .child(
            div()
                .grid()
                .grid_cols(2)
                .gap_2()
                .child(listen_port_input)
                .when(!editor.is_dynamic(), |this| {
                    this.children(target_port_input)
                }),
        )
        .when(!editor.is_dynamic(), |this| {
            this.children(target_host_input)
        })
        .child(
            div()
                .grid()
                .grid_cols(2)
                .gap_2()
                .child(tunnel_editor_option(
                    palette,
                    "network-tunnel-editor-bind-local",
                    app.tr("network.bindLocalhostOnly"),
                    "127.0.0.1",
                    editor.bind_localhost,
                    cx.listener(|this, _, _, cx| {
                        this.set_network_tunnel_bind_localhost(true, cx);
                    }),
                ))
                .child(tunnel_editor_option(
                    palette,
                    "network-tunnel-editor-bind-all",
                    app.tr("network.bindAllInterfaces"),
                    "0.0.0.0",
                    !editor.bind_localhost,
                    cx.listener(|this, _, _, cx| {
                        this.set_network_tunnel_bind_localhost(false, cx);
                    }),
                )),
        )
        .child(tunnel_editor_option(
            palette,
            "network-tunnel-editor-auto",
            app.tr("network.autoOpen"),
            app.tr("network.tunnelConnectionHint"),
            editor.auto_open,
            cx.listener(|this, _, _, cx| {
                this.toggle_network_tunnel_auto_open(cx);
            }),
        ))
        .child(
            div()
                .rounded_sm()
                .border_1()
                .border_color(rgb(palette.border))
                .bg(rgb(palette.input))
                .p_3()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(palette.text_muted))
                        .child(app.tr("network.tunnelPreview")),
                )
                .child(
                    div()
                        .font_family(crate::features::gpui_code_font_family())
                        .text_xs()
                        .text_color(rgb(palette.text))
                        .child(preview),
                ),
        )
        .when_some(editor.error.clone(), |this, error| {
            this.child(div().text_xs().text_color(rgb(palette.danger)).child(error))
        })
        .child(network_dialog_footer(
            app,
            palette,
            "network-tunnel-editor-cancel",
            "network-tunnel-editor-save",
            app.tr("common.save"),
            cx.listener(|this, _, _, cx| {
                this.close_network_tunnel_editor(cx);
            }),
            cx.listener(|this, _, _, cx| {
                this.save_network_tunnel_editor(cx);
            }),
        ));

    // Escape and Enter belong to the dialog, not to any one box: the inputs
    // deliberately leave both unconsumed so they reach here.
    let card = card
        .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, _, cx| {
            match event.keystroke.key.as_str() {
                "escape" => {
                    cx.stop_propagation();
                    this.close_network_tunnel_editor(cx);
                }
                "enter" => {
                    cx.stop_propagation();
                    this.save_network_tunnel_editor(cx);
                }
                _ => {}
            }
        }))
        .into_any_element();

    network_modal_shell(
        palette,
        app.shell_surface_color(palette.bg),
        "network-tunnel-editor-modal",
        640.,
        card,
    )
}

pub(in crate::features::pages::tunnels) fn tunnel_editor_input(
    app: &mut NyaTermApp,
    field: NetworkTunnelEditorField,
    caption: &'static str,
    value: String,
    cx: &mut Context<NyaTermApp>,
) -> gpui::AnyElement {
    app.text_input_field(
        format!("network.tunnel-editor.{}", tunnel_editor_field_key(field)),
        caption,
        &value,
        TextInputSetup::default(),
        cx,
    )
    .into_any_element()
}

/// The stable part of a tunnel field's input id.
pub(in crate::features::pages::tunnels) fn tunnel_editor_field_key(
    field: NetworkTunnelEditorField,
) -> &'static str {
    match field {
        NetworkTunnelEditorField::Name => "name",
        NetworkTunnelEditorField::ListenPort => "listen-port",
        NetworkTunnelEditorField::TargetHost => "target-host",
        NetworkTunnelEditorField::TargetPort => "target-port",
    }
}

pub(in crate::features::pages::tunnels) fn tunnel_editor_selector(
    palette: crate::theme::ThemePalette,
    id: impl Into<String>,
    label: &'static str,
    value: String,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(gpui::SharedString::from(id.into()))
        .h(px(52.))
        .px_3()
        .py_2()
        .flex()
        .flex_col()
        .gap_1()
        .rounded_md()
        .border_1()
        .border_color(rgb(palette.border))
        .bg(rgb(palette.bg))
        .cursor_pointer()
        .hover(|this| this.bg(rgb(palette.surface)))
        .child(
            div()
                .text_size(px(11.))
                .text_color(rgb(palette.text_muted))
                .child(label),
        )
        .child(
            div()
                .font_family(crate::features::gpui_code_font_family())
                .text_size(px(12.))
                .text_color(rgb(palette.text))
                .child(truncate_preview(&value, 42)),
        )
        .on_click(on_click)
}

pub(in crate::features::pages::tunnels) fn tunnel_editor_option(
    palette: crate::theme::ThemePalette,
    id: impl Into<String>,
    title: &'static str,
    detail: &'static str,
    active: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    // Tauri-like selectable option cards for bind host / auto open.
    div()
        .id(gpui::SharedString::from(id.into()))
        .rounded_md()
        .border_1()
        .border_color(if active {
            rgb(palette.link)
        } else {
            rgb(palette.border)
        })
        .bg(if active {
            rgb(palette.hover)
        } else {
            rgb(palette.bg)
        })
        .px_3()
        .py_2()
        .flex()
        .flex_col()
        .gap_1()
        .cursor_pointer()
        .hover(|this| this.bg(rgb(palette.surface)))
        .child(
            div()
                .text_size(px(12.))
                .font_weight(FontWeight(600.))
                .text_color(if active {
                    rgb(palette.link)
                } else {
                    rgb(palette.text)
                })
                .child(title),
        )
        .child(
            div()
                .text_size(px(11.))
                .text_color(rgb(palette.text_muted))
                .child(detail),
        )
        .on_click(on_click)
}

pub(super) fn tunnel_editor_preview(editor: &NetworkTunnelEditorState) -> String {
    let bind_host = if editor.bind_localhost {
        "127.0.0.1"
    } else {
        "0.0.0.0"
    };
    let listen_port = editor.listen_port.trim();
    let listen_port = if listen_port.is_empty() {
        "?"
    } else {
        listen_port
    };
    if editor.is_dynamic() {
        return format!("SOCKS {bind_host}:{listen_port}");
    }

    let target_host = editor.target_host.trim();
    let target_host = if target_host.is_empty() {
        "?"
    } else {
        target_host
    };
    let target_port = editor.target_port.trim();
    let target_port = if target_port.is_empty() {
        "?"
    } else {
        target_port
    };
    if editor.tunnel_type == "remote" {
        format!("remote {bind_host}:{listen_port} -> {target_host}:{target_port}")
    } else {
        format!("local {bind_host}:{listen_port} -> {target_host}:{target_port}")
    }
}
