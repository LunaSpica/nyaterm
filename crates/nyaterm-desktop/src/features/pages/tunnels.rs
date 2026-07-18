use gpui::{
    App, ClickEvent, Context, FontWeight, Hsla, IntoElement, KeyDownEvent, Window, div, prelude::*,
    px, rgb, svg,
};

use std::collections::{HashMap, HashSet};

use crate::widgets::{empty_panel, small_button, status_pill};

use super::super::{
    NetworkDeleteConfirmState, NetworkGroupDeleteConfirmState, NetworkGroupEditorState,
    NetworkProxyEditorField, NetworkProxyEditorState, NetworkTab, NetworkTunnelEditorField,
    NetworkTunnelEditorState, NyaTermApp, modal_dialog_footer, modal_dialog_shell, transfer_input,
    tunnel_endpoint, tunnel_mode, tunnel_mode_label, tunnel_name,
};
use nyaterm_core::{ProxyConfig, ProxyGroup, TunnelConfig, TunnelGroup, truncate_preview};
use nyaterm_transport::SshTunnelInfo;

#[path = "tunnels/common.rs"]
mod common;
#[path = "tunnels/proxy.rs"]
mod proxy;
#[path = "tunnels/tunnel.rs"]
mod tunnel;

use common::{
    network_delete_confirm_panel, network_group_delete_confirm_panel, network_group_editor_panel,
    network_tab_button,
};
use proxy::{network_proxy_editor_panel, proxy_section, proxy_sections};
use tunnel::{network_tunnel_editor_panel, tunnel_section, tunnel_sections};

