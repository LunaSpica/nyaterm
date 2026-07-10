use super::*;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::io::Read;

#[derive(Debug, Default)]
struct ImportSummary {
    imported_commands: usize,
    imported_categories: usize,
    updated_commands: usize,
    total_commands: usize,
    total_categories: usize,
}

#[derive(Debug, Default)]
struct ImportConfig {
    commands: Vec<ImportCommand>,
    categories: Vec<ImportCategory>,
}

#[derive(Debug)]
struct ImportCategory {
    id: Option<String>,
    name: String,
}

#[derive(Debug)]
struct ImportCommand {
    id: Option<String>,
    label: String,
    command: String,
    category_id: Option<String>,
    category: Option<String>,
    description: Option<String>,
    color_tag: Option<String>,
    icon_tag: Option<String>,
    pinned: Option<bool>,
    execution_mode: Option<String>,
    source: Option<String>,
    risk_level: Option<RiskLevel>,
}

impl QuickCommandImportPathPromptKind {
    fn prompt_label(self) -> &'static str {
        match self {
            Self::NyatermJson => "Import NyaTerm quick commands JSON",
            Self::WindTermQuickbar => "Import WindTerm quickbar.config",
            Self::XshellXts => "Import Xshell quick buttons .xts",
        }
    }

    fn selecting_status(self) -> &'static str {
        match self {
            Self::NyatermJson => "selecting quick command JSON import file",
            Self::WindTermQuickbar => "selecting WindTerm quickbar import file",
            Self::XshellXts => "selecting Xshell quick button import file",
        }
    }
}

impl NyaTermApp {
    pub(in crate::ui::view) fn open_quick_command_import_dialog(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.quick_command_import_path_prompt.is_some() {
            self.terminal_status = "quick command import picker is already open".to_string();
            cx.notify();
            return;
        }

        self.quick_command_import_dialog_open = true;
        self.terminal_status = "select a quick command import source".to_string();
        window.focus(&self.quick_command_import_focus);
        cx.notify();
    }

    pub(in crate::ui::view) fn close_quick_command_import_dialog(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        self.quick_command_import_dialog_open = false;
        cx.notify();
    }

    pub(in crate::ui::view) fn select_quick_command_import_source(
        &mut self,
        kind: QuickCommandImportPathPromptKind,
        cx: &mut Context<Self>,
    ) {
        self.quick_command_import_dialog_open = false;
        self.prompt_quick_command_import(kind, cx);
    }

    fn prompt_quick_command_import(
        &mut self,
        kind: QuickCommandImportPathPromptKind,
        cx: &mut Context<Self>,
    ) {
        if self.quick_command_import_path_prompt.is_some() {
            self.terminal_status = "quick command import picker is already open".to_string();
            cx.notify();
            return;
        }

        let options = PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some(SharedString::from(kind.prompt_label())),
        };
        let receiver = cx.prompt_for_paths(options);
        let config_dir = self.runtime.config_dir().to_path_buf();
        let portable_key_path = self.runtime.portable_key_path().map(ToOwned::to_owned);
        self.quick_command_import_path_prompt = Some(kind);
        self.terminal_status = kind.selecting_status().to_string();

