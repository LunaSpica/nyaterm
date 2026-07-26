use super::*;

use crate::models::{QuickCommandVariableDef, QuickCommandVariablePromptState};

impl NyaTermApp {
    pub(in crate::features) fn cancel_quick_command_variable_prompt(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        self.quick_command_variable_prompt = None;
        self.terminal_status = "quick command variables cancelled".to_string();
        cx.notify();
    }

    pub(in crate::features) fn submit_quick_command_variable_prompt(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        let Some(prompt) = self.quick_command_variable_prompt.take() else {
            return;
        };
        let mut command_text = prompt.command.clone();
        for variable in &prompt.variables {
            command_text = command_text.replace(&variable.raw, &variable.value);
        }
        self.send_resolved_quick_command(
            prompt.command_id,
            prompt.label,
            command_text,
            prompt.execute,
            prompt.send_to_all,
            cx,
        );
    }

    pub(in crate::features) fn focus_quick_command_variable(
        &mut self,
        index: usize,
        cx: &mut Context<Self>,
    ) {
        let Some(prompt) = self.quick_command_variable_prompt.as_mut() else {
            return;
        };
        if index < prompt.variables.len() {
            prompt.focused_index = index;
            cx.notify();
        }
    }

    pub(in crate::features) fn cycle_quick_command_variable_option(
        &mut self,
        index: usize,
        delta: isize,
        cx: &mut Context<Self>,
    ) {
        let Some(prompt) = self.quick_command_variable_prompt.as_mut() else {
            return;
        };
        let Some(variable) = prompt.variables.get_mut(index) else {
            return;
        };
        if variable.options.is_empty() {
            return;
        }
        let current = variable
            .options
            .iter()
            .position(|option| option == &variable.value)
            .unwrap_or(0);
        let len = variable.options.len() as isize;
        let next = (current as isize + delta).rem_euclid(len) as usize;
        let value = variable.options[next].clone();
        sync_quick_command_variable_value(prompt, index, value);
        prompt.focused_index = index;
        cx.notify();
    }

    pub(in crate::features) fn handle_quick_command_variable_key_down(
        &mut self,
        event: &KeyDownEvent,
        cx: &mut Context<Self>,
    ) {
        self.mark_user_activity();
        let keystroke = &event.keystroke;
        let primary = keystroke.modifiers.platform || keystroke.modifiers.control;
        if primary && !keystroke.modifiers.alt && matches!(keystroke.key.as_str(), "v" | "V") {
            if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text())
                && let Some(prompt) = self.quick_command_variable_prompt.as_mut()
                && let Some(variable) = prompt.variables.get(prompt.focused_index)
                && variable.options.is_empty()
            {
                let mut value = variable.value.clone();
                value.push_str(&text);
                sync_quick_command_variable_value(prompt, prompt.focused_index, value);
                cx.notify();
            }
            return;
        }
        if primary || keystroke.modifiers.alt || keystroke.modifiers.function {
            return;
        }

        match keystroke.key.as_str() {
            "escape" => self.cancel_quick_command_variable_prompt(cx),
            "enter" => self.submit_quick_command_variable_prompt(cx),
            "tab" => {
                if let Some(prompt) = self.quick_command_variable_prompt.as_mut() {
                    let len = prompt.variables.len().max(1);
                    prompt.focused_index = (prompt.focused_index + 1) % len;
                    cx.notify();
                }
            }
            "backspace" => {
                if let Some(prompt) = self.quick_command_variable_prompt.as_mut()
                    && let Some(variable) = prompt.variables.get(prompt.focused_index)
                    && variable.options.is_empty()
                {
                    let mut value = variable.value.clone();
                    value.pop();
                    sync_quick_command_variable_value(prompt, prompt.focused_index, value);
                    cx.notify();
                }
            }
            "left" | "up" => {
                if let Some(prompt) = self.quick_command_variable_prompt.as_ref() {
                    self.cycle_quick_command_variable_option(prompt.focused_index, -1, cx);
                }
            }
            "right" | "down" => {
                if let Some(prompt) = self.quick_command_variable_prompt.as_ref() {
                    self.cycle_quick_command_variable_option(prompt.focused_index, 1, cx);
                }
            }
            _ => {
                if let Some(input) = keystroke
                    .key_char
                    .as_deref()
                    .filter(|input| !input.is_empty())
                    && let Some(prompt) = self.quick_command_variable_prompt.as_mut()
                    && let Some(variable) = prompt.variables.get(prompt.focused_index)
                    && variable.options.is_empty()
                {
                    let mut value = variable.value.clone();
                    value.push_str(input);
                    sync_quick_command_variable_value(prompt, prompt.focused_index, value);
                    cx.notify();
                }
            }
        }
    }
}

