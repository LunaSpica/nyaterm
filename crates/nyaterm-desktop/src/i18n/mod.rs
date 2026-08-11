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
    use std::collections::HashSet;

    use regex::Regex;

    use super::{EN_CATALOG, EN_JSON, ZH_CN_CATALOG, ZH_CN_JSON, catalog, text};

    #[test]
    fn resolves_tauri_locale_keys_and_normalizes_chinese_ids() {
        assert_eq!(text("en", "menu.file"), "File");
        assert_eq!(text("zh-CN", "menu.file"), "文件");
        assert_eq!(text("zh_CN", "common.cancel"), "取消");
        assert_eq!(text("zh-Hans", "settings.title"), "设置");
        assert_eq!(text("en", "common.copyToClipboard"), "Copy");
        assert_eq!(text("zh-CN", "common.copyToClipboard"), "复制");
        assert_eq!(text("en", "common.retry"), "Retry");
        assert_eq!(text("zh-CN", "common.retry"), "重试");
    }

    #[test]
    fn falls_back_to_english_then_the_key() {
        assert_eq!(text("fr", "menu.help"), "Help");
        assert_eq!(
            text("zh-CN", "missing.translation.key"),
            "missing.translation.key"
        );
    }

    #[test]
    fn english_and_chinese_catalogs_have_identical_keys() {
        let english = catalog(&EN_CATALOG, EN_JSON).keys().collect::<HashSet<_>>();
        let chinese = catalog(&ZH_CN_CATALOG, ZH_CN_JSON)
            .keys()
            .collect::<HashSet<_>>();
        assert_eq!(english, chinese);
    }

    #[test]
    fn connection_editor_static_translation_keys_exist_in_both_catalogs() {
        let sources = [
            include_str!("../features/pages/connections/editor/mod.rs"),
            include_str!("../features/pages/connections/editor/connection/mod.rs"),
            include_str!("../features/pages/connections/editor/connection/local.rs"),
            include_str!("../features/pages/connections/editor/connection/rdp.rs"),
            include_str!("../features/pages/connections/editor/connection/recording.rs"),
            include_str!("../features/pages/connections/editor/connection/serial.rs"),
            include_str!("../features/pages/connections/editor/connection/ssh.rs"),
            include_str!("../features/pages/connections/editor/connection/telnet.rs"),
            include_str!("../features/connections/connection_runtime/editor.rs"),
            include_str!("../features/connections/connection_runtime/window.rs"),
            include_str!("../features/connections/state/editor_logic.rs"),
        ];
        let key_pattern = Regex::new(r#"(?:\btr|self\.tr|I18n)\(\s*\"([^\"]+)\""#)
            .expect("translation-key regex");
        let english = catalog(&EN_CATALOG, EN_JSON);
        let chinese = catalog(&ZH_CN_CATALOG, ZH_CN_JSON);
        let mut missing = Vec::new();
        for source in sources {
            for captures in key_pattern.captures_iter(source) {
                let key = captures.get(1).expect("key capture").as_str();
                if !english.contains_key(key) || !chinese.contains_key(key) {
                    missing.push(key.to_string());
                }
            }
        }
        missing.sort();
        missing.dedup();
        assert!(
            missing.is_empty(),
            "missing connection editor keys: {missing:?}"
        );
    }

    #[test]
    fn connection_editor_algorithm_and_telnet_labels_are_localized() {
        assert_eq!(text("en", "dialog.sshAlgorithms"), "SSH algorithms");
        assert_eq!(text("zh-CN", "dialog.sshAlgorithms"), "SSH 算法");
        assert_eq!(text("en", "dialog.telnetAutoLogin"), "Auto Login");
        assert_eq!(text("zh-CN", "dialog.telnetAutoLogin"), "自动登录");
        assert_eq!(
            text("en", "dialog.algorithmUnsupportedError")
                .replace("{{algorithm}}", "future-kex")
                .replace("{{category}}", "key exchanges"),
            "future-kex is not supported in key exchanges."
        );
    }
}