        cx.spawn(async move |this, cx| {
            let result = match receiver.await {
                Ok(Ok(Some(paths))) => match paths.into_iter().next() {
                    Some(path) => match import_quick_commands_from_path(
                        &config_dir,
                        portable_key_path.clone(),
                        kind,
                        &path,
                    ) {
                        Ok(summary) => QuickCommandImportPathPromptResult::Imported {
                            imported_commands: summary.imported_commands,
                            imported_categories: summary.imported_categories,
                            updated_commands: summary.updated_commands,
                            total_commands: summary.total_commands,
                            total_categories: summary.total_categories,
                        },
                        Err(error) => QuickCommandImportPathPromptResult::Failed(error),
                    },
                    None => QuickCommandImportPathPromptResult::Cancelled,
                },
                Ok(Ok(None)) => QuickCommandImportPathPromptResult::Cancelled,
                Ok(Err(error)) => QuickCommandImportPathPromptResult::Failed(error.to_string()),
                Err(_) => QuickCommandImportPathPromptResult::Closed,
            };
            let _ = this.update(cx, |this, cx| {
                this.apply_quick_command_import_result(result);
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    fn apply_quick_command_import_result(&mut self, result: QuickCommandImportPathPromptResult) {
        self.quick_command_import_path_prompt = None;
        match result {
            QuickCommandImportPathPromptResult::Imported {
                imported_commands,
                imported_categories,
                updated_commands,
                total_commands,
                total_categories,
            } => {
                self.refresh_quick_commands();
                self.terminal_status = format!(
                    "imported {imported_commands} quick command(s), updated {updated_commands}, categories +{imported_categories}, total {total_commands}/{total_categories}"
                );
                self.store_status.message = self.terminal_status.clone();
                self.store_status.ready = true;
            }
            QuickCommandImportPathPromptResult::Cancelled => {
                self.terminal_status = "quick command import cancelled".to_string();
            }
            QuickCommandImportPathPromptResult::Failed(error) => {
                self.terminal_status = format!("quick command import failed: {error}");
                self.store_status.message = self.terminal_status.clone();
                self.store_status.ready = false;
            }
            QuickCommandImportPathPromptResult::Closed => {
                self.terminal_status =
                    "quick command import picker closed before returning".to_string();
            }
        }
    }
}

fn import_quick_commands_from_path(
    config_dir: &std::path::Path,
    portable_key_path: Option<PathBuf>,
    kind: QuickCommandImportPathPromptKind,
    path: &std::path::Path,
) -> Result<ImportSummary, String> {
    let store = ConnectionStore::open_with_portable_key_path(config_dir, portable_key_path)
        .map_err(|error| error.to_string())?;
    let mut config = store
        .load_quick_commands()
        .map_err(|error| error.to_string())?;
    let import_config = match kind {
        QuickCommandImportPathPromptKind::NyatermJson => {
            let raw = std::fs::read_to_string(path).map_err(|error| error.to_string())?;
            parse_nyaterm_import(&raw)?
        }
        QuickCommandImportPathPromptKind::WindTermQuickbar => {
            let raw = std::fs::read_to_string(path).map_err(|error| error.to_string())?;
            parse_windterm_quickbar(&raw)?
        }
        QuickCommandImportPathPromptKind::XshellXts => parse_xshell_xts_quick_buttons(path)?,
    };
    if import_config.commands.is_empty() {
        return Err("No valid quick commands found in import file".to_string());
    }
    let mut summary = merge_import(&mut config, import_config)?;
    store
        .save_quick_commands(config.clone())
        .map_err(|error| error.to_string())?;
    summary.total_commands = config.commands.len();
    summary.total_categories = config.categories.len();
    Ok(summary)
}

fn parse_nyaterm_import(raw: &str) -> Result<ImportConfig, String> {
    let value = serde_json::from_str::<Value>(raw).map_err(|error| error.to_string())?;
    parse_import_value(value)
}

fn parse_windterm_quickbar(raw: &str) -> Result<ImportConfig, String> {
    let entries = serde_json::from_str::<Vec<Value>>(raw)
        .map_err(|error| format!("Invalid WindTerm quickbar JSON: {error}"))?;
    let mut commands = Vec::new();

    for entry in entries {
        let label = entry
            .get("quick.label")
            .and_then(Value::as_str)
            .map(str::trim)
            .unwrap_or("");
        let command = entry
            .get("quick.text")
            .and_then(Value::as_str)
            .map(str::trim)
            .unwrap_or("");
        if label.is_empty() || command.is_empty() {
            continue;
        }

        let id = entry
            .get("quick.uuid")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string);
        let category = entry
            .get("quick.group")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string);
        let icon_tag = entry
            .get("quick.icon")
            .and_then(Value::as_str)
            .and_then(map_windterm_icon);
        let execution_mode = match entry
            .get("quick.type")
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim()
        {
            value if value.eq_ignore_ascii_case("Send Text") => "append",
            _ => "execute",
        };

        commands.push(ImportCommand {
            id,
            label: label.to_string(),
            command: command.to_string(),
            category_id: None,
            category,
            description: None,
            color_tag: None,
            icon_tag,
            pinned: Some(false),
            execution_mode: Some(execution_mode.to_string()),
            source: Some("manual".to_string()),
            risk_level: None,
        });
    }

    Ok(ImportConfig {
        commands,
        categories: Vec::new(),
    })
}

fn parse_xshell_xts_quick_buttons(path: &std::path::Path) -> Result<ImportConfig, String> {
    let file = std::fs::File::open(path)
        .map_err(|error| format!("Cannot open Xshell XTS file: {error}"))?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|error| format!("Invalid ZIP/XTS file: {error}"))?;

    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|error| format!("ZIP entry error: {error}"))?;
        let entry_path = decode_text(entry.name_raw()).replace('\\', "/");
        let normalized_path = entry_path.trim_start_matches("./").trim_start_matches('/');
        let lookup_path = normalized_path.to_ascii_lowercase();
        if lookup_path != "xsl/quickbutton files/commands.qbl"
            && !lookup_path.ends_with("/xsl/quickbutton files/commands.qbl")
        {
            continue;
        }

