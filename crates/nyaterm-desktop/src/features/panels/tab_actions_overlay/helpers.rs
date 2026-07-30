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
    menu: TabActionsMenuGeometry,
    submenu: TabActionsSubmenuGeometry,
    viewport: (f32, f32),
) -> (f32, f32) {
    let (viewport_width, viewport_height) = viewport;
    let margin = 8.;
    let gap = 4.;
    let right = menu.x + menu.width + gap;
    let x = if right + submenu.width <= viewport_width - margin {
        right
    } else {
        (menu.x - submenu.width - gap).max(margin)
    };
    let max_y = (viewport_height - submenu.height - margin).max(margin);
    (x, (menu.y + submenu.trigger_offset).clamp(margin, max_y))
}

#[cfg(test)]
mod tests {
    use super::{TabActionsMenuGeometry, TabActionsSubmenuGeometry, tab_actions_submenu_position};

    #[test]
    fn submenu_opens_right_when_space_is_available() {
        assert_eq!(
            tab_actions_submenu_position(
                TabActionsMenuGeometry {
                    x: 100.,
                    y: 80.,
                    width: 240.,
                },
                TabActionsSubmenuGeometry {
                    width: 176.,
                    trigger_offset: 0.,
                    height: 104.,
                },
                (800., 600.),
            ),
            (344., 80.)
        );
    }

    #[test]
    fn submenu_flips_left_and_clamps_to_bottom() {
        assert_eq!(
            tab_actions_submenu_position(
                TabActionsMenuGeometry {
                    x: 540.,
                    y: 430.,
                    width: 240.,
                },
                TabActionsSubmenuGeometry {
                    width: 176.,
                    trigger_offset: 120.,
                    height: 104.,
                },
                (800., 600.),
            ),
            (360., 488.)
        );
    }
}
#[derive(Clone, Copy)]
pub(super) struct TabActionsMenuGeometry {
    pub x: f32,
    pub y: f32,
    pub width: f32,
}

#[derive(Clone, Copy)]
pub(super) struct TabActionsSubmenuGeometry {
    pub width: f32,
    pub trigger_offset: f32,
    pub height: f32,
}
