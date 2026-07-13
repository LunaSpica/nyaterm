use super::*;

pub(super) fn clamp_menu_position(
    x: f32,
    y: f32,
    menu_w: f32,
    menu_h: f32,
    viewport_w: f32,
    viewport_h: f32,
) -> (f32, f32) {
    let margin = 8.0;
    let max_x = (viewport_w - menu_w - margin).max(margin);
    let max_y = (viewport_h - menu_h - margin).max(margin);
    (x.clamp(margin, max_x), y.clamp(margin, max_y))
}

pub(super) fn terminal_ctx_item(
    palette: crate::theme::ThemePalette,
    id: impl Into<String>,
    label: impl Into<String>,
    shortcut: Option<String>,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let label = label.into();
    let mut row = div()
        .id(SharedString::from(id.into()))
        .h(px(28.))
        .px_3()
        .flex()
        .items_center()
        .justify_between()
        .gap_3()
        .text_size(px(12.))
        .text_color(rgb(palette.text))
        .cursor_pointer()
        .hover(|this| this.bg(rgb(palette.hover)))
        .on_click(on_click)
        .child(div().child(label));
    if let Some(shortcut) = shortcut {
        row = row.child(
            div()
                .text_size(px(10.))
                .text_color(rgb(palette.text_dimmed))
                .child(shortcut),
        );
    }
    row
}

pub(super) fn terminal_ctx_separator(palette: crate::theme::ThemePalette) -> impl IntoElement {
    div().h(px(1.)).my_1().mx_2().bg(rgb(palette.border))
}

pub(super) fn open_external_url(url: &str) -> Result<(), String> {
    let url = url.trim();
    if url.is_empty() {
        return Err("empty url".to_string());
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(url)
            .spawn()
            .map(|_| ())
            .map_err(|error| format!("failed to open url: {error}"))
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", ""])
            .arg(url)
            .spawn()
            .map(|_| ())
            .map_err(|error| format!("failed to open url: {error}"))
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        std::process::Command::new("xdg-open")
            .arg(url)
            .spawn()
            .map(|_| ())
            .map_err(|error| format!("failed to open url: {error}"))
    }
}

pub(super) fn search_engine_url(template: &str, query: &str) -> String {
    let encoded = urlencoding_minimal(query);
    if template.contains("%s") {
        template.replace("%s", &encoded)
    } else {
        format!("{template}{encoded}")
    }
}

pub(super) fn urlencoding_minimal(input: &str) -> String {
    let mut out = String::with_capacity(input.len() * 3);
    for b in input.as_bytes() {
        match *b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*b as char);
            }
            b' ' => out.push_str("%20"),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

pub(super) fn available_translation_providers(
    settings: &nyaterm_core::TranslationSettings,
) -> Vec<(String, String)> {
    let mut providers = vec![
        ("google".to_string(), "Google".to_string()),
        ("microsoft".to_string(), "Microsoft".to_string()),
    ];
    if !settings.deepl_api_key.trim().is_empty() {
        providers.push(("deepl".to_string(), "DeepL".to_string()));
    }
    if !settings.baidu_app_id.trim().is_empty() && !settings.baidu_app_key.trim().is_empty() {
        providers.push(("baidu".to_string(), "Baidu".to_string()));
    }
    if !settings.ali_app_id.trim().is_empty() && !settings.ali_app_key.trim().is_empty() {
        providers.push(("ali".to_string(), "Aliyun".to_string()));
    }
    if !settings.youdao_app_id.trim().is_empty() && !settings.youdao_app_key.trim().is_empty() {
        providers.push(("youdao".to_string(), "Youdao".to_string()));
    }
    providers
}

pub(super) fn selection_as_openable_url(selected: &str) -> Option<String> {
    let trimmed = selected.trim();
    if trimmed.is_empty() || trimmed.chars().any(|ch| ch.is_whitespace()) {
        return None;
    }
    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with("https://") || lower.starts_with("http://") {
        return Some(trimmed.to_string());
    }
    // Bare domains like example.com/path — require a dotted host and no path spaces.
    if trimmed.contains('.')
        && !trimmed.contains("://")
        && trimmed.chars().all(|ch| {
            ch.is_ascii_alphanumeric()
                || matches!(
                    ch,
                    '.' | '/' | ':' | '-' | '_' | '?' | '=' | '&' | '%' | '#' | '+'
                )
        })
    {
        // Prefer not to open single-token words without a TLD-ish shape.
        let host = trimmed.split('/').next().unwrap_or(trimmed);
        if host.contains('.') && host.split('.').all(|part| !part.is_empty()) {
            return Some(format!("https://{trimmed}"));
        }
    }
    None
}

pub(super) fn search_engine_menu_icon_prefix(icon: Option<&str>) -> String {
    let label = match icon.unwrap_or("default") {
        "google" => "G",
        "bing" => "B",
        "duckduckgo" => "D",
        "github" => "GH",
        "gitlab" => "GL",
        "baidu" => "Bd",
        "yahoo" => "Y!",
        "youtube" => "YT",
        "bilibili" => "Bi",
        "zhihu" => "Zh",
        "openai" => "AI",
        "claude" => "Cl",
        "gemini" => "Ge",
        _ => "·",
    };
    format!("[{label}] ")
}