        let mut raw = Vec::new();
        entry
            .read_to_end(&mut raw)
            .map_err(|error| format!("Failed to read {entry_path}: {error}"))?;
        return Ok(parse_xshell_quick_buttons_content(&decode_text(&raw)));
    }

    Err("Xshell quick button file not found: xsl/QuickButton Files/commands.qbl".to_string())
}

fn parse_xshell_quick_buttons_content(raw: &str) -> ImportConfig {
    let sections = parse_ini_sections(raw);
    let Some(quick_button) = sections.get("QuickButton") else {
        return ImportConfig::default();
    };

    let mut buttons: BTreeMap<usize, HashMap<String, String>> = BTreeMap::new();
    for (key, value) in quick_button {
        let Some(rest) = key.strip_prefix("Button_") else {
            continue;
        };
        let Some((index, field)) = rest.split_once('_') else {
            continue;
        };
        let Ok(index) = index.parse::<usize>() else {
            continue;
        };

        buttons
            .entry(index)
            .or_default()
            .insert(field.to_string(), value.clone());
    }

    let commands = buttons
        .into_iter()
        .filter_map(|(_, fields)| {
            let button_type = fields.get("Type").map(String::as_str).unwrap_or("");
            if button_type.trim() != "1" {
                return None;
            }

            let label = fields.get("Name").map(String::as_str).unwrap_or("").trim();
            let command = fields
                .get("Action")
                .map(String::as_str)
                .unwrap_or("")
                .trim();
            if label.is_empty() || command.is_empty() {
                return None;
            }

            Some(ImportCommand {
                id: None,
                label: label.to_string(),
                command: command.to_string(),
                category_id: None,
                category: None,
                description: trim_optional(fields.get("Desc").cloned()),
                color_tag: None,
                icon_tag: None,
                pinned: Some(false),
                execution_mode: Some("append".to_string()),
                source: Some("manual".to_string()),
                risk_level: None,
            })
        })
        .collect();

    ImportConfig {
        commands,
        categories: Vec::new(),
    }
}

fn parse_ini_sections(raw: &str) -> HashMap<String, HashMap<String, String>> {
    let mut sections: HashMap<String, HashMap<String, String>> = HashMap::new();
    let mut current_section = String::new();

    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
            continue;
        }

        if line.starts_with('[') && line.ends_with(']') {
            current_section = line[1..line.len() - 1].to_string();
            sections.entry(current_section.clone()).or_default();
            continue;
        }

        if let Some((key, value)) = line.split_once('=') {
            sections
                .entry(current_section.clone())
                .or_default()
                .insert(key.trim().to_string(), value.trim().to_string());
        }
    }

    sections
}

fn decode_text(raw: &[u8]) -> String {
    if let Some((encoding, bom_len)) = encoding_rs::Encoding::for_bom(raw) {
        let (decoded, _, _) = encoding.decode(&raw[bom_len..]);
        return decoded.into_owned();
    }

    match std::str::from_utf8(raw) {
        Ok(value) => value.to_string(),
        Err(_) => {
            let (decoded, _, _) = encoding_rs::GBK.decode(raw);
            decoded.into_owned()
        }
    }
}

