use super::*;

pub(super) fn ai_setting_hint(
    palette: crate::theme::ThemePalette,
    title: &'static str,
    detail: &'static str,
) -> impl IntoElement {
    div()
        .rounded_sm()
        .border_1()
        .border_color(rgb(palette.border))
        .bg(rgb(palette.input))
        .p_3()
        .child(
            div()
                .text_xs()
                .font_weight(FontWeight(800.))
                .text_color(rgb(palette.text))
                .child(title),
        )
        .child(
            div()
                .mt_1()
                .text_size(px(10.))
                .text_color(rgb(palette.text_muted))
                .line_height(px(14.))
                .child(detail),
        )
}

pub(super) fn ai_boolean_state(
    palette: crate::theme::ThemePalette,
    label: &'static str,
    enabled: bool,
) -> impl IntoElement {
    div()
        .rounded_md()
        .border_1()
        .border_color(rgb(palette.border))
        .bg(rgb(palette.input))
        .p_3()
        .child(
            div()
                .text_xs()
                .text_color(rgb(palette.text_muted))
                .child(label),
        )
        .child(
            div()
                .mt_1()
                .text_sm()
                .font_weight(FontWeight(700.))
                .text_color(if enabled {
                    rgb(0x86efac)
                } else {
                    rgb(palette.text_muted)
                })
                .child(if enabled { "enabled" } else { "disabled" }),
        )
}

pub(super) fn ai_provider_kind_label(kind: &AiProviderKind) -> &'static str {
    match kind {
        AiProviderKind::Openai => "OpenAI",
        AiProviderKind::Anthropic => "Anthropic",
        AiProviderKind::Gemini => "Gemini",
        AiProviderKind::Deepseek => "DeepSeek",
        AiProviderKind::Groq => "Groq",
        AiProviderKind::Ollama => "Ollama",
        AiProviderKind::Xai => "xAI",
        AiProviderKind::Cohere => "Cohere",
        AiProviderKind::Mimo => "Mimo",
        AiProviderKind::Zai => "Z.ai",
        AiProviderKind::OpenaiCompatible => "OpenAI Compatible",
    }
}

pub(super) fn ai_model_source_label(source: &AiModelSource) -> &'static str {
    match source {
        AiModelSource::RustGenai => "discovered",
        AiModelSource::Manual => "manual",
    }
}
