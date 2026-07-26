use super::*;

pub(super) const SEARCH_ENGINE_ICON_IDS: &[&str] = &[
    "google",
    "bing",
    "duckduckgo",
    "github",
    "gitlab",
    "baidu",
    "yahoo",
    "youtube",
    "bilibili",
    "zhihu",
    "openai",
    "claude",
    "gemini",
    "default",
];

pub(super) fn search_engine_icon_label(icon: Option<&str>) -> String {
    match icon.unwrap_or("default") {
        "google" => "G".into(),
        "bing" => "B".into(),
        "duckduckgo" => "D".into(),
        "github" => "GH".into(),
        "gitlab" => "GL".into(),
        "baidu" => "Bd".into(),
        "yahoo" => "Y!".into(),
        "youtube" => "YT".into(),
        "bilibili" => "Bi".into(),
        "zhihu" => "Zh".into(),
        "openai" => "AI".into(),
        "claude" => "Cl".into(),
        "gemini" => "Ge".into(),
        _ => "?".into(),
    }
}

pub(super) fn search_engine_icon_color(icon: Option<&str>) -> u32 {
    match icon.unwrap_or("default") {
        "google" => 0x4285f4,
        "bing" => 0x008373,
        "duckduckgo" => 0xde5833,
        "github" => 0x8b949e,
        "gitlab" => 0xfc6d26,
        "baidu" => 0x2932e1,
        "yahoo" => 0x410093,
        "youtube" => 0xff0000,
        "bilibili" => 0x00a1d6,
        "zhihu" => 0x0084ff,
        "openai" => 0x10a37f,
        "claude" => 0xd97757,
        "gemini" => 0x4285f4,
        _ => 0x8b949e,
    }
}

pub(super) fn parse_keyword_swatch(value: &str) -> Option<u32> {
    let hex = value.trim().strip_prefix('#').unwrap_or(value.trim());
    if hex.len() != 6 {
        return None;
    }
    u32::from_str_radix(hex, 16).ok()
}