fn parse_import_value(value: Value) -> Result<ImportConfig, String> {
    match value {
        Value::Array(commands) => Ok(ImportConfig {
            commands: commands
                .into_iter()
                .map(parse_import_command)
                .collect::<Result<Vec<_>, _>>()?,
            categories: Vec::new(),
        }),
        Value::Object(mut object) => {
            let categories = match object.remove("categories") {
                Some(Value::Array(categories)) => categories
                    .into_iter()
                    .map(parse_import_category)
                    .collect::<Result<Vec<_>, _>>()?,
                Some(Value::Null) | None => Vec::new(),
                Some(_) => return Err("categories must be an array".to_string()),
            };
            let commands = match object.remove("commands") {
                Some(Value::Array(commands)) => commands
                    .into_iter()
                    .map(parse_import_command)
                    .collect::<Result<Vec<_>, _>>()?,
                Some(Value::Null) | None => Vec::new(),
                Some(_) => return Err("commands must be an array".to_string()),
            };
            Ok(ImportConfig {
                commands,
                categories,
            })
        }
        _ => Err("quick command import must be an object or command array".to_string()),
    }
}

fn parse_import_category(value: Value) -> Result<ImportCategory, String> {
    let Value::Object(object) = value else {
        return Err("category must be an object".to_string());
    };
    let name = required_string_field(&object, "name")?;
    Ok(ImportCategory {
        id: optional_string_field(&object, "id")?,
        name,
    })
}

fn parse_import_command(value: Value) -> Result<ImportCommand, String> {
    let Value::Object(object) = value else {
        return Err("command must be an object".to_string());
    };
    Ok(ImportCommand {
        id: optional_string_field(&object, "id")?,
        label: required_string_field(&object, "label")?,
        command: required_string_field(&object, "command")?,
        category_id: optional_string_field(&object, "category_id")?,
        category: optional_string_field(&object, "category")?,
        description: optional_string_field(&object, "description")?,
        color_tag: optional_string_field(&object, "color_tag")?,
        icon_tag: optional_string_field(&object, "icon_tag")?,
        pinned: optional_bool_field(&object, "pinned")?,
        execution_mode: optional_string_field(&object, "execution_mode")?,
        source: optional_string_field(&object, "source")?,
        risk_level: optional_risk_field(&object, "risk_level")?,
    })
}

fn required_string_field(
    object: &serde_json::Map<String, Value>,
    field: &str,
) -> Result<String, String> {
    let Some(value) = object.get(field) else {
        return Err(format!("{field} cannot be empty"));
    };
    match value {
        Value::String(value) => Ok(value.clone()),
        Value::Null => Err(format!("{field} cannot be empty")),
        _ => Err(format!("{field} must be a string")),
    }
}

fn optional_string_field(
    object: &serde_json::Map<String, Value>,
    field: &str,
) -> Result<Option<String>, String> {
    match object.get(field) {
        Some(Value::String(value)) => Ok(Some(value.clone())),
        Some(Value::Null) | None => Ok(None),
        Some(_) => Err(format!("{field} must be a string")),
    }
}

fn optional_bool_field(
    object: &serde_json::Map<String, Value>,
    field: &str,
) -> Result<Option<bool>, String> {
    match object.get(field) {
        Some(Value::Bool(value)) => Ok(Some(*value)),
        Some(Value::Null) | None => Ok(None),
        Some(_) => Err(format!("{field} must be a boolean")),
    }
}

fn optional_risk_field(
    object: &serde_json::Map<String, Value>,
    field: &str,
) -> Result<Option<RiskLevel>, String> {
    let Some(value) = optional_string_field(object, field)? else {
        return Ok(None);
    };
    match value.trim() {
        "low" => Ok(Some(RiskLevel::Low)),
        "medium" => Ok(Some(RiskLevel::Medium)),
        "high" => Ok(Some(RiskLevel::High)),
        "critical" => Ok(Some(RiskLevel::Critical)),
        _ => Err(format!(
            "{field} must be one of: low, medium, high, critical"
        )),
    }
}

