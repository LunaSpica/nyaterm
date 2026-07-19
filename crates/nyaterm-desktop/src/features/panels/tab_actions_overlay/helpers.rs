pub(super) fn clamp_tab_actions_position(
    x: f32,
    y: f32,
    menu_w: f32,
    menu_h: f32,
    viewport_w: f32,
    viewport_h: f32,
) -> (f32, f32) {
    let max_x = (viewport_w - menu_w - 8.0).max(8.0);
    let max_y = (viewport_h - menu_h - 8.0).max(8.0);
    (x.clamp(8.0, max_x), y.clamp(8.0, max_y))
}

pub(super) fn tab_actions_submenu_position(
    menu_x: f32,
    menu_y: f32,
    menu_width: f32,
    submenu_width: f32,
    trigger_offset: f32,
    submenu_height: f32,
    viewport_width: f32,
    viewport_height: f32,
) -> (f32, f32) {
    let margin = 8.;
    let gap = 4.;
    let right = menu_x + menu_width + gap;
    let x = if right + submenu_width <= viewport_width - margin {
        right
    } else {
        (menu_x - submenu_width - gap).max(margin)
    };
    let max_y = (viewport_height - submenu_height - margin).max(margin);
    (x, (menu_y + trigger_offset).clamp(margin, max_y))
}

#[cfg(test)]
mod tests {
    use super::tab_actions_submenu_position;

    #[test]
    fn submenu_opens_right_when_space_is_available() {
        assert_eq!(
            tab_actions_submenu_position(100., 80., 240., 176., 0., 104., 800., 600.),
            (344., 80.)
        );
    }

    #[test]
    fn submenu_flips_left_and_clamps_to_bottom() {
        assert_eq!(
            tab_actions_submenu_position(540., 430., 240., 176., 120., 104., 800., 600.),
            (360., 488.)
        );
    }
}
