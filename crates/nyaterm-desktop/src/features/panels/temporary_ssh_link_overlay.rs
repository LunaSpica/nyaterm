use gpui::{
    AnyElement, Context, IntoElement as _, ParentElement as _, SharedString, Styled as _, div,
    prelude::{FluentBuilder as _, InteractiveElement as _, StatefulInteractiveElement as _},
    px, rgb, rgba,
};
use nyaterm_ui::NyaScrollArea;

use crate::features::{NyaTermApp, TextInputSetup};
use crate::temporary_ssh_link::{
    TemporaryLinkProtocol, build_temporary_serial_link, parse_temporary_ssh_link,
    parse_temporary_telnet_link,
};

impl NyaTermApp {
    pub(in crate::features) fn temporary_ssh_link_dialog_content(
        &mut self,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let palette = self.theme_palette();
        let protocol = self.session.dialog_temporary_link_protocol();
        let draft = self.session.dialog_temporary_ssh_link_draft().to_string();
        let serial_port = self.session.dialog_temporary_serial_port_name().to_string();
        let serial_baud_rate = self.session.dialog_temporary_serial_baud_rate().to_string();
        let error_key =
            temporary_link_error_key(self, protocol, &draft, &serial_port, &serial_baud_rate);

        div()
            .min_w_0()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(palette.text_muted))
                    .line_height(px(16.))
                    .child(self.tr("temporarySsh.description")),
            )
            .child(
                div().flex().items_center().gap_1().children([
                    temporary_link_protocol_button(
                        self,
                        protocol,
                        TemporaryLinkProtocol::Ssh,
                        "temporarySsh.protocolSsh",
                        cx,
                    )
                    .into_any_element(),
                    temporary_link_protocol_button(
                        self,
                        protocol,
                        TemporaryLinkProtocol::Telnet,
                        "temporarySsh.protocolTelnet",
                        cx,
                    )
                    .into_any_element(),
                    temporary_link_protocol_button(
                        self,
                        protocol,
                        TemporaryLinkProtocol::Serial,
                        "temporarySsh.protocolSerial",
                        cx,
                    )
                    .into_any_element(),
                ]),
            )
            .when(protocol != TemporaryLinkProtocol::Serial, |this| {
                let placeholder = match protocol {
                    TemporaryLinkProtocol::Ssh => self.tr("temporarySsh.placeholder"),
                    TemporaryLinkProtocol::Telnet => self.tr("temporarySsh.telnetPlaceholder"),
                    TemporaryLinkProtocol::Serial => self.tr("temporarySsh.placeholder"),
                };
                this.child(
                    self.text_input_box(
                        "temporary-ssh.link",
                        &draft,
                        TextInputSetup::placeholder(placeholder),
                        cx,
                    )
                    .into_any_element(),
                )
            })
            .when(protocol == TemporaryLinkProtocol::Serial, |this| {
                this.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(
                            self.text_input_box(
                                "temporary-ssh.serial-port",
                                &serial_port,
                                TextInputSetup::placeholder(
                                    self.tr("temporarySsh.serialPortPlaceholder"),
                                ),
                                cx,
                            )
                            .into_any_element(),
                        )
                        .child(
                            self.text_input_box(
                                "temporary-ssh.baud-rate",
                                &serial_baud_rate,
                                TextInputSetup::placeholder(
                                    self.tr("temporarySsh.baudRatePlaceholder"),
                                ),
                                cx,
                            )
                            .into_any_element(),
                        )
                        .when(!self.connection_state.serial_ports().is_empty(), |this| {
                            this.child(
                                NyaScrollArea::new("temporary-serial-port-list")
                                    .max_h(px(96.))
                                    .child(div().flex().flex_col().gap_1().children(
                                        self.connection_state.serial_ports().iter().cloned().map(
                                            |port| {
                                                let selected = port == serial_port;
                                                let port_for_click = port.clone();
                                                div()
                                                    .id(SharedString::from(format!(
                                                        "temporary-serial-port-{port}"
                                                    )))
                                                    .h(px(28.))
                                                    .px_2()
                                                    .rounded_sm()
                                                    .flex()
                                                    .items_center()
                                                    .text_xs()
                                                    .text_color(rgb(if selected {
                                                        palette.primary
                                                    } else {
                                                        palette.text
                                                    }))
                                                    .bg(if selected {
                                                        rgba((palette.primary << 8) | 0x18)
                                                    } else {
                                                        rgba(0x00000000)
                                                    })
                                                    .cursor_pointer()
                                                    .hover(move |this| this.bg(rgb(palette.hover)))
                                                    .on_click(cx.listener(move |this, _, _, cx| {
                                                        this.apply_temporary_serial_port_name(
                                                            port_for_click.clone(),
                                                            cx,
                                                        );
                                                    }))
                                                    .child(port)
                                            },
                                        ),
                                    )),
                            )
                        }),
                )
            })
            .when_some(error_key, |this, key| {
                this.child(
                    div()
                        .text_size(px(12.))
                        .text_color(rgb(palette.danger))
                        .child(self.tr(key)),
                )
            })
            .into_any_element()
    }
}

fn temporary_link_error_key(
    app: &NyaTermApp,
    protocol: TemporaryLinkProtocol,
    draft: &str,
    serial_port: &str,
    serial_baud_rate: &str,
) -> Option<&'static str> {
    app.session
        .dialog_temporary_ssh_link_error()
        .or_else(|| match protocol {
            TemporaryLinkProtocol::Ssh => {
                if draft.trim().is_empty() {
                    None
                } else {
                    parse_temporary_ssh_link(draft)
                        .as_ref()
                        .err()
                        .map(|error| error.locale_key())
                }
            }
            TemporaryLinkProtocol::Telnet => {
                if draft.trim().is_empty() {
                    None
                } else {
                    parse_temporary_telnet_link(draft)
                        .as_ref()
                        .err()
                        .map(|error| error.locale_key())
                }
            }
            TemporaryLinkProtocol::Serial => {
                if serial_port.trim().is_empty() && serial_baud_rate.trim() == "115200" {
                    None
                } else {
                    build_temporary_serial_link(serial_port, serial_baud_rate)
                        .as_ref()
                        .err()
                        .map(|error| error.locale_key())
                }
            }
        })
}

fn temporary_link_protocol_button(
    app: &NyaTermApp,
    active: TemporaryLinkProtocol,
    protocol: TemporaryLinkProtocol,
    label_key: &'static str,
    cx: &mut Context<NyaTermApp>,
) -> impl gpui::IntoElement {
    let palette = app.theme_palette();
    let selected = active == protocol;
    div()
        .id(SharedString::from(format!(
            "temporary-link-protocol-{}",
            protocol.as_str()
        )))
        .h(px(28.))
        .px_2()
        .rounded_sm()
        .border_1()
        .border_color(rgb(if selected {
            palette.primary
        } else {
            palette.border
        }))
        .bg(if selected {
            rgba((palette.primary << 8) | 0x18)
        } else {
            rgba(0x00000000)
        })
        .flex()
        .items_center()
        .justify_center()
        .text_xs()
        .text_color(rgb(if selected {
            palette.primary
        } else {
            palette.text_muted
        }))
        .cursor_pointer()
        .hover(move |this| this.bg(rgb(palette.hover)))
        .on_click(cx.listener(move |this, _, _, cx| {
            this.set_temporary_link_protocol(protocol, cx);
        }))
        .child(app.tr(label_key))
}