fn merge_import(
    config: &mut QuickCommandsConfig,
    import_config: ImportConfig,
) -> Result<ImportSummary, String> {
    let mut summary = ImportSummary::default();
    let mut category_names = BTreeMap::new();
    for category in &config.categories {
        category_names.insert(category.name.clone(), category.id.clone());
    }

    for category in import_config.categories {
        let name = require_text(&category.name, "category.name")?;
        let id_input = category.id.unwrap_or_else(|| slugify(&name));
        let id = normalize_id(&id_input, "category.id")?;
        if upsert_category(
            config,
            QuickCommandCategory {
                id: id.clone(),
                name: name.clone(),
            },
        ) {
            summary.imported_categories += 1;
        }
        category_names.insert(name, id);
    }

    let mut seen_ids = BTreeSet::new();
    let now = current_time_ms();
    for command in import_config.commands {
        let label = require_text(&command.label, "command.label")?;
        let command_text = require_text(&command.command, "command.command")?;
        let id_input = command.id.unwrap_or_else(|| format!("cmd-{}", uuid()));
        let id = normalize_id(&id_input, "command.id")?;
        if !seen_ids.insert(id.clone()) {
            return Err(format!("Duplicate command id in import file: {id}"));
        }

        let category_id = match (command.category_id, command.category) {
            (Some(category_id), _) => {
                let category_id = normalize_id(&category_id, "command.category_id")?;
                ensure_category(
                    config,
                    &mut category_names,
                    &category_id,
                    &category_id,
                    &mut summary,
                );
                Some(category_id)
            }
            (None, Some(category_name)) => {
                let category_name = require_text(&category_name, "command.category")?;
                let category_id = category_names
                    .get(&category_name)
                    .cloned()
                    .unwrap_or_else(|| slugify(&category_name));
                ensure_category(
                    config,
                    &mut category_names,
                    &category_id,
                    &category_name,
                    &mut summary,
                );
                Some(category_id)
            }
            (None, None) => None,
        };

        let execution_mode = command
            .execution_mode
            .as_deref()
            .unwrap_or("execute")
            .trim()
            .to_string();
        validate_one_of(
            &execution_mode,
            &["execute", "append"],
            "command.execution_mode",
        )?;
        if let Some(source) = command.source.as_deref().map(str::trim) {
            validate_one_of(source, &["manual", "ai"], "command.source")?;
        }

        let imported = QuickCommand {
            id,
            label,
            command: command_text,
            category_id,
            description: trim_optional(command.description),
            color_tag: trim_optional(command.color_tag),
            icon_tag: trim_optional(command.icon_tag),
            pinned: command.pinned,
            execution_mode: Some(execution_mode),
            source: trim_optional(command.source),
            risk_level: command.risk_level,
            updated_at: Some(now),
            created_at: Some(now),
            use_count: None,
        };
        if upsert_command(config, imported) {
            summary.imported_commands += 1;
        } else {
            summary.updated_commands += 1;
        }
    }
    Ok(summary)
}

fn ensure_category(
    config: &mut QuickCommandsConfig,
    category_names: &mut BTreeMap<String, String>,
    id: &str,
    name: &str,
    summary: &mut ImportSummary,
) {
    if config.categories.iter().any(|category| category.id == id) {
        category_names.insert(name.to_string(), id.to_string());
        return;
    }
    config.categories.push(QuickCommandCategory {
        id: id.to_string(),
        name: name.to_string(),
    });
    category_names.insert(name.to_string(), id.to_string());
    summary.imported_categories += 1;
}

fn upsert_category(config: &mut QuickCommandsConfig, category: QuickCommandCategory) -> bool {
    if let Some(existing) = config
        .categories
        .iter_mut()
        .find(|item| item.id == category.id)
    {
        *existing = category;
        false
    } else {
        config.categories.push(category);
        true
    }
}

