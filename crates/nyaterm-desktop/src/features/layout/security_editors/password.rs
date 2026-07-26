use super::*;

use crate::models::{SecurityPasswordEditorField, SecurityPasswordEditorState};

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
        let password_display = if editor.password.is_empty() {
            if editor.has_password {
                self.tr("passwordManager.passwordUnchanged").to_string()
            } else {
                " ".to_string()
            }
        } else if editor.show_password {
            truncate_preview(&editor.password, 48)
        } else {
            "•".repeat(editor.password.chars().count().min(24))
        };
        div()
            .rounded_md()
            .border_1()
            .border_color(rgb(palette.border))
            .bg(rgb(palette.bg))
            .p_3()
            .flex()
            .flex_col()
            .gap_2()
            .track_focus(&self.security_password_editor_focus)
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
                palette,
                "security-pw-name",
                self.tr("passwordManager.nameLabel"),
                if editor.name.is_empty() {
                    " ".to_string()
                } else {
                    editor.name.clone()
                },
                editor.focused_field == SecurityPasswordEditorField::Name,
                cx.listener(|this, _, window, cx| {
                    this.focus_security_password_field(
                        SecurityPasswordEditorField::Name,
                        window,
                        cx,
                    );
                }),
            ))
            .child(
                div()
                    .flex()
                    .items_end()
                    .gap_1()
                    .child(div().min_w_0().flex_1().child(security_editor_field(
                        palette,
                        "security-pw-value",
                        self.tr("passwordManager.passwordLabel"),
                        password_display,
                        editor.focused_field == SecurityPasswordEditorField::Password,
                        cx.listener(|this, _, window, cx| {
                            this.focus_security_password_field(
                                SecurityPasswordEditorField::Password,
                                window,
                                cx,
                            );
                        }),
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
