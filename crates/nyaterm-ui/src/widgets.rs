use crate::button::{NyaButton, NyaButtonVariant, NyaIconButton};
use crate::theme::ThemePalette;
use gpui::{
    AnyElement, App, ClickEvent, FontWeight, Hsla, IntoElement, Pixels, RenderOnce, SharedString,
    Window, div, prelude::*, px, rgb,
};
use gpui_component::scroll::ScrollableElement as _;

fn platform_code_font_family() -> &'static str {
    if cfg!(target_os = "windows") {
        "Consolas"
    } else {
        "JetBrains Mono"
    }
}

#[derive(IntoElement)]
pub struct NyaScrollArea {
    id: SharedString,
    max_height: Option<Pixels>,
    children: Vec<AnyElement>,
}

impl NyaScrollArea {
    pub fn new(id: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            max_height: None,
            children: Vec::new(),
        }
    }

    pub fn max_h(mut self, height: Pixels) -> Self {
        self.max_height = Some(height);
        self
    }

    pub fn child(mut self, child: impl IntoElement) -> Self {
        self.children.push(child.into_any_element());
        self
    }
}

impl RenderOnce for NyaScrollArea {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let mut area = div()
            .id(self.id)
            .flex()
            .flex_col()
            .children(self.children)
            .overflow_y_scrollbar();
        if let Some(max_height) = self.max_height {
            area = area.max_h(max_height);
        }
        area
    }
}

pub fn status_pill(
    label: &'static str,
    fg: impl Into<Hsla>,
    bg: impl Into<Hsla>,
) -> impl IntoElement {
    div()
        .rounded_sm()
        .px_2()
        .py_1()
        .text_xs()
        .text_color(fg.into())
        .bg(bg.into())
        .child(label)
}

pub fn empty_panel(text: &'static str, palette: ThemePalette) -> impl IntoElement {
    div()
        .rounded_md()
        .border_1()
        .border_color(rgb(palette.border))
        .bg(rgb(palette.input))
        .p_4()
        .text_sm()
        .text_color(rgb(palette.text_muted))
        .child(text)
}

pub fn section_header(
    title: &'static str,
    detail: &'static str,
    palette: ThemePalette,
) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .child(div().text_2xl().font_weight(FontWeight(800.)).child(title))
        .child(
            div()
                .text_sm()
                .text_color(rgb(palette.text_muted))
                .child(detail),
        )
}

pub fn capability_line(
    palette: ThemePalette,
    label: &'static str,
    value: impl Into<SharedString>,
) -> impl IntoElement {
    div()
        .mt_2()
        .flex()
        .items_center()
        .justify_between()
        .text_sm()
        .child(div().text_color(rgb(palette.text)).child(label))
        .child(
            div()
                .text_xs()
                .text_color(rgb(palette.text_muted))
                .child(value.into()),
        )
}

pub fn session_info_row(
    palette: ThemePalette,
    label: &'static str,
    value: String,
) -> impl IntoElement {
    div()
        .rounded_sm()
        .border_1()
        .border_color(rgb(palette.border))
        .bg(rgb(palette.input))
        .px_3()
        .py_2()
        .flex()
        .items_start()
        .gap_3()
        .child(
            div()
                .w(px(104.))
                .flex_none()
                .text_xs()
                .font_weight(FontWeight(700.))
                .text_color(rgb(palette.text_muted))
                .child(label),
        )
        .child(
            div()
                .min_w_0()
                .flex_1()
                .font_family(platform_code_font_family())
                .text_xs()
                .text_color(rgb(palette.text))
                .child(value),
        )
}

pub fn small_button(
    _palette: ThemePalette,
    id: impl Into<String>,
    label: &'static str,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    NyaButton::new(id.into(), label)
        .variant(NyaButtonVariant::Secondary)
        .small()
        .compact()
        .on_click(on_click)
}

pub fn mode_button(
    id: impl Into<String>,
    label: &'static str,
    active: bool,
    _palette: ThemePalette,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    NyaButton::new(id.into(), label)
        .variant(NyaButtonVariant::Ghost)
        .selected(active)
        .small()
        .compact()
        .on_click(on_click)
}

pub fn svg_icon_button(
    id: impl Into<String>,
    icon_path: &'static str,
    icon_size: f32,
    _palette: ThemePalette,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    NyaIconButton::new(id.into(), icon_path)
        .icon_size(px(icon_size))
        .on_click(on_click)
}