fn upsert_command(config: &mut QuickCommandsConfig, command: QuickCommand) -> bool {
    if let Some(existing) = config
        .commands
        .iter_mut()
        .find(|item| item.id == command.id)
    {
        let created_at = existing.created_at;
        let use_count = existing.use_count;
        *existing = command;
        existing.created_at = created_at.or(existing.created_at);
        existing.use_count = use_count.or(existing.use_count);
        false
    } else {
        config.commands.push(command);
        true
    }
}

fn require_text(value: &str, field: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(format!("{field} cannot be empty"));
    }
    Ok(trimmed.to_string())
}

fn normalize_id(value: &str, field: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(format!("{field} cannot be empty"));
    }
    Ok(trimmed.to_string())
}

fn trim_optional(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn validate_one_of(value: &str, allowed: &[&str], field: &str) -> Result<(), String> {
    if allowed.contains(&value) {
        Ok(())
    } else {
        Err(format!("{field} must be one of: {}", allowed.join(", ")))
    }
}

fn slugify(value: &str) -> String {
    let mut output = String::new();
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            output.push(ch.to_ascii_lowercase());
        } else if ch == '-' || ch == '_' {
            output.push(ch);
        } else if ch.is_whitespace() && !output.ends_with('-') {
            output.push('-');
        }
    }
    let output = output.trim_matches('-').to_string();
    if output.is_empty() {
        format!("category-{}", uuid())
    } else {
        output
    }
}

fn map_windterm_icon(value: &str) -> Option<String> {
    let normalized = value.to_ascii_lowercase();
    let mappings = [
        ("kubernetes", "k8s"),
        ("k8s", "k8s"),
        ("docker", "docker"),
        ("linux", "linux"),
        ("ubuntu", "ubuntu"),
        ("debian", "debian"),
        ("centos", "centos"),
        ("fedora", "fedora"),
        ("apple", "apple"),
        ("github", "github"),
        ("gitlab", "gitlab"),
        ("nginx", "nginx"),
        ("redis", "redis"),
        ("postgres", "postgres"),
        ("mysql", "mysql"),
        ("mongo", "mongodb"),
        ("python", "python"),
        ("javascript", "js"),
        ("typescript", "ts"),
        ("rust", "rust"),
        ("node", "node"),
        ("php", "php"),
        ("aws", "aws"),
        ("gcp", "gcp"),
    ];

    mappings
        .iter()
        .find_map(|(needle, icon)| normalized.contains(needle).then(|| (*icon).to_string()))
}