impl NyaTermApp {
    pub(in crate::features) fn tunnels_view(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = self.theme_palette();
        let open_tunnels = self
            .tunnel_manager
            .list()
            .unwrap_or_default()
            .into_iter()
            .map(|info| (info.id.clone(), info))
            .collect::<HashMap<_, _>>();
        let sections = tunnel_sections(palette, &self.tunnels, &self.tunnel_groups);
        let proxy_sections = proxy_sections(&self.proxies, &self.proxy_groups);
        let missing_connections = self
            .tunnels
            .iter()
            .filter(|tunnel| {
                tunnel.connection_id.as_deref().is_none_or(|id| {
                    !self
                        .connections
                        .iter()
                        .any(|connection| connection.id == id)
                })
            })
            .count();
        let mut tunnel_list = div().flex().flex_col().gap_2();
        if self.tunnels.is_empty() {
            tunnel_list = tunnel_list.child(empty_panel(
                "No saved tunnels were found in the native runtime directory yet.",
                self.theme_palette(),
            ));
        } else {
            for section in sections {
                tunnel_list =
                    tunnel_list.child(tunnel_section(palette, section, &open_tunnels, self, cx));
            }
        }

        let mut proxy_list = div().flex().flex_col().gap_2();
        if self.proxies.is_empty() {
            proxy_list = proxy_list.child(empty_panel(
                "No saved proxies were found in the native runtime directory yet.",
                self.theme_palette(),
            ));
        } else {
            for section in proxy_sections {
                proxy_list = proxy_list.child(proxy_section(palette, section, self, cx));
            }
        }

        // Tauri NetworkPanel body (PanelHeader is shared):
        // scroll(p-3) > Tabs(grid-cols-2) > config row (label + New Group/New item) > grouped list.
        // Network create/edit/delete use modal dialogs (Tauri Dialog) over the panel.
        let config_label = match self.network_tab {
            NetworkTab::Tunnels => "Tunnel Config",
            NetworkTab::Proxies => "Proxy Config",
        };

        div()
            .flex()
            .flex_col()
            .size_full()
            .overflow_hidden()
            .bg(rgb(self.theme_palette().surface))
            .when(
                self.network_tab == NetworkTab::Tunnels && missing_connections > 0,
                |this| {
                    this.child(
                        div()
                            .mx_2()
                            .mt_2()
                            .rounded_md()
                            .border_1()
                            .border_color(rgb(0xfacc15))
                            .bg(rgb(0x2f260f))
                            .px_2()
                            .py_1()
                            .text_xs()
                            .text_color(rgb(0xfef3c7))
                            .child(format!(
                                "{missing_connections} tunnel profile(s) reference missing SSH connections."
                            )),
                    )
                },
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .overflow_hidden()
                    .child(
                        div()
                            .id("network-list-scroll")
                            .size_full()
                            .overflow_scroll()
                            .scrollbar_width(px(6.))
                            .p_2()
                            .flex()
                            .flex_col()
                            .gap_2()
                            // TabsList grid-cols-2 h-8
                            .child(
                                div()
                                    .h(px(32.))
                                    .w_full()
                                    .flex()
                                    .items_center()
                                    .gap_1()
                                    .rounded_md()
                                    .border_1()
                                    .border_color(rgb(self.theme_palette().border))
                                    .bg(rgb(self.theme_palette().input))
                                    .p(px(2.))
                                    .child(
                                        network_tab_button(
                                            "network-tab-tunnels",
                                            "Tunnels",
                                            self.network_tab == NetworkTab::Tunnels, self.theme_palette(), cx.listener(|this, _, _, cx| {
                                                this.set_network_tab(NetworkTab::Tunnels, cx);
                                            }),
                                        ),
                                    )
                                    .child(
                                        network_tab_button(
                                            "network-tab-proxies",
                                            "Proxies",
                                            self.network_tab == NetworkTab::Proxies, self.theme_palette(), cx.listener(|this, _, _, cx| {
                                                this.set_network_tab(NetworkTab::Proxies, cx);
                                            }),
                                        ),
                                    ),
                            )
                            // Config row: label left, group + new right (Tauri)
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .gap_2()
                                    .child(
                                        div()
                                            .text_size(px(12.))
                                            .font_weight(FontWeight(600.))
                                            .text_color(rgb(palette.text))
                                            .child(config_label),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap_1()
                                            .child(icon_network_action(
                                                palette,
                                                "network-group-new",
                                                "📁+",
                                                cx.listener(|this, _, _, cx| {
                                                    this.open_network_group_editor(
                                                        this.network_tab,
                                                        None,
                                                        cx,
                                                    );
                                                }),
                                            ))
                                            .when(self.network_tab == NetworkTab::Tunnels, |this| {
                                                this.child(small_button(palette,
                                                    "network-tunnel-new",
                                                    "+ Tunnel",
                                                    cx.listener(|this, _, window, cx| {
                                                        this.open_network_tunnel_editor(
                                                            None, window, cx,
                                                        );
                                                    }),
                                                ))
                                            })
                                            .when(self.network_tab == NetworkTab::Proxies, |this| {
                                                this.child(small_button(palette,
                                                    "network-proxy-new",
                                                    "+ Proxy",
                                                    cx.listener(|this, _, window, cx| {
                                                        this.open_network_proxy_editor(
                                                            None, window, cx,
                                                        );
                                                    }),
                                                ))
                                            }),
                                    ),
                            )
                            .child(match self.network_tab {
                                NetworkTab::Tunnels => tunnel_list.into_any_element(),
                                NetworkTab::Proxies => proxy_list.into_any_element(),
                            }),
                    ),
            )
            // Tauri-style Dialog overlays (absolute) above the panel body.
            .when_some(self.network_delete_confirm.clone(), |this, confirm| {
                this.child(network_delete_confirm_panel(palette, confirm, cx))
            })
            .when_some(self.network_group_editor.clone(), |this, editor| {
                this.child(network_group_editor_panel(
                    palette,
                    editor,
                    &self.network_group_editor_focus,
                    cx,
                ))
            })
            .when_some(self.network_group_delete_confirm.clone(), |this, confirm| {
                this.child(network_group_delete_confirm_panel(palette, confirm, cx))
            })
            .when_some(self.network_tunnel_editor.clone(), |this, editor| {
                this.child(network_tunnel_editor_panel(
                    palette,
                    editor,
                    self,
                    &self.network_tunnel_editor_focus,
                    cx,
                ))
            })
            .when_some(self.network_proxy_editor.clone(), |this, editor| {
                this.child(network_proxy_editor_panel(
                    palette,
                    editor,
                    self,
                    &self.network_proxy_editor_focus,
                    cx,
                ))
            })
    }
}

fn icon_network_action(
    palette: crate::theme::ThemePalette,
    id: impl Into<String>,
    label: &'static str,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(gpui::SharedString::from(id.into()))
        .size(px(28.))
        .flex()
        .items_center()
        .justify_center()
        .rounded_md()
        .text_size(px(12.))
        .text_color(rgb(palette.text_muted))
        .cursor_pointer()
        .hover(|this| {
            this.bg(rgb(palette.surface_elevated))
                .text_color(rgb(palette.text))
        })
        .child(label)
        .on_click(on_click)
}
