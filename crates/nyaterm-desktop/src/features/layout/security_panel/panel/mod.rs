use super::*;

use crate::models::{SecurityAuthTab, SecurityDeleteConfirmState};

mod credentials;
mod keys;
mod otp;
mod passwords;

impl NyaTermApp {
    pub(in crate::features) fn security_auth_panel(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let active_tab = self.security.auth_tab;
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
                div()
                    .px_3()
                    .pt_3()
                    .pb_0()
                    .flex()
                    .flex_col()
                    // Tauri SecurityAuthPanel: full-width 4-col segment tabs under PanelHeader.
                    .child(
                        div()
                            .h(px(32.))
                            .w_full()
                            .rounded_md()
                            .bg(rgb(palette.surface_elevated))
                            .p(px(2.))
                            .flex()
                            .items_center()
                            .gap(px(2.))
                            .child(self.security_tab_chip(SecurityAuthTab::Keys, cx))
                            .child(self.security_tab_chip(SecurityAuthTab::Passwords, cx))
                            .child(self.security_tab_chip(SecurityAuthTab::Otp, cx))
                            .child(self.security_tab_chip(SecurityAuthTab::Credentials, cx)),
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
            .when_some(self.security.delete_confirm.clone(), |this, confirm| {
                this.child(self.security_delete_confirm_panel(confirm, cx))
            })
            .when(self.security.unlock.prompt_open, |this| {
                this.child(self.security_unlock_prompt(cx))
            })
            .when(self.security.unlock.master_required_prompt_open, |this| {
                this.child(self.security_master_required_prompt(cx))
            })
    }

    fn security_delete_confirm_panel(
        &mut self,
        confirm: SecurityDeleteConfirmState,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let (title_key, description_key) = match confirm.kind {
            SecurityAuthTab::Keys => ("settings.deleteKey", "settings.deleteKeyConfirm"),
            SecurityAuthTab::Passwords => (
                "passwordManager.deleteTitle",
                "passwordManager.deleteConfirm",
            ),
            SecurityAuthTab::Credentials => (
                "credentialManager.deleteTitle",
                "credentialManager.deleteConfirm",
            ),
            SecurityAuthTab::Otp => ("otpManager.deleteTitle", "otpManager.deleteConfirm"),
        };
        let description = self.tr(description_key).replace("{{name}}", &confirm.label);
        let card = div()
            .p_6()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .text_size(px(15.))
                    .font_weight(FontWeight(700.))
                    .text_color(rgb(palette.text))
                    .child(self.tr(title_key)),
            )
            .child(
                div()
                    .text_size(px(12.))
                    .line_height(px(18.))
                    .text_color(rgb(palette.text_muted))
                    .child(description),
            )
            .child(modal_dialog_footer_localized_danger(
                palette,
                "security-delete-cancel",
                "security-delete-confirm",
                self.tr("common.cancel"),
                self.tr("common.delete"),
                cx.listener(|this, _, _, cx| {
                    this.cancel_security_delete(cx);
                }),
                cx.listener(|this, _, _, cx| {
                    this.confirm_security_delete(cx);
                }),
            ));
        modal_dialog_shell(
            palette,
            self.shell_surface_color(palette.bg),
            "security-delete-confirm-modal",
            384.,
            card,
        )
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
