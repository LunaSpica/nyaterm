use gpui::{
    Context, FontWeight, div,
    prelude::{FluentBuilder, ParentElement, Styled},
    px, rgb,
};
use nyaterm_ui::{NyaTabItem, NyaTabs};

use crate::features::NyaTermApp;
use crate::models::{
    ConnectionEditorField, ConnectionEditorPasswordSource, ConnectionEditorSelect,
};

use super::super::super::list::{
    ConnectionEditorRenderContext, connection_editor_select, editor_field, editor_stepper_field,
    required,
};
use super::ConnectionEditorSectionContext;

pub(super) fn connection_editor_rdp_section(
    section: ConnectionEditorSectionContext<'_>,
    cx: &mut Context<NyaTermApp>,
) -> gpui::Div {
    let ConnectionEditorSectionContext {
        palette,
        editor,
        language,
        fields,
    } = section;
    let tr = |key: &'static str| crate::i18n::text(language, key);
    let auth_values = vec!["none".to_string(), "password".to_string()];
    let auth_tabs = NyaTabs::new("connection-rdp-auth-tabs")
        .items([
            NyaTabItem::new(tr("dialog.noAuthentication")),
            NyaTabItem::new(tr("dialog.password")),
        ])
        .selected_index(if editor.auth_mode == "none" { 0 } else { 1 })
        .on_select(cx.listener(move |this, index: &usize, _, cx| {
            let Some(value) = auth_values.get(*index) else {
                return;
            };
            this.set_connection_editor_select_value(
                ConnectionEditorSelect::Authentication,
                Some(value.as_str()),
                cx,
            );
        }));
    let password_source_tabs = NyaTabs::new("connection-rdp-password-source-tabs")
        .items([
            NyaTabItem::new(tr("dialog.askWhenConnecting")),
            NyaTabItem::new(tr("dialog.directPassword")),
            NyaTabItem::new(tr("dialog.savedPassword")),
        ])
        .selected_index(match editor.password_source {
            ConnectionEditorPasswordSource::Ask => 0,
            ConnectionEditorPasswordSource::Direct => 1,
            ConnectionEditorPasswordSource::Saved => 2,
        })
        .on_select(cx.listener(|this, index, _, cx| {
            let source = match *index {
                0 => ConnectionEditorPasswordSource::Ask,
                1 => ConnectionEditorPasswordSource::Direct,
                _ => ConnectionEditorPasswordSource::Saved,
            };
            this.set_connection_editor_password_source(source, cx);
        }));

    div()
        .flex()
        .flex_col()
        .gap_3()
        .child(
            div()
                .flex()
                .gap_3()
                .child(div().min_w_0().flex_1().child(editor_field(
                    palette,
                    required(tr("dialog.host")),
                    ConnectionEditorField::Host,
                    fields,
                    cx,
                )))
                .child(div().w(px(150.)).flex_none().child(editor_stepper_field(
                    palette,
                    required(tr("dialog.port")),
                    ConnectionEditorField::Port,
                    fields,
                    cx,
                ))),
        )
        .child(editor_field(
            palette,
            tr("dialog.username"),
            ConnectionEditorField::Username,
            fields,
            cx,
        ))
        .child(editor_field(
            palette,
            tr("dialog.rdpDomain"),
            ConnectionEditorField::Domain,
            fields,
            cx,
        ))
        .child(
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(
                    div()
                        .text_xs()
                        .font_weight(FontWeight(500.))
                        .text_color(rgb(palette.text_muted))
                        .child(tr("dialog.authentication")),
                )
                .child(auth_tabs)
                .when(editor.auth_mode != "none", |this| {
                    this.child(password_source_tabs)
                        .when(
                            editor.password_source == ConnectionEditorPasswordSource::Direct,
                            |this| {
                                this.child(editor_field(
                                    palette,
                                    tr("dialog.password"),
                                    ConnectionEditorField::Password,
                                    fields,
                                    cx,
                                ))
                            },
                        )
                        .when(
                            editor.password_source == ConnectionEditorPasswordSource::Saved,
                            |this| {
                                this.child(connection_editor_select(
                                    ConnectionEditorRenderContext {
                                        palette,
                                        fields,
                                        cx,
                                    },
                                    "connection-editor-rdp-saved-password",
                                    tr("dialog.savedPassword"),
                                    ConnectionEditorSelect::SavedPassword,
                                ))
                            },
                        )
                }),
        )
}
