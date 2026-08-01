//! Controlled, reusable selects keyed by stable feature ids.
//!
//! Persisted feature state remains authoritative for the selected value. The
//! component entity owns focus and popup state, and emits only committed value
//! changes back to the application.

use std::collections::HashMap;

use gpui::{
    App, AppContext, Context, Entity, InteractiveElement as _, IntoElement, ParentElement as _,
    SharedString, Styled as _, Subscription, Window, div, px,
};
use nyaterm_core::RiskLevel;
use nyaterm_ui::{
    NYA_FORM_CONTROL_HEIGHT_PX, NyaSelect, NyaSelectEvent, NyaSelectOption, NyaSelectState,
};

use super::NyaTermApp;
use crate::models::ConnectionEditorSelect;
use crate::send_command::{
    SendCommandDataType, SendCommandLineEnding, SendCommandMode, SendCommandTarget,
};
use crate::temporary_ssh_link::TemporaryLinkProtocol;

pub(in crate::features) const FOLLOW_UI_THEME_VALUE: &str = "__nya_follow_ui_theme__";
pub(in crate::features) const NO_SELECTION_VALUE: &str = "__nya_no_selection__";
pub(in crate::features) const PENDING_CONNECTION_GROUP_VALUE: &str =
    "__nya_pending_connection_group__";

#[derive(Default)]
pub(in crate::features) struct SelectRegistry {
    fields: HashMap<SharedString, Entity<NyaSelectState>>,
    subscriptions: HashMap<SharedString, Subscription>,
}

impl NyaTermApp {
    pub(in crate::features) fn select_entity<I>(
        &mut self,
        id: I,
        options: Vec<NyaSelectOption>,
        selected_value: Option<String>,
        disabled: bool,
        cx: &mut Context<Self>,
    ) -> Entity<NyaSelectState>
    where
        I: Into<SharedString>,
    {
        let id = id.into();
        let select = if let Some(select) = self.selects.fields.get(&id) {
            select.clone()
        } else {
            let select = cx.new(|cx| {
                NyaSelectState::new(cx, options.clone(), selected_value.clone()).disabled(disabled)
            });
            let subscription_id = id.clone();
            let subscription =
                cx.subscribe(
                    &select,
                    move |app: &mut NyaTermApp, _, event, cx| match event {
                        NyaSelectEvent::Changed(value) => {
                            app.on_select_changed(&subscription_id, value.as_deref(), cx);
                        }
                    },
                );
            self.selects.fields.insert(id.clone(), select.clone());
            self.selects.subscriptions.insert(id, subscription);
            select
        };

        select.update(cx, |select, cx| {
            select.set_options(options, cx);
            select.set_selected_value(selected_value, cx);
            select.set_disabled(disabled, cx);
        });
        select
    }

    pub(in crate::features) fn select_is_focused(
        &self,
        id: &str,
        window: &Window,
        cx: &App,
    ) -> bool {
        self.selects
            .fields
            .get(id)
            .is_some_and(|select| select.read(cx).is_focused(window, cx))
    }

    pub(in crate::features) fn select_menu_is_focused(
        &self,
        id: &str,
        window: &Window,
        cx: &App,
    ) -> bool {
        self.selects
            .fields
            .get(id)
            .is_some_and(|select| select.read(cx).is_menu_focused(window, cx))
    }

