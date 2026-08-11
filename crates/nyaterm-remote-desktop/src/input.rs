use std::collections::HashSet;

use crate::RdpInputEvent;

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct RemoteKey {
    pub scan_code: u16,
    pub extended: bool,
}

pub fn viewport_to_remote(
    point_x: f32,
    point_y: f32,
    viewport_width: f32,
    viewport_height: f32,
    remote_width: u32,
    remote_height: u32,
) -> Option<(u32, u32)> {
    if viewport_width <= 0.0 || viewport_height <= 0.0 || remote_width == 0 || remote_height == 0 {
        return None;
    }
    let scale = (viewport_width / remote_width as f32).min(viewport_height / remote_height as f32);
    let image_width = remote_width as f32 * scale;
    let image_height = remote_height as f32 * scale;
    let left = (viewport_width - image_width) * 0.5;
    let top = (viewport_height - image_height) * 0.5;
    if point_x < left
        || point_y < top
        || point_x >= left + image_width
        || point_y >= top + image_height
    {
        return None;
    }
    let x = ((point_x - left) / scale)
        .floor()
        .clamp(0.0, remote_width.saturating_sub(1) as f32) as u32;
    let y = ((point_y - top) / scale)
        .floor()
        .clamp(0.0, remote_height.saturating_sub(1) as f32) as u32;
    Some((x, y))
}

#[derive(Debug, Default)]
pub struct KeyMapper {
    pressed: HashSet<RemoteKey>,
    right_alt_pressed: bool,
}

impl KeyMapper {
    pub fn map_key(name: &str) -> Option<RemoteKey> {
        let normalized = name.trim().to_ascii_lowercase();
        let (scan_code, extended) = match normalized.as_str() {
            "escape" | "esc" => (0x01, false),
            "1" => (0x02, false),
            "2" => (0x03, false),
            "3" => (0x04, false),
            "4" => (0x05, false),
            "5" => (0x06, false),
            "6" => (0x07, false),
            "7" => (0x08, false),
            "8" => (0x09, false),
            "9" => (0x0a, false),
            "0" => (0x0b, false),
            "-" | "minus" => (0x0c, false),
            "=" | "equal" => (0x0d, false),
            "backspace" => (0x0e, false),
            "tab" => (0x0f, false),
            "q" => (0x10, false),
            "w" => (0x11, false),
            "e" => (0x12, false),
            "r" => (0x13, false),
            "t" => (0x14, false),
            "y" => (0x15, false),
            "u" => (0x16, false),
            "i" => (0x17, false),
            "o" => (0x18, false),
            "p" => (0x19, false),
            "[" | "bracketleft" => (0x1a, false),
            "]" | "bracketright" => (0x1b, false),
            "enter" | "return" => (0x1c, false),
            "control" | "ctrl" | "controlleft" => (0x1d, false),
            "a" => (0x1e, false),
            "s" => (0x1f, false),
            "d" => (0x20, false),
            "f" => (0x21, false),
            "g" => (0x22, false),
            "h" => (0x23, false),
            "j" => (0x24, false),
            "k" => (0x25, false),
            "l" => (0x26, false),
            ";" | "semicolon" => (0x27, false),
            "'" | "quote" => (0x28, false),
            "`" | "backquote" => (0x29, false),
            "shift" | "shiftleft" => (0x2a, false),
            "\\" | "backslash" => (0x2b, false),
            "z" => (0x2c, false),
            "x" => (0x2d, false),
            "c" => (0x2e, false),
            "v" => (0x2f, false),
            "b" => (0x30, false),
            "n" => (0x31, false),
            "m" => (0x32, false),
            "," | "comma" => (0x33, false),
            "." | "period" => (0x34, false),
            "/" | "slash" => (0x35, false),
            "shiftright" => (0x36, false),
            "alt" | "altleft" => (0x38, false),
            "space" => (0x39, false),
            "capslock" => (0x3a, false),
            "f1" => (0x3b, false),
            "f2" => (0x3c, false),
            "f3" => (0x3d, false),
            "f4" => (0x3e, false),
            "f5" => (0x3f, false),
            "f6" => (0x40, false),
            "f7" => (0x41, false),
            "f8" => (0x42, false),
            "f9" => (0x43, false),
            "f10" => (0x44, false),
            "numlock" => (0x45, false),
            "scrolllock" => (0x46, false),
            "f11" => (0x57, false),
            "f12" => (0x58, false),
            "controlright" => (0x1d, true),
            "altright" | "altgraph" => (0x38, true),
            "home" => (0x47, true),
            "arrowup" | "up" => (0x48, true),
            "pageup" => (0x49, true),
            "arrowleft" | "left" => (0x4b, true),
            "arrowright" | "right" => (0x4d, true),
            "end" => (0x4f, true),
            "arrowdown" | "down" => (0x50, true),
            "pagedown" => (0x51, true),
            "insert" => (0x52, true),
            "delete" => (0x53, true),
            "meta" | "super" | "metaleft" => (0x5b, true),
            "metaright" => (0x5c, true),
            "contextmenu" => (0x5d, true),
            _ => return None,
        };
        Some(RemoteKey {
            scan_code,
            extended,
        })
    }