pub(super) fn parse_quick_command_variables(command: &str) -> Vec<QuickCommandVariableDef> {
    let mut variables = Vec::new();
    let mut seen = HashSet::<String>::new();
    let mut rest = command;
    while let Some(start) = rest.find("{{") {
        let after_start = &rest[start + 2..];
        let Some(end) = after_start.find("}}") else {
            break;
        };
        let content = &after_start[..end];
        let raw = format!("{{{{{content}}}}}");
        rest = &after_start[end + 2..];
        if content.is_empty() || !seen.insert(raw.clone()) {
            continue;
        }

        let (name, options, value) = if content.contains('|') {
            let mut parts = content.split('|');
            let name = parts.next().unwrap_or_default();
            let options = parts.next().unwrap_or_default();
            let options = options
                .split(',')
                .map(str::trim)
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>();
            let value = options.first().cloned().unwrap_or_default();
            (name.trim().to_string(), options, value)
        } else if content.contains('=') {
            let mut parts = content.split('=');
            let name = parts.next().unwrap_or_default();
            let default_value = parts.next().unwrap_or_default();
            (
                name.trim().to_string(),
                Vec::new(),
                default_value.trim().to_string(),
            )
        } else {
            (content.trim().to_string(), Vec::new(), String::new())
        };
        variables.push(QuickCommandVariableDef {
            raw,
            name,
            options,
            value,
        });
    }
    let mut values_by_name = HashMap::new();
    for variable in &variables {
        values_by_name.insert(variable.name.clone(), variable.value.clone());
    }
    for variable in &mut variables {
        if let Some(value) = values_by_name.get(&variable.name) {
            variable.value = value.clone();
        }
    }
    variables
}

fn sync_quick_command_variable_value(
    prompt: &mut QuickCommandVariablePromptState,
    index: usize,
    value: String,
) {
    let Some(variable) = prompt.variables.get(index) else {
        return;
    };
    let name = variable.name.clone();
    for variable in &mut prompt.variables {
        if variable.name == name {
            variable.value = value.clone();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_quick_command_variables_like_tauri_dialog() {
        let variables =
            parse_quick_command_variables("ssh {{ host }} --env {{mode=prod}} {{target|a, b,}}");

        assert_eq!(variables.len(), 3);
        assert_eq!(variables[0].raw, "{{ host }}");
        assert_eq!(variables[0].name, "host");
        assert_eq!(variables[0].value, "");
        assert!(variables[0].options.is_empty());
        assert_eq!(variables[1].raw, "{{mode=prod}}");
        assert_eq!(variables[1].name, "mode");
        assert_eq!(variables[1].value, "prod");
        assert_eq!(variables[2].raw, "{{target|a, b,}}");
        assert_eq!(variables[2].name, "target");
        assert_eq!(variables[2].options, ["a", "b", ""]);
        assert_eq!(variables[2].value, "a");
    }

    #[test]
    fn deduplicates_by_raw_token_and_keeps_js_split_semantics() {
        let variables = parse_quick_command_variables("{{x=a=b}} {{choice|one|two}} {{x=a=b}}");

        assert_eq!(variables.len(), 2);
        assert_eq!(variables[0].raw, "{{x=a=b}}");
        assert_eq!(variables[0].name, "x");
        assert_eq!(variables[0].value, "a");
        assert_eq!(variables[1].raw, "{{choice|one|two}}");
        assert_eq!(variables[1].name, "choice");
        assert_eq!(variables[1].options, ["one"]);
        assert_eq!(variables[1].value, "one");
    }

    #[test]
    fn shares_values_for_variables_with_the_same_name() {
        let variables = parse_quick_command_variables("{{host=prod}} {{host=dev}}");

        assert_eq!(variables.len(), 2);
        assert_eq!(variables[0].value, "dev");
        assert_eq!(variables[1].value, "dev");

        let mut prompt = QuickCommandVariablePromptState {
            command_id: "cmd".to_string(),
            label: "Command".to_string(),
            command: "{{host=prod}} {{host=dev}}".to_string(),
            execute: true,
            send_to_all: false,
            variables,
            focused_index: 0,
        };
        sync_quick_command_variable_value(&mut prompt, 0, "stage".to_string());

        assert_eq!(prompt.variables[0].value, "stage");
        assert_eq!(prompt.variables[1].value, "stage");
    }
}
