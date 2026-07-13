pub(super) fn is_builtin_ai_provider_id(id: &str) -> bool {
    matches!(
        id,
        "openai"
            | "anthropic"
            | "gemini"
            | "deepseek"
            | "ollama"
            | "xai"
            | "cohere"
            | "mimo"
            | "zai"
            | "groq"
    )
}

pub(super) fn seed_builtin_ai_models_for_provider(
    settings: &mut nyaterm_core::AiSettings,
    provider_kind: &nyaterm_core::AiProviderKind,
) {
    let names: &[&str] = match provider_kind {
        nyaterm_core::AiProviderKind::Openai => &[
            "gpt-4o-mini",
            "gpt-4o",
            "gpt-4.1",
            "gpt-4.1-mini",
            "o3-mini",
            "o4-mini",
        ],
        nyaterm_core::AiProviderKind::Anthropic => &[
            "claude-3-haiku-20240307",
            "claude-3-5-sonnet-20241022",
            "claude-sonnet-4-20250514",
        ],
        nyaterm_core::AiProviderKind::Gemini => &["gemini-2.0-flash", "gemini-1.5-pro"],
        nyaterm_core::AiProviderKind::Deepseek => &["deepseek-chat", "deepseek-reasoner"],
        nyaterm_core::AiProviderKind::Ollama => &["llama3", "llama3.1", "qwen2.5"],
        nyaterm_core::AiProviderKind::Xai => &["grok-3", "grok-2"],
        nyaterm_core::AiProviderKind::Cohere => &["command-a-03-2025", "command-r-plus"],
        nyaterm_core::AiProviderKind::Mimo => &["mimo-v2.5-pro"],
        nyaterm_core::AiProviderKind::Zai => &["glm-4", "glm-4-flash"],
        nyaterm_core::AiProviderKind::Groq => &["llama-3.3-70b-versatile"],
        nyaterm_core::AiProviderKind::OpenaiCompatible => &[],
    };
    let existing: std::collections::HashSet<String> = settings
        .models
        .iter()
        .map(|model| model.id.clone())
        .collect();
    for name in names {
        let model_id = nyaterm_core::ai_model_id_for_provider(provider_kind, name);
        if existing.contains(&model_id) {
            continue;
        }
        settings.models.push(nyaterm_core::AiModelConfigItem {
            id: model_id,
            name: (*name).to_string(),
            provider_kind: Some(provider_kind.clone()),
            credential_id: None,
            enabled: false,
            source: nyaterm_core::AiModelSource::RustGenai,
            last_seen_at: None,
        });
    }
}