    pub fn key_down(&mut self, name: &str, repeat: bool) -> Option<RdpInputEvent> {
        let key = Self::map_key(name)?;
        let was_pressed = !self.pressed.insert(key);
        if name.eq_ignore_ascii_case("altright") || name.eq_ignore_ascii_case("altgraph") {
            self.right_alt_pressed = true;
        }
        Some(RdpInputEvent::KeyDown {
            scan_code: key.scan_code,
            extended: key.extended,
            repeat: repeat || was_pressed,
        })
    }

    pub fn key_up(&mut self, name: &str) -> Option<RdpInputEvent> {
        let key = Self::map_key(name)?;
        if !self.pressed.remove(&key) {
            return None;
        }
        if name.eq_ignore_ascii_case("altright") || name.eq_ignore_ascii_case("altgraph") {
            self.right_alt_pressed = false;
        }
        Some(RdpInputEvent::KeyUp {
            scan_code: key.scan_code,
            extended: key.extended,
            repeat: false,
        })
    }

    pub fn alt_gr_active(&self) -> bool {
        self.right_alt_pressed
    }

    pub fn release_all(&mut self) -> Option<RdpInputEvent> {
        if self.pressed.is_empty() {
            return None;
        }
        self.pressed.clear();
        self.right_alt_pressed = false;
        Some(RdpInputEvent::ReleaseAllKeys)
    }
}

#[cfg(test)]
mod tests {
    use crate::{KeyMapper, RdpInputEvent, viewport_to_remote};

    #[test]
    fn maps_contain_geometry_and_rejects_black_bars() {
        assert_eq!(
            viewport_to_remote(50.0, 25.0, 100.0, 50.0, 200, 100),
            Some((100, 50))
        );
        assert_eq!(viewport_to_remote(50.0, 5.0, 100.0, 100.0, 200, 100), None);
        assert_eq!(
            viewport_to_remote(50.0, 50.0, 100.0, 100.0, 200, 100),
            Some((100, 50))
        );
        assert_eq!(viewport_to_remote(5.0, 50.0, 100.0, 100.0, 100, 200), None);
    }

    #[test]
    fn maps_navigation_alt_gr_repeat_and_release_all() {
        assert_eq!(KeyMapper::map_key("ArrowLeft").unwrap().scan_code, 0x4b);
        assert!(KeyMapper::map_key("ArrowLeft").unwrap().extended);
        let mut mapper = KeyMapper::default();
        assert!(matches!(
            mapper.key_down("AltRight", false),
            Some(RdpInputEvent::KeyDown {
                extended: true,
                repeat: false,
                ..
            })
        ));
        assert!(mapper.alt_gr_active());
        assert!(matches!(
            mapper.key_down("AltRight", false),
            Some(RdpInputEvent::KeyDown { repeat: true, .. })
        ));
        assert_eq!(mapper.release_all(), Some(RdpInputEvent::ReleaseAllKeys));
        assert!(!mapper.alt_gr_active());
    }
}
