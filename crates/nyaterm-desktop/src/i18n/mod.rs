use std::collections::HashMap;
use std::sync::OnceLock;

use serde_json::Value;

const EN_JSON: &str = include_str!("locales/en.json");
const ZH_CN_JSON: &str = include_str!("locales/zh-CN.json");

static EN_CATALOG: OnceLock<HashMap<String, String>> = OnceLock::new();
static ZH_CN_CATALOG: OnceLock<HashMap<String, String>> = OnceLock::new();

pub(crate) fn text(language: &str, key: &'static str) -> &'static str {
    let selected = if is_simplified_chinese(language) {
        catalog(&ZH_CN_CATALOG, ZH_CN_JSON)
    } else {
        catalog(&EN_CATALOG, EN_JSON)
    };

    selected
        .get(key)
        .or_else(|| catalog(&EN_CATALOG, EN_JSON).get(key))
        .map(String::as_str)
        .unwrap_or(key)
}

fn is_simplified_chinese(language: &str) -> bool {
    let normalized = language.trim().replace('_', "-").to_ascii_lowercase();
    normalized == "zh" || normalized == "zh-cn" || normalized.starts_with("zh-hans")
}

fn catalog(
    slot: &'static OnceLock<HashMap<String, String>>,
    json: &'static str,
) -> &'static HashMap<String, String> {
    slot.get_or_init(|| {
        let value: Value = serde_json::from_str(json).expect("embedded locale JSON must be valid");
        let mut output = HashMap::new();
        flatten_json(None, &value, &mut output);
        output
    })
}

fn flatten_json(prefix: Option<&str>, value: &Value, output: &mut HashMap<String, String>) {
    let Value::Object(entries) = value else {
        return;
    };

    for (name, value) in entries {
        let key = prefix.map_or_else(|| name.clone(), |prefix| format!("{prefix}.{name}"));
        match value {
            Value::String(text) => {
                output.insert(key, text.clone());
            }
            Value::Object(_) => flatten_json(Some(&key), value, output),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::text;

    #[test]
    fn resolves_tauri_locale_keys_and_normalizes_chinese_ids() {
        assert_eq!(text("en", "menu.file"), "File");
        assert_eq!(text("zh-CN", "menu.file"), "文件");
        assert_eq!(text("zh_CN", "common.cancel"), "取消");
        assert_eq!(text("zh-Hans", "settings.title"), "设置");
    }

    #[test]
    fn falls_back_to_english_then_the_key() {
        assert_eq!(text("fr", "menu.help"), "Help");
        assert_eq!(
            text("zh-CN", "missing.translation.key"),
            "missing.translation.key"
        );
    }
}
