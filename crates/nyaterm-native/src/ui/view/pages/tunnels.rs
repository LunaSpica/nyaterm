use gpui::{
    App, ClickEvent, Context, FontWeight, Hsla, IntoElement, KeyDownEvent, Window, div, prelude::*,
    px, rgb, rgba, svg,
};

use std::collections::{HashMap, HashSet};

use crate::ui::components::{empty_panel, small_button, status_pill};

use super::super::{
    NetworkDeleteConfirmState, NetworkGroupDeleteConfirmState, NetworkGroupEditorState,
    NetworkProxyEditorField, NetworkProxyEditorState, NetworkTab, NetworkTunnelEditorField,
    NetworkTunnelEditorState, NyaTermApp, modal_dialog_footer, modal_dialog_shell, transfer_input,
    tunnel_endpoint, tunnel_mode, tunnel_mode_label, tunnel_name,
};
use nyaterm_domain::{ProxyConfig, ProxyGroup, TunnelConfig, TunnelGroup, truncate_preview};
use nyaterm_session::SshTunnelInfo;

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
use proxy::{network_proxy_editor_panel, proxy_matches, proxy_section, proxy_sections};
use tunnel::{network_tunnel_editor_panel, tunnel_matches, tunnel_section, tunnel_sections};

impl NyaTermApp {
    pub(in crate::ui::view) fn tunnels_view(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let open_tunnels = self
            .tunnel_manager
            .list()
            .unwrap_or_default()
            .into_iter()
            .map(|info| (info.id.clone(), info))
            .collect::<HashMap<_, _>>();
        let query = self.tunnel_search_draft.trim().to_ascii_lowercase();
        let filtered_tunnels = self
            .tunnels
            .iter()
            .filter(|tunnel| tunnel_matches(tunnel, &query))
            .cloned()
            .collect::<Vec<_>>();
        let sections = tunnel_sections(&filtered_tunnels, &self.tunnel_groups);
        let proxy_query = self.proxy_search_draft.trim().to_ascii_lowercase();
        let filtered_proxies = self
            .proxies
            .iter()
            .filter(|proxy| proxy_matches(proxy, &proxy_query))
            .cloned()
            .collect::<Vec<_>>();
        let proxy_sections = proxy_sections(&filtered_proxies, &self.proxy_groups);
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
            ));
        } else if filtered_tunnels.is_empty() {
            tunnel_list = tunnel_list.child(empty_panel("No tunnels match the current search."));
        } else {
            for section in sections {
                tunnel_list = tunnel_list.child(tunnel_section(section, &open_tunnels, self, cx));
            }
        }

        let mut proxy_list = div().flex().flex_col().gap_2();
        if self.proxies.is_empty() {
            proxy_list = proxy_list.child(empty_panel(
                "No saved proxies were found in the native runtime directory yet.",
            ));
        } else if filtered_proxies.is_empty() {
            proxy_list = proxy_list.child(empty_panel("No proxies match the current search."));
        } else {
            for section in proxy_sections {
                proxy_list = proxy_list.child(proxy_section(section, self, cx));
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
            .bg(rgb(0x161b22))
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
            .when_some(self.network_delete_confirm.clone(), |this, confirm| {
                this.child(
                    div()
                        .px_2()
                        .pt_2()
                        .child(network_delete_confirm_panel(confirm, cx)),
                )
            })
            .when_some(self.network_group_editor.clone(), |this, editor| {
                this.child(
                    div()
                        .px_2()
                        .pt_2()
                        .child(network_group_editor_panel(
                            editor,
                            &self.network_group_editor_focus,
                            cx,
                        )),
                )
            })
            .when_some(self.network_group_delete_confirm.clone(), |this, confirm| {
                this.child(
                    div()
                        .px_2()
                        .pt_2()
                        .child(network_group_delete_confirm_panel(confirm, cx)),
                )
            })
            .when_some(self.network_tunnel_editor.clone(), |this, editor| {
                this.child(
                    div()
                        .px_2()
                        .pt_2()
                        .child(network_tunnel_editor_panel(
                            editor,
                            self,
                            &self.network_tunnel_editor_focus,
                            cx,
                        )),
                )
            })
            .when_some(self.network_proxy_editor.clone(), |this, editor| {
                this.child(
                    div()
                        .px_2()
                        .pt_2()
                        .child(network_proxy_editor_panel(
                            editor,
                            self,
                            &self.network_proxy_editor_focus,
                            cx,
                        )),
                )
            })
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
                                    .border_color(rgb(0x30363d))
                                    .bg(rgb(0x0d1117))
                                    .p(px(2.))
                                    .child(
                                        network_tab_button(
                                            "network-tab-tunnels",
                                            "Tunnels",
                                            self.network_tab == NetworkTab::Tunnels,
                                            cx.listener(|this, _, _, cx| {
                                                this.set_network_tab(NetworkTab::Tunnels, cx);
                                            }),
                                        ),
                                    )
                                    .child(
                                        network_tab_button(
                                            "network-tab-proxies",
                                            "Proxies",
                                            self.network_tab == NetworkTab::Proxies,
                                            cx.listener(|this, _, _, cx| {
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
                                            .text_sm()
                                            .font_weight(FontWeight(600.))
                                            .text_color(rgb(0xc9d1d9))
                                            .child(config_label),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap_1()
                                            .child(icon_network_action(
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
                                                this.child(small_button(
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
                                                this.child(small_button(
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
                            // Optional compact search (native convenience; denser than old strip)
                            .child(
                                div()
                                    .h(px(28.))
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        transfer_input(
                                            if self.network_tab == NetworkTab::Tunnels {
                                                "tunnel-search-input"
                                            } else {
                                                "proxy-search-input"
                                            },
                                            "Search",
                                            if self.network_tab == NetworkTab::Tunnels {
                                                self.tunnel_search_draft.clone()
                                            } else {
                                                self.proxy_search_draft.clone()
                                            },
                                            true,
                                        )
                                        .flex_1()
                                        .track_focus(if self.network_tab == NetworkTab::Tunnels {
                                            &self.tunnel_search_focus
                                        } else {
                                            &self.proxy_search_focus
                                        })
                                        .on_click(cx.listener(|this, _, window, cx| {
                                            if this.network_tab == NetworkTab::Tunnels {
                                                window.focus(&this.tunnel_search_focus);
                                            } else {
                                                window.focus(&this.proxy_search_focus);
                                            }
                                            cx.notify();
                                        }))
                                        .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                                            cx.stop_propagation();
                                            if this.network_tab == NetworkTab::Tunnels {
                                                this.handle_tunnel_search_key_down(event, cx);
                                            } else {
                                                this.handle_proxy_search_key_down(event, cx);
                                            }
                                        })),
                                    )
                                    .child(
                                        div()
                                            .font_family("JetBrains Mono")
                                            .text_size(px(10.))
                                            .text_color(rgb(0x6e7681))
                                            .child(match self.network_tab {
                                                NetworkTab::Tunnels => format!(
                                                    "{}/{}",
                                                    filtered_tunnels.len(),
                                                    self.tunnels.len()
                                                ),
                                                NetworkTab::Proxies => format!(
                                                    "{}/{}",
                                                    filtered_proxies.len(),
                                                    self.proxies.len()
                                                ),
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
                this.child(network_delete_confirm_panel(confirm, cx))
            })
            .when_some(self.network_group_editor.clone(), |this, editor| {
                this.child(network_group_editor_panel(
                    editor,
                    &self.network_group_editor_focus,
                    cx,
                ))
            })
            .when_some(self.network_group_delete_confirm.clone(), |this, confirm| {
                this.child(network_group_delete_confirm_panel(confirm, cx))
            })
            .when_some(self.network_tunnel_editor.clone(), |this, editor| {
                this.child(network_tunnel_editor_panel(
                    editor,
                    self,
                    &self.network_tunnel_editor_focus,
                    cx,
                ))
            })
            .when_some(self.network_proxy_editor.clone(), |this, editor| {
                this.child(network_proxy_editor_panel(
                    editor,
                    self,
                    &self.network_proxy_editor_focus,
                    cx,
                ))
            })
    }
}

fn icon_network_action(
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
        .text_color(rgb(0x8b949e))
        .cursor_pointer()
        .hover(|this| this.bg(rgb(0x21262d)).text_color(rgb(0xc9d1d9)))
        .child(label)
        .on_click(on_click)
}
