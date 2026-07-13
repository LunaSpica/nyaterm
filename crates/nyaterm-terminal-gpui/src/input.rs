use super::*;

pub fn terminal_key_bytes(event: &KeyDownEvent) -> Option<Vec<u8>> {
    let keystroke = &event.keystroke;
    if keystroke.modifiers.function {
        return None;
    }
    // Super/Win key combos are reserved for the shell/OS.
    if keystroke.modifiers.platform && !keystroke.modifiers.control && !keystroke.modifiers.alt {
        return None;
    }

    let key = keystroke.key.as_str();
    let ctrl = keystroke.modifiers.control;
    let alt = keystroke.modifiers.alt;
    let shift = keystroke.modifiers.shift;

    // Ctrl+Arrow / Alt+Arrow CSI sequences (Tauri XTerminal word-nav parity).
    if matches!(key, "up" | "down" | "left" | "right") {
        if ctrl && !alt && !shift {
            let suffix = match key {
                "up" => b"\x1b[1;5A",
                "down" => b"\x1b[1;5B",
                "right" => b"\x1b[1;5C",
                "left" => b"\x1b[1;5D",
                _ => unreachable!(),
            };
            return Some(suffix.to_vec());
        }
        if alt && !ctrl && !shift {
            let suffix = match key {
                "up" => b"\x1b[1;3A",
                "down" => b"\x1b[1;3B",
                "right" => b"\x1b[1;3C",
                "left" => b"\x1b[1;3D",
                _ => unreachable!(),
            };
            return Some(suffix.to_vec());
        }
    }

    if ctrl && !alt {
        return control_key_bytes(key);
    }

    // Plain navigation / editing keys (no modifiers other than shift where irrelevant).
    if !ctrl && !alt && !keystroke.modifiers.platform {
        match key {
            "enter" => return Some(b"\r".to_vec()),
            "backspace" => return Some(vec![0x7f]),
            "tab" => return Some(b"\t".to_vec()),
            "escape" => return Some(vec![0x1b]),
            "up" => return Some(b"\x1b[A".to_vec()),
            "down" => return Some(b"\x1b[B".to_vec()),
            "right" => return Some(b"\x1b[C".to_vec()),
            "left" => return Some(b"\x1b[D".to_vec()),
            "home" => return Some(b"\x1b[H".to_vec()),
            "end" => return Some(b"\x1b[F".to_vec()),
            "delete" => return Some(b"\x1b[3~".to_vec()),
            "pageup" => return Some(b"\x1b[5~".to_vec()),
            "pagedown" => return Some(b"\x1b[6~".to_vec()),
            _ => {}
        }

        return keystroke
            .key_char
            .as_deref()
            .filter(|value| !value.is_empty())
            .map(|value| value.as_bytes().to_vec());
    }

    None
}
pub(super) fn control_key_bytes(key: &str) -> Option<Vec<u8>> {
    // Ctrl+Arrow handled above.
    if matches!(key, "up" | "down" | "left" | "right") {
        return None;
    }
    let byte = match key {
        "space" => 0x00,
        "left_bracket" | "[" => 0x1b,
        "backslash" | "\\" => 0x1c,
        "right_bracket" | "]" => 0x1d,
        "6" => 0x1e,
        "slash" | "/" => 0x1f,
        value if value.len() == 1 => {
            let byte = value.as_bytes()[0].to_ascii_lowercase();
            if byte.is_ascii_lowercase() {
                byte - b'a' + 1
            } else {
                return None;
            }
        }
        _ => return None,
    };
    Some(vec![byte])
}
pub fn trim_terminal_output(output: &mut String) {
    const MAX_BYTES: usize = 64 * 1024;
    if output.len() <= MAX_BYTES {
        return;
    }
    let drain_to = output
        .char_indices()
        .find_map(|(index, _)| (index >= output.len() - MAX_BYTES).then_some(index))
        .unwrap_or(0);
    output.drain(..drain_to);
}
pub fn initial_terminal_screen(banner: &str) -> TerminalScreen {
    terminal_screen_from_output(banner)
}
pub fn terminal_screen_from_output(output: &str) -> TerminalScreen {
    let mut screen = TerminalScreen::default();
    screen.advance(output.as_bytes());
    screen
}