fn current_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn imports_config_and_array_shapes() {
        let config = parse_nyaterm_import(
            r#"{"categories":[{"id":"ops","name":"Ops"}],"commands":[{"id":"c1","label":"List","command":"ls","category_id":"ops"}]}"#,
        )
        .expect("config import parses");
        assert_eq!(config.categories.len(), 1);
        assert_eq!(config.commands.len(), 1);

        let config = parse_nyaterm_import(r#"[{"label":"Pwd","command":"pwd"}]"#)
            .expect("array import parses");
        assert_eq!(config.categories.len(), 0);
        assert_eq!(config.commands.len(), 1);
    }

    #[test]
    fn merge_import_updates_commands_and_creates_named_categories() {
        let mut config = QuickCommandsConfig::default();
        let import_config = parse_nyaterm_import(
            r#"{"commands":[{"id":"c1","label":"List","command":"ls","category":"Ops"},{"id":"c1","label":"Dupe","command":"pwd"}]}"#,
        )
        .expect("json parses");
        let error = merge_import(&mut config, import_config).expect_err("duplicate ids fail");
        assert!(error.contains("Duplicate command id"));

        let import_config = parse_nyaterm_import(
            r#"{"commands":[{"id":"c1","label":"List","command":"ls","category":"Ops"},{"id":"c2","label":"Pwd","command":"pwd","execution_mode":"append"}]}"#,
        )
        .expect("json parses");
        let summary = merge_import(&mut config, import_config).expect("merge succeeds");
        assert_eq!(summary.imported_commands, 2);
        assert_eq!(summary.imported_categories, 1);
        assert_eq!(config.categories[0].name, "Ops");
        assert_eq!(config.commands[1].execution_mode.as_deref(), Some("append"));

        let import_config =
            parse_nyaterm_import(r#"[{"id":"c1","label":"List all","command":"ls -la"}]"#)
                .expect("json parses");
        let summary = merge_import(&mut config, import_config).expect("update succeeds");
        assert_eq!(summary.updated_commands, 1);
        assert_eq!(config.commands[0].label, "List all");
    }

    #[test]
    fn imports_windterm_quickbar_json() {
        let import_config = parse_windterm_quickbar(
            r#"[{
                "quick.group": "快速",
                "quick.icon": "session::docker-blue",
                "quick.label": "miniconda3 安装",
                "quick.text": "echo install",
                "quick.type": "Send Text",
                "quick.uuid": "70127d80-24b8-46eb-958d-f944c5e423dd"
            }]"#,
        )
        .expect("windterm quickbar parses");
        let mut config = QuickCommandsConfig::default();

        let summary = merge_import(&mut config, import_config).expect("merge succeeds");

        assert_eq!(summary.imported_commands, 1);
        assert_eq!(summary.imported_categories, 1);
        assert_eq!(config.categories[0].name, "快速");
        assert_eq!(
            config.commands[0].id,
            "70127d80-24b8-46eb-958d-f944c5e423dd"
        );
        assert_eq!(config.commands[0].label, "miniconda3 安装");
        assert_eq!(config.commands[0].command, "echo install");
        assert_eq!(config.commands[0].execution_mode.as_deref(), Some("append"));
        assert_eq!(config.commands[0].source.as_deref(), Some("manual"));
        assert_eq!(config.commands[0].icon_tag.as_deref(), Some("docker"));
        assert_eq!(config.commands[0].pinned, Some(false));
    }

    #[test]
    fn windterm_defaults_execute_and_skips_empty_entries() {
        let import_config = parse_windterm_quickbar(
            r#"[
                {"quick.label":"","quick.text":"echo no"},
                {"quick.label":"No text"},
                {"quick.label":"Version","quick.text":"rustc --version","quick.type":"Run Command","quick.icon":"Typescript"}
            ]"#,
        )
        .expect("windterm quickbar parses");

        assert_eq!(import_config.commands.len(), 1);
        assert_eq!(import_config.commands[0].label, "Version");
        assert_eq!(
            import_config.commands[0].execution_mode.as_deref(),
            Some("execute")
        );
        assert_eq!(import_config.commands[0].icon_tag.as_deref(), Some("ts"));
    }

    #[test]
    fn imports_xshell_quick_buttons_type_one_only() {
        let import_config = parse_xshell_quick_buttons_content(
            r#"[Info]
Version=8.2
Count=3
Expanded=1
[QuickButton]
Button_0_Name=测试
Button_1_Name=Pwd
Button_2_Name=Ignored
Button_0_Type=1
Button_1_Type=1
Button_2_Type=2
Button_0_Action=ls -la
Button_1_Action=pwd
Button_2_Action=whoami
Button_0_Desc=List files
"#,
        );
        let mut config = QuickCommandsConfig::default();

        let summary = merge_import(&mut config, import_config).expect("merge succeeds");

        assert_eq!(summary.imported_commands, 2);
        assert_eq!(config.commands[0].label, "测试");
        assert_eq!(config.commands[0].command, "ls -la");
        assert_eq!(
            config.commands[0].description.as_deref(),
            Some("List files")
        );
        assert_eq!(config.commands[0].execution_mode.as_deref(), Some("append"));
        assert_eq!(config.commands[0].source.as_deref(), Some("manual"));
        assert_eq!(config.commands[1].label, "Pwd");
        assert_eq!(config.commands[1].command, "pwd");
    }

    #[test]
    fn decodes_xshell_text_with_utf_bom_and_gbk_fallback() {
        let utf16_le = [0xff, 0xfe, b'T', 0, b'E', 0, b'S', 0, b'T', 0];
        assert_eq!(decode_text(&utf16_le), "TEST");

        let gbk = [0xb2, 0xe2, 0xca, 0xd4];
        assert_eq!(decode_text(&gbk), "测试");
    }
}
