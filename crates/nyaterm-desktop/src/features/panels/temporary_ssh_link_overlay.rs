use gpui::{
    AnyElement, Context, IntoElement as _, ParentElement as _, Styled as _, div,
    prelude::FluentBuilder as _, px, rgb,
};

use crate::features::{NyaTermApp, TextInputSetup};
use crate::temporary_ssh_link::parse_temporary_ssh_link;

impl NyaTermApp {
    pub(in crate::features) fn temporary_ssh_link_dialog_content(
        &mut self,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let palette = self.theme_palette();
        let draft = self.session.dialog_temporary_ssh_link_draft().to_string();
        let input = self
            .text_input_box(
                "temporary-ssh.link",
                &draft,
                TextInputSetup::placeholder(self.tr("temporarySsh.placeholder")),
                cx,
            )
            .into_any_element();
        let parsed = parse_temporary_ssh_link(&draft);
        let error_key = self.session.dialog_temporary_ssh_link_error().or_else(|| {
            if draft.trim().is_empty() {
                None
            } else {
                parsed.as_ref().err().map(|error| error.locale_key())
            }
        });

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
            .child(input)
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
