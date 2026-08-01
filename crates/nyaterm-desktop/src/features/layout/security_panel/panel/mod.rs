use gpui::{
    App, ClickEvent, Context, FontWeight, IntoElement, SharedString, Window, div, prelude::*, px,
    rgb,
};
use nyaterm_ui::{NyaTabItem, NyaTabs};

use crate::features::NyaTermApp;
use crate::models::SecurityAuthTab;
use crate::theme::ThemePalette;

mod credentials;
mod keys;
mod otp;
mod passwords;

impl NyaTermApp {
    pub(in crate::features) fn security_auth_panel(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let active_tab = self.security.auth_tab();
        let palette = self.theme_palette();

        let body = match active_tab {
            SecurityAuthTab::Keys => self.security_keys_body(palette, cx),
            SecurityAuthTab::Passwords => self.security_passwords_body(palette, cx),
            SecurityAuthTab::Credentials => self.security_credentials_body(palette, cx),
            SecurityAuthTab::Otp => self.security_otp_body(palette, cx),
        };

        div()
            .size_full()
            .relative()
            .flex()
            .flex_col()
            .bg(self.shell_transparent_color(palette.surface))
            .child(
                div().px_3().pt_3().pb_0().flex().flex_col().child(
                    NyaTabs::new("security-auth-tabs")
                        .items([
                            NyaTabItem::new(self.tr(SecurityAuthTab::Keys.i18n_key())),
                            NyaTabItem::new(self.tr(SecurityAuthTab::Passwords.i18n_key())),
                            NyaTabItem::new(self.tr(SecurityAuthTab::Otp.i18n_key())),
                            NyaTabItem::new(self.tr(SecurityAuthTab::Credentials.i18n_key())),
                        ])
                        .selected_index(match active_tab {
                            SecurityAuthTab::Keys => 0,
                            SecurityAuthTab::Passwords => 1,
                            SecurityAuthTab::Otp => 2,
                            SecurityAuthTab::Credentials => 3,
                        })
                        .on_select(cx.listener(|this, index, _, cx| {
                            let tab = match *index {
                                0 => SecurityAuthTab::Keys,
                                1 => SecurityAuthTab::Passwords,
                                2 => SecurityAuthTab::Otp,
                                _ => SecurityAuthTab::Credentials,
                            };
                            this.set_security_auth_tab(tab, cx);
                        })),
                ),
            )
            .child(body)
            .when(
                matches!(
                    active_tab,
                    SecurityAuthTab::Passwords | SecurityAuthTab::Credentials
                ),
                |this| this.child(self.security_secret_footer(cx)),
            )
            .when(self.security.unlock_prompt_open(), |this| {
                this.child(self.security_unlock_prompt(cx))
            })
            .when(self.security.master_required_prompt_open(), |this| {
                this.child(self.security_master_required_prompt(cx))
            })
    }
}

fn security_auth_body_base(id: &'static str) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .flex_1()
        .min_h_0()
        .overflow_y_scroll()
        .flex()
        .flex_col()
        .gap_2()
        .px_3()
        .pt_3()
        .pb_3()
}

fn security_tab_toolbar(
    palette: ThemePalette,
    title: &'static str,
    add_id: impl Into<String>,
    add_label: &'static str,
    enabled: bool,
    on_add: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> gpui::Div {
    div()
        .flex_none()
        .h(px(28.))
        .flex()
        .items_center()
        .justify_between()
        .gap_2()
        .child(
            div()
                .text_xs()
                .font_weight(FontWeight(600.))
                .text_color(rgb(palette.text))
                .child(title),
        )
        .child(security_toolbar_action_button(
            palette, add_id, add_label, enabled, on_add,
        ))
}

fn security_toolbar_action_button(
    palette: ThemePalette,
    id: impl Into<String>,
    label: &'static str,
    enabled: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(SharedString::from(id.into()))
        .h(px(26.))
        .px_2()
        .rounded_md()
        .flex()
        .items_center()
        .text_size(px(11.))
        .font_weight(FontWeight(600.))
        .text_color(rgb(if enabled {
            palette.link
        } else {
            palette.text_dimmed
        }))
        .when(enabled, |this| {
            this.cursor_pointer().hover(|this| {
                this.bg(rgb(palette.surface_elevated))
                    .text_color(rgb(palette.text))
            })
        })
        .when(!enabled, |this| this.opacity(0.45))
        .child(label)
        .on_click(move |event, window, cx| {
            if enabled {
                on_click(event, window, cx);
            }
        })
}