    pub(in crate::features) fn select_control<I>(
        &mut self,
        id: I,
        options: Vec<NyaSelectOption>,
        selected_value: Option<String>,
        disabled: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<I>
    where
        I: Into<SharedString>,
    {
        self.select_control_with_appearance(id, options, selected_value, disabled, true, cx)
    }

    pub(in crate::features) fn bare_select_control<I>(
        &mut self,
        id: I,
        options: Vec<NyaSelectOption>,
        selected_value: Option<String>,
        disabled: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<I>
    where
        I: Into<SharedString>,
    {
        self.select_control_with_appearance(id, options, selected_value, disabled, false, cx)
    }

    pub(in crate::features) fn form_select_control<I>(
        &mut self,
        id: I,
        options: Vec<NyaSelectOption>,
        selected_value: Option<String>,
        disabled: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<I>
    where
        I: Into<SharedString>,
    {
        let id = id.into();
        let select = self.select_entity(id.clone(), options, selected_value, disabled, cx);

        div()
            .id(id)
            .w_full()
            .h(px(NYA_FORM_CONTROL_HEIGHT_PX))
            .child(NyaSelect::new(&select))
    }

    fn select_control_with_appearance<I>(
        &mut self,
        id: I,
        options: Vec<NyaSelectOption>,
        selected_value: Option<String>,
        disabled: bool,
        appearance: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<I>
    where
        I: Into<SharedString>,
    {
        let id = id.into();
        let select = self.select_entity(id.clone(), options, selected_value, disabled, cx);

        div()
            .id(id)
            .w_full()
            .max_w(px(360.))
            .h(px(NYA_FORM_CONTROL_HEIGHT_PX))
            .child(NyaSelect::new(&select).appearance(appearance))
    }

    fn on_select_changed(&mut self, id: &str, value: Option<&str>, cx: &mut Context<Self>) {
        let Some(value) = value else {
            return;
        };

        match id {
            "appearance-ui-theme" => self.update_appearance_theme(value, cx),
            "appearance-terminal-theme" => {
                self.set_terminal_theme((value != FOLLOW_UI_THEME_VALUE).then_some(value), cx)
            }
            "appearance-minimum-contrast" => self.set_minimum_contrast_ratio(value, cx),
            "appearance-background-fit" => self.set_background_image_fit(value, cx),
            "appearance-terminal-font-weight" => {
                if let Ok(weight) = value.parse() {
                    self.set_terminal_font_weight(weight, cx);
                }
            }
            "appearance-terminal-font-weight-bold" => {
                if let Ok(weight) = value.parse() {
                    self.set_terminal_font_weight_bold(weight, cx);
                }
            }
            "appearance-cursor-style" => self.set_cursor_style(value, cx),
            "network-tunnel-editor-type" => self.set_network_tunnel_type(value, cx),
            "network-tunnel-editor-connection" => self.set_network_tunnel_connection(
                (value != NO_SELECTION_VALUE).then(|| value.to_string()),
                cx,
            ),
            "network-tunnel-editor-group" => self.set_network_tunnel_group(
                (value != NO_SELECTION_VALUE).then(|| value.to_string()),
                cx,
            ),
            "network-proxy-editor-protocol" => self.set_network_proxy_protocol(value, cx),
            "network-proxy-editor-group" => self.set_network_proxy_group(
                (value != NO_SELECTION_VALUE).then(|| value.to_string()),
                cx,
            ),
            "bottom-command-data-select" => {
                let data_type = match value {
                    "hex" => SendCommandDataType::Hex,
                    _ => SendCommandDataType::Text,
                };
                self.set_send_command_data_type(data_type, cx);
            }
            "bottom-command-mode-select" => {
                let mode = match value {
                    "byte" => SendCommandMode::Byte,
                    "packet" => SendCommandMode::Packet,
                    "character" => SendCommandMode::Character,
                    _ => SendCommandMode::Line,
                };
                self.set_send_command_mode(mode, cx);
            }
            "bottom-command-target-select" => {
                let target = match value {
                    "all" => SendCommandTarget::AllCompatible,
                    value if value.starts_with("group:") => {
                        SendCommandTarget::Group(value[6..].to_string())
                    }
                    _ => SendCommandTarget::Current,
                };
                self.set_send_command_target(target, cx);
            }
            "bottom-command-eol-select" => {
                let line_ending = match value {
                    "none" => SendCommandLineEnding::None,
                    "cr" => SendCommandLineEnding::Cr,
                    "lf" => SendCommandLineEnding::Lf,
                    _ => SendCommandLineEnding::Crlf,
                };
                self.set_send_command_line_ending(line_ending, cx);
            }
            "temporary-link-protocol" => {
                let protocol = match value {
                    "telnet" => TemporaryLinkProtocol::Telnet,
                    "serial" => TemporaryLinkProtocol::Serial,
                    _ => TemporaryLinkProtocol::Ssh,
                };
                self.set_temporary_link_protocol(protocol, cx);
            }
            "temporary-link-serial-port" => {
                self.apply_temporary_serial_port_name(value.to_string(), cx);
            }
            "cloud-provider-select" => self.update_cloud_sync_provider(value, cx),
            "ai-smart-risk" => {
                let risk = match value {
                    "low" => Some(RiskLevel::Low),
                    "medium" => Some(RiskLevel::Medium),
                    "high" => Some(RiskLevel::High),
                    "critical" => Some(RiskLevel::Critical),
                    _ => None,
                };
                if let Some(risk) = risk {
                    self.update_ai_smart_auto_execute_max_risk(risk, cx);
                }
            }
            id if id.starts_with("connection-editor-") => {
                let select = match id {
                    "connection-editor-group-select" => ConnectionEditorSelect::Group,
                    "connection-editor-saved-password" => ConnectionEditorSelect::SavedPassword,
                    "connection-editor-ssh-key" => ConnectionEditorSelect::SshKey,
                    "connection-editor-otp" => ConnectionEditorSelect::Otp,
                    "connection-editor-proxy" => ConnectionEditorSelect::Proxy,
                    "connection-editor-proxy-jump" => ConnectionEditorSelect::ProxyJump,
                    "connection-editor-backspace" => ConnectionEditorSelect::Backspace,
                    "connection-editor-telnet-enter-mode" => {
                        ConnectionEditorSelect::TelnetEnterMode
                    }
                    "connection-editor-shell" => ConnectionEditorSelect::Shell,
                    "connection-editor-serial-port" => ConnectionEditorSelect::SerialPort,
                    "connection-editor-baud-rate" => ConnectionEditorSelect::BaudRate,
                    "connection-editor-data-bits" => ConnectionEditorSelect::DataBits,
                    "connection-editor-parity" => ConnectionEditorSelect::Parity,
                    "connection-editor-stop-bits" => ConnectionEditorSelect::StopBits,
                    _ => return,
                };
                if select == ConnectionEditorSelect::Group
                    && value == PENDING_CONNECTION_GROUP_VALUE
                {
                    return;
                }
                self.set_connection_editor_select_value(
                    select,
                    (value != NO_SELECTION_VALUE).then_some(value),
                    cx,
                );
            }
            _ => {
                let font = id
                    .strip_prefix("appearance-terminal-font-")
                    .and_then(|index| index.parse::<usize>().ok())
                    .map(|index| (true, index))
                    .or_else(|| {
                        id.strip_prefix("appearance-ui-font-")
                            .and_then(|index| index.parse::<usize>().ok())
                            .map(|index| (false, index))
                    });
                if let Some((terminal, index)) = font {
                    self.set_appearance_font_stack_entry(terminal, index, value.to_string(), cx);
                }
            }
        }
    }
}
