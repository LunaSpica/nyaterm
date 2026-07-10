use gpui::{
    App, ClickEvent, Context, FontWeight, Hsla, IntoElement, KeyDownEvent, Window, div, prelude::*,
    px, rgb,
};

use std::collections::{HashMap, HashSet};

use crate::ui::components::{empty_panel, section_header, small_button, status_pill};

use super::super::{
    NetworkDeleteConfirmState, NetworkGroupDeleteConfirmState, NetworkGroupEditorState,
    NetworkProxyEditorField, NetworkProxyEditorState, NetworkTab, NetworkTunnelEditorField,
    NetworkTunnelEditorState, NyaTermApp, metric, transfer_input, tunnel_endpoint, tunnel_mode,
    tunnel_mode_label, tunnel_name,
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
        let auto_open_count = self
            .tunnels
            .iter()
            .filter(|tunnel| tunnel.auto_open)
            .count();
        let proxy_command_count = self
            .proxies
            .iter()
            .filter(|proxy| proxy.protocol == "proxycommand")
            .count();

        let mut tunnel_list = div().flex().flex_col().gap_3();
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

        let mut proxy_list = div().flex().flex_col().gap_3();
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

        div()
            .flex()
            .flex_col()
            .size_full()
            .p_5()
            .gap_4()
            .child(section_header(
                "Network",
                "Proxy and SSH tunnel profiles grouped like the legacy network panel.",
            ))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(network_tab_button(
                        "network-tab-tunnels",
                        "Tunnels",
                        self.network_tab == NetworkTab::Tunnels,
                        cx.listener(|this, _, _, cx| {
                            this.set_network_tab(NetworkTab::Tunnels, cx);
                        }),
                    ))
                    .child(network_tab_button(
                        "network-tab-proxies",
                        "Proxies",
                        self.network_tab == NetworkTab::Proxies,
                        cx.listener(|this, _, _, cx| {
                            this.set_network_tab(NetworkTab::Proxies, cx);
                        }),
                    ))
                    .child(small_button(
                        "network-group-new",
                        "New Group",
                        cx.listener(|this, _, _, cx| {
                            this.open_network_group_editor(this.network_tab, None, cx);
                        }),
                    ))
                    .when(self.network_tab == NetworkTab::Tunnels, |this| {
                        this.child(small_button(
                            "network-tunnel-new",
                            "New Tunnel",
                            cx.listener(|this, _, window, cx| {
                                this.open_network_tunnel_editor(None, window, cx);
                            }),
                        ))
                    })
                    .when(self.network_tab == NetworkTab::Proxies, |this| {
                        this.child(small_button(
                            "network-proxy-new",
                            "New Proxy",
                            cx.listener(|this, _, window, cx| {
                                this.open_network_proxy_editor(None, window, cx);
                            }),
                        ))
                    })
            )
            .child(
                div()
                    .grid()
                    .grid_cols(5)
                    .gap_3()
                    .child(metric(
                        "Profiles",
                        match self.network_tab {
                            NetworkTab::Tunnels => self.tunnels.len(),
                            NetworkTab::Proxies => self.proxies.len(),
                        }
                        .to_string(),
                    ))
                    .child(metric(
                        "Visible",
                        match self.network_tab {
                            NetworkTab::Tunnels => filtered_tunnels.len(),
                            NetworkTab::Proxies => filtered_proxies.len(),
                        }
                        .to_string(),
                    ))
                    .child(metric(
                        "Groups",
                        match self.network_tab {
                            NetworkTab::Tunnels => self.tunnel_groups.len(),
                            NetworkTab::Proxies => self.proxy_groups.len(),
                        }
                        .to_string(),
                    ))
                    .child(metric(
                        if self.network_tab == NetworkTab::Tunnels {
                            "Open"
                        } else {
                            "SOCKS/HTTP"
                        },
                        if self.network_tab == NetworkTab::Tunnels {
                            open_tunnels.len().to_string()
                        } else {
                            self.proxies
                                .iter()
                                .filter(|proxy| proxy.protocol != "proxycommand")
                                .count()
                                .to_string()
                        },
                    ))
                    .child(metric(
                        if self.network_tab == NetworkTab::Tunnels {
                            "Auto"
                        } else {
                            "Command"
                        },
                        if self.network_tab == NetworkTab::Tunnels {
                            auto_open_count.to_string()
                        } else {
                            proxy_command_count.to_string()
                        },
                    )),
            )
            .when(
                self.network_tab == NetworkTab::Tunnels && missing_connections > 0,
                |this| {
                this.child(
                    div()
                        .rounded_md()
                        .border_1()
                        .border_color(rgb(0xfacc15))
                        .bg(rgb(0x2f260f))
                        .p_3()
                        .text_sm()
                        .text_color(rgb(0xfef3c7))
                        .child(format!(
                            "{missing_connections} tunnel profile(s) reference missing SSH connections."
                        )),
                )
            })
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
            .child(
                div()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(0x2a3140))
                    .bg(rgb(0x151923))
                    .p_3()
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
                    .child(status_pill(
                        if self.network_tab == NetworkTab::Tunnels {
                            "Tunnel"
                        } else {
                            "Proxy"
                        },
                        rgb(0x93c5fd),
                        rgb(0x17233a),
                    ))
                    .child(
                        div()
                            .font_family("JetBrains Mono")
                            .text_xs()
                            .text_color(rgb(0x98a3b8))
                            .child(match self.network_tab {
                                NetworkTab::Tunnels => {
                                    format!("{}/{}", filtered_tunnels.len(), self.tunnels.len())
                                }
                                NetworkTab::Proxies => {
                                    format!("{}/{}", filtered_proxies.len(), self.proxies.len())
                                }
                            }),
                    ),
            )
            .child(match self.network_tab {
                NetworkTab::Tunnels => tunnel_list.into_any_element(),
                NetworkTab::Proxies => proxy_list.into_any_element(),
            })
    }
}
