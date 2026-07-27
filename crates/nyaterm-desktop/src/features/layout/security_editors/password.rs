use super::*;

use crate::models::SecurityPasswordEditorState;

impl NyaTermApp {
    pub(in crate::features) fn security_password_editor_view(
        &mut self,
        editor: SecurityPasswordEditorState,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let title = if editor.id.is_some() {
            self.tr("passwordManager.editTitle")
        } else {
            self.tr("passwordManager.newTitle")
        };
        // A stored secret is never shown, so the box says so in its
        // placeholder rather than standing a row of bullets in for it. The
        // reveal toggle now unmasks the box itself.
        let password_placeholder = if editor.has_password {
            self.tr("passwordManager.passwordUnchanged")
        } else {
            ""
        };
        let password_masked = !editor.show_password;
        div()
            .rounded_md()
            .border_1()
            .border_color(rgb(palette.border))
            .bg(rgb(palette.bg))
            .p_3()
            .flex()
            .flex_col()
            .gap_2()
            .track_focus(&self.security.editors.password_focus)
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                this.handle_security_password_editor_key_down(event, window, cx);
            }))
            .child(
                div()
                    .text_xs()
                    .font_weight(FontWeight(800.))
                    .text_color(rgb(palette.text))
                    .child(title),
            )
            .child(security_editor_field(
                self,
                "pw-name",
                self.tr("passwordManager.nameLabel"),
                editor.name.clone(),
                TextInputSetup::default(),
                cx,
            ))
            .child(
                div()
                    .flex()
                    .items_end()
                    .gap_1()
                    .child(div().min_w_0().flex_1().child(security_editor_field(
                        self,
                        "pw-value",
                        self.tr("passwordManager.passwordLabel"),
                        editor.password.clone(),
                        TextInputSetup {
                            placeholder: password_placeholder.into(),
                            masked: password_masked,
                            multi_line: false,
                        },
                        cx,
                    )))
                    .child(small_button(
                        palette,
                        "security-pw-toggle-vis",
                        self.tr(if editor.show_password {
                            "passwordManager.hidePassword"
                        } else {
                            "passwordManager.showPassword"
                        }),
                        cx.listener(|this, _, _, cx| {
                            this.toggle_security_password_editor_visibility(cx);
                        }),
                    )),
            )
            .when_some(editor.error.clone(), |this, error| {
                this.child(
                    div()
                        .text_size(px(10.))
                        .text_color(rgb(palette.danger))
                        .child(error),
                )
            })
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(small_button(
                        palette,
                        "security-pw-save",
                        self.tr("common.save"),
                        cx.listener(|this, _, window, cx| {
                            this.save_security_password_editor(window, cx);
                        }),
                    ))
                    .child(small_button(
                        palette,
                        "security-pw-cancel",
                        self.tr("common.cancel"),
                        cx.listener(|this, _, _, cx| {
                            this.close_security_password_editor(cx);
                        }),
                    )),
            )
    }
}
