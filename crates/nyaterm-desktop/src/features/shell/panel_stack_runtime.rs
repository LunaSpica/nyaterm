use std::collections::HashSet;

use gpui::{
    AnyElement, App, AppContext as _, ClickEvent, Context, InteractiveElement as _, IntoElement,
    MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, ParentElement as _, SharedString,
    StatefulInteractiveElement as _, Styled as _, Window, div, prelude::FluentBuilder as _, px,
    rgb, svg,
};
use nyaterm_core::{AgentCommandExecutionMode, truncate_preview};

use crate::features::{ChromeTooltip, NyaTermApp, TextInputSetup, panel_header_with_actions};
use crate::models::{
    ActivityBarZone, MainMode, NavItem, NetworkTab, PanelSide, RightFocus, SecurityAuthTab,
    SettingsTab,
};
use crate::theme::ThemePalette;

const EXCLUSIVE_PANEL_IDS: &[&str] = &["aiAssistant"];
const NON_PANEL_IDS: &[&str] = &["settings", "lock", "quickCmdBar", "serialSend"];

impl NyaTermApp {
    pub(in crate::features) fn is_exclusive_panel_id(id: &str) -> bool {
        EXCLUSIVE_PANEL_IDS.contains(&id)
    }

    pub(in crate::features) fn is_stackable_panel_id(id: &str) -> bool {
        !NON_PANEL_IDS.contains(&id) && !Self::is_exclusive_panel_id(id)
    }

    pub(in crate::features) fn toggle_panel_multi_open(&mut self, cx: &mut Context<Self>) {
        self.shell.panels.multi_open = !self.shell.panels.multi_open;
        if self.shell.panels.multi_open {
            if self.shell.panels.left_open.is_empty() {
                if let Some(panel) = self.shell.panels.active_left {
                    let id = panel.persistence_id().to_string();
                    if Self::is_stackable_panel_id(&id) {
                        self.shell.panels.left_open.push(id);
                    }
                }
            }
            if self.shell.panels.right_open.is_empty() {
                if let Some(panel) = self.shell.panels.active_right {
                    let id = panel.persistence_id().to_string();
                    if Self::is_stackable_panel_id(&id) {
                        self.shell.panels.right_open.push(id);
                    }
                }
            }
            self.terminal.view.status = "multi-open panels enabled".to_string();
        } else {
            // Collapse to active-only mode.
            if self.shell.panels.active_left.is_none() {
                self.shell.panels.active_left = self
                    .shell
                    .panels
                    .left_open
                    .first()
                    .and_then(|id| NavItem::from_persistence_id(id));
            }
            if self.shell.panels.active_right.is_none() {
                self.shell.panels.active_right = self
                    .shell
                    .panels
                    .right_open
                    .first()
                    .and_then(|id| NavItem::from_persistence_id(id));
            }
            self.shell.panels.left_open.clear();
            self.shell.panels.right_open.clear();
            self.terminal.view.status = "single panel mode".to_string();
        }
        self.persist_ui_layout();
        cx.notify();
    }

    pub(in crate::features) fn side_open_panel_ids(&self, side: PanelSide) -> Vec<String> {
        if !self.shell.panels.multi_open {
            let active = match side {
                PanelSide::Left => self.shell.panels.active_left,
                PanelSide::Right => self.shell.panels.active_right,
            };
            return active
                .map(|item| item.persistence_id().to_string())
                .into_iter()
                .collect();
        }

        let open = match side {
            PanelSide::Left => &self.shell.panels.left_open,
            PanelSide::Right => &self.shell.panels.right_open,
        };
        if open.is_empty() {
            return Vec::new();
        }
        let open_set: HashSet<_> = open.iter().cloned().collect();
        let zones = match side {
            PanelSide::Left => [ActivityBarZone::LeftTop, ActivityBarZone::LeftBottom],
            PanelSide::Right => [ActivityBarZone::RightTop, ActivityBarZone::RightBottom],
        };
        let mut ordered = Vec::new();
        for zone in zones {
            for id in self.shell.chrome.activity_bar_layout.zone(zone) {
                if open_set.contains(id) && Self::is_stackable_panel_id(id) {
                    ordered.push(id.clone());
                }
            }
        }
        ordered
    }

    pub(in crate::features) fn side_overlay_panel(&self, side: PanelSide) -> Option<NavItem> {
        if !self.shell.panels.multi_open {
            return None;
        }
        let active = match side {
            PanelSide::Left => self.shell.panels.active_left,
            PanelSide::Right => self.shell.panels.active_right,
        }?;
        let id = active.persistence_id();
        Self::is_exclusive_panel_id(id).then_some(active)
    }

    pub(in crate::features) fn panel_stack_weight(&self, panel_id: &str) -> f32 {
        self.shell.panels.stack_weight(panel_id)
    }

    pub(in crate::features) fn panel_side_for_item(&self, item: NavItem) -> Option<PanelSide> {
        self.shell
            .chrome
            .activity_bar_layout
            .side_for_entry(item.persistence_id())
            .or_else(|| item.is_left_panel().then_some(PanelSide::Left))
            .or_else(|| item.is_right_panel().then_some(PanelSide::Right))
    }

    pub(in crate::features) fn open_or_toggle_panel(
        &mut self,
        item: NavItem,
        cx: &mut Context<Self>,
    ) {
        if item == NavItem::Settings || item.opens_settings() {
            self.open_page(NavItem::Settings, cx);
            return;
        }
        if !self.shell.panels.multi_open {
            self.open_panel(item, cx);
            return;
        }

        let id = item.persistence_id().to_string();
        let Some(side) = self.panel_side_for_item(item) else {
            self.open_panel(item, cx);
            return;
        };

        self.shell.navigation.main_mode = MainMode::Workspace;
        self.shell.navigation.selected_nav = item;
        if item == NavItem::Recording {
            self.shell.panels.right_focus = RightFocus::Recording;
        } else {
            self.shell.panels.right_focus = RightFocus::Default;
        }

        if Self::is_exclusive_panel_id(&id) {
            let active = match side {
                PanelSide::Left => self.shell.panels.active_left,
                PanelSide::Right => self.shell.panels.active_right,
            };
            if active == Some(item) {
                // Dismiss exclusive overlay to stack.
                let fallback = self
                    .side_open_panel_ids(side)
                    .into_iter()
                    .find_map(|open_id| NavItem::from_persistence_id(&open_id));
                match side {
                    PanelSide::Left => {
                        self.shell.panels.active_left = fallback;
                        self.shell.panels.left_collapsed = fallback.is_none();
                    }
                    PanelSide::Right => {
                        self.shell.panels.active_right = fallback;
                        self.shell.panels.right_collapsed = fallback.is_none();
                    }
                }
                self.terminal.view.status = format!("{} closed", item.label());
            } else {
                match side {
                    PanelSide::Left => {
                        self.shell.panels.active_left = Some(item);
                        self.shell.panels.left_collapsed = false;
                    }
                    PanelSide::Right => {
                        self.shell.panels.active_right = Some(item);
                        self.shell.panels.right_collapsed = false;
                    }
                }
                self.terminal.view.status = format!("{} opened", item.label());
            }
            self.persist_ui_layout();
            cx.notify();
            return;
        }

        let open_list = match side {
            PanelSide::Left => &mut self.shell.panels.left_open,
            PanelSide::Right => &mut self.shell.panels.right_open,
        };
        let is_open = open_list.iter().any(|value| value == &id);
        let active = match side {
            PanelSide::Left => self.shell.panels.active_left,
            PanelSide::Right => self.shell.panels.active_right,
        };

        // If exclusive overlay is showing and stacked panel already open, reveal stack.
        if is_open
            && active
                .map(|item| Self::is_exclusive_panel_id(item.persistence_id()))
                .unwrap_or(false)
        {
            match side {
                PanelSide::Left => {
                    self.shell.panels.active_left = Some(item);
                    self.shell.panels.left_collapsed = false;
                }
                PanelSide::Right => {
                    self.shell.panels.active_right = Some(item);
                    self.shell.panels.right_collapsed = false;
                }
            }
            self.terminal.view.status = format!("{} focused", item.label());
            self.persist_ui_layout();
            cx.notify();
            return;
        }

        if is_open {
            open_list.retain(|value| value != &id);
            let next_active = if open_list.is_empty() {
                None
            } else if active
                .map(|item| item.persistence_id() == id)
                .unwrap_or(false)
            {
                open_list
                    .first()
                    .and_then(|value| NavItem::from_persistence_id(value))
            } else {
                active.filter(|item| open_list.iter().any(|value| value == item.persistence_id()))
            };
            match side {
                PanelSide::Left => {
                    self.shell.panels.active_left = next_active;
                    self.shell.panels.left_collapsed =
                        next_active.is_none() && self.shell.panels.left_open.is_empty();
                }
                PanelSide::Right => {
                    self.shell.panels.active_right = next_active;
                    self.shell.panels.right_collapsed =
                        next_active.is_none() && self.shell.panels.right_open.is_empty();
                }
            }
            self.terminal.view.status = format!("{} closed", item.label());
        } else {
            open_list.push(id);
            match side {
                PanelSide::Left => {
                    self.shell.panels.active_left = Some(item);
                    self.shell.panels.left_collapsed = false;
                }
                PanelSide::Right => {
                    self.shell.panels.active_right = Some(item);
                    self.shell.panels.right_collapsed = false;
                }
            }
            self.terminal.view.status = format!("{} opened", item.label());
        }
        self.persist_ui_layout();
        cx.notify();
    }

    pub(in crate::features) fn ensure_panel_in_stack(&mut self, item: NavItem) {
        self.shell.navigation.main_mode = MainMode::Workspace;
        self.shell.navigation.selected_nav = item;
        if !self.shell.panels.multi_open {
            self.ensure_panel_open(item);
            return;
        }
        let id = item.persistence_id().to_string();
        match self.panel_side_for_item(item) {
            Some(PanelSide::Left) => {
                self.shell.panels.left_collapsed = false;
                if Self::is_exclusive_panel_id(&id) {
                    self.shell.panels.active_left = Some(item);
                } else {
                    if !self.shell.panels.left_open.iter().any(|value| value == &id) {
                        self.shell.panels.left_open.push(id);
                    }
                    self.shell.panels.active_left = Some(item);
                }
            }
            Some(PanelSide::Right) => {
                self.shell.panels.right_collapsed = false;
                self.shell.panels.right_focus = if item == NavItem::Recording {
                    RightFocus::Recording
                } else {
                    RightFocus::Default
                };
                if Self::is_exclusive_panel_id(&id) {
                    self.shell.panels.active_right = Some(item);
                } else {
                    if !self
                        .shell
                        .panels
                        .right_open
                        .iter()
                        .any(|value| value == &id)
                    {
                        self.shell.panels.right_open.push(id);
                    }
                    self.shell.panels.active_right = Some(item);
                }
            }
            None => {}
        }
    }

    pub(in crate::features) fn start_panel_stack_resize(
        &mut self,
        side: PanelSide,
        above_id: String,
        below_id: String,
        event: &MouseDownEvent,
        container_height: f32,
        cx: &mut Context<Self>,
    ) {
        self.shell.panels.start_stack_resize(
            side,
            above_id,
            below_id,
            event.position.y,
            container_height,
        );
        self.terminal.view.status = "resizing panel stack".to_string();
        cx.notify();
    }

    pub(in crate::features) fn update_panel_stack_resize(
        &mut self,
        event: &MouseMoveEvent,
        cx: &mut Context<Self>,
    ) {
        if self.shell.panels.update_stack_resize(event.position.y) {
            cx.notify();
        }
    }

    pub(in crate::features) fn finish_panel_stack_resize(
        &mut self,
        _event: &MouseUpEvent,
        cx: &mut Context<Self>,
    ) {
        if self.shell.panels.finish_stack_resize() {
            self.persist_ui_layout();
            self.terminal.view.status = "panel stack sizes saved".to_string();
            cx.notify();
        }
    }

    pub(in crate::features) fn sync_panel_stack_to_settings(&mut self) {
        self.settings.summary.ui_panel_multi_open = self.shell.panels.multi_open;
        self.settings.summary.ui_left_open_panels = self.shell.panels.left_open.clone();
        self.settings.summary.ui_right_open_panels = self.shell.panels.right_open.clone();
        self.settings.summary.ui_panel_stack_sizes = self
            .shell
            .panels
            .stack_sizes
            .iter()
            .filter_map(|(key, value)| {
                let scaled = (*value * 1000.).round();
                (scaled.is_finite() && scaled > 0.).then(|| (key.clone(), scaled as u32))
            })
            .collect();
    }

    pub(in crate::features) fn apply_panel_stack_from_settings(&mut self) {
        self.shell.panels.multi_open = self.settings.summary.ui_panel_multi_open;
        self.shell.panels.left_open = self.settings.summary.ui_left_open_panels.clone();
        self.shell.panels.right_open = self.settings.summary.ui_right_open_panels.clone();
        self.shell.panels.stack_sizes = self
            .settings
            .summary
            .ui_panel_stack_sizes
            .iter()
            .filter_map(|(key, value)| (*value > 0).then(|| (key.clone(), (*value as f32) / 1000.)))
            .collect();
        if self.shell.panels.multi_open {
            if self.shell.panels.left_open.is_empty() {
                if let Some(panel) = self.shell.panels.active_left {
                    let id = panel.persistence_id().to_string();
                    if Self::is_stackable_panel_id(&id) {
                        self.shell.panels.left_open.push(id);
                    }
                }
            }
            if self.shell.panels.right_open.is_empty() {
                if let Some(panel) = self.shell.panels.active_right {
                    let id = panel.persistence_id().to_string();
                    if Self::is_stackable_panel_id(&id) {
                        self.shell.panels.right_open.push(id);
                    }
                }
            }
        }
    }

    pub(in crate::features) fn side_panel_stack(
        &mut self,
        side: PanelSide,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        use gpui::relative;

        let open_ids = self.side_open_panel_ids(side);
        let mut stack = if open_ids.is_empty() {
            let fallback = match side {
                PanelSide::Left => self.current_left_panel(),
                PanelSide::Right => self.current_right_panel(),
            }
            .or_else(|| {
                self.shell
                    .chrome
                    .activity_bar_layout
                    .first_panel_on_side(side)
            })
            .unwrap_or(NavItem::Workspace);
            self.single_side_panel(side, fallback, window, cx)
        } else if open_ids.len() == 1 || !self.shell.panels.multi_open {
            let panel = open_ids
                .first()
                .and_then(|id| NavItem::from_persistence_id(id))
                .or_else(|| {
                    self.shell
                        .chrome
                        .activity_bar_layout
                        .first_panel_on_side(side)
                })
                .unwrap_or(NavItem::Workspace);
            self.single_side_panel(side, panel, window, cx)
        } else {
            let weights: Vec<f32> = open_ids
                .iter()
                .map(|id| self.panel_stack_weight(id))
                .collect();
            let total: f32 = weights.iter().sum::<f32>().max(0.001);
            let count = open_ids.len();
            let mut stack = div().size_full().flex().flex_col().min_h_0();
            for (index, panel_id) in open_ids.iter().enumerate() {
                let panel = NavItem::from_persistence_id(panel_id).unwrap_or(NavItem::Transfers);
                let basis = weights[index] / total;
                let meta = self.side_panel_meta(side, panel);
                let title = panel
                    .i18n_key()
                    .map(|key| self.tr(key))
                    .unwrap_or_else(|| panel.panel_title());
                let actions = self.side_panel_header_actions(panel, cx);
                let palette = self.theme_palette();
                let body = match side {
                    PanelSide::Left => self.left_panel_body(panel, window, cx),
                    PanelSide::Right => self.right_panel_body(panel, window, cx),
                };
                stack = stack.child(
                    div()
                        .flex_shrink()
                        .flex_basis(relative(basis))
                        .min_h(px(48.))
                        .flex()
                        .flex_col()
                        .overflow_hidden()
                        .child(panel_header_with_actions(
                            title,
                            meta,
                            palette,
                            self.shell_transparent_color(palette.section_header),
                            actions,
                        ))
                        .child(div().flex_1().min_h_0().overflow_hidden().child(body)),
                );
                if index + 1 < count {
                    let above = panel_id.clone();
                    let below = open_ids[index + 1].clone();
                    stack = stack.child(self.panel_stack_resize_handle(side, above, below, cx));
                }
            }
            stack
        };

        if let Some(overlay) = self.side_overlay_panel(side) {
            stack = stack.opacity(0.);
            return div()
                .relative()
                .size_full()
                .overflow_hidden()
                .child(stack)
                .child(
                    self.single_side_panel(side, overlay, window, cx)
                        .absolute()
                        .top_0()
                        .left_0()
                        .right_0()
                        .bottom_0(),
                );
        }

        stack
    }

    fn single_side_panel(
        &mut self,
        side: PanelSide,
        panel: NavItem,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::Div {
        let meta = self.side_panel_meta(side, panel);
        let title = panel
            .i18n_key()
            .map(|key| self.tr(key))
            .unwrap_or_else(|| panel.panel_title());
        let actions = self.side_panel_header_actions(panel, cx);
        let palette = self.theme_palette();
        let body = match side {
            PanelSide::Left => self.left_panel_body(panel, window, cx),
            PanelSide::Right => self.right_panel_body(panel, window, cx),
        };
        div()
            .size_full()
            .flex()
            .flex_col()
            .child(panel_header_with_actions(
                title,
                meta,
                palette,
                self.shell_transparent_color(palette.section_header),
                actions,
            ))
            .child(div().flex_1().min_h_0().overflow_hidden().child(body))
    }

    /// Tauri PanelHeader meta/actions: Connections shows total count; AI shows model name.
    fn side_panel_meta(&self, _side: PanelSide, panel: NavItem) -> SharedString {
        match panel {
            NavItem::Connections => SharedString::from(""),
            NavItem::AiAssistant => {
                let label = self
                    .ai_selected_model_id()
                    .and_then(|model_id| {
                        self.ai
                            .settings_config()
                            .models
                            .iter()
                            .find(|model| model.id == model_id)
                            .map(|model| truncate_preview(&model.name, 28))
                    })
                    .unwrap_or_else(|| self.tr("ai.notConfigured").to_string());
                SharedString::from(label)
            }
            NavItem::ActiveSessions => SharedString::from(self.active_sessions_header_count()),
            // Tauri NetworkPanel header shows active tab profile count.
            NavItem::Tunnels => {
                let count = match self.connection_state.network_active_tab() {
                    NetworkTab::Tunnels => self.tunnel_state.tunnels().len(),
                    NetworkTab::Proxies => self.tunnel_state.proxies().len(),
                };
                SharedString::from(count.to_string())
            }
            NavItem::Transfers => {
                if self.transfer.browser_view().entries.is_empty() {
                    SharedString::from("")
                } else {
                    SharedString::from(self.transfer.browser_view().entries.len().to_string())
                }
            }
            NavItem::Processes => self
                .session
                .active_ssh_config()
                .and_then(|_| self.remote_ops.loaded_process_count())
                .map(|count| SharedString::from(count.to_string()))
                .unwrap_or_else(|| SharedString::from("")),
            NavItem::Docker => {
                if self.session.active_ssh_config().is_none() {
                    return SharedString::from("");
                }
                let Some(version) = self.remote_ops.docker_engine_version() else {
                    return SharedString::from("");
                };
                SharedString::from(format!("Engine {}", truncate_preview(&version, 24)))
            }
            // Tauri SecurityAuthPanel header actions show active-tab count.
            NavItem::SecurityAuth => {
                let count = match self.security.auth_tab() {
                    SecurityAuthTab::Keys => self.security.ssh_keys().len(),
                    SecurityAuthTab::Passwords => self.security.passwords().len(),
                    SecurityAuthTab::Credentials => self.security.credentials().len(),
                    SecurityAuthTab::Otp => self.security.otp_entries().len(),
                };
                SharedString::from(count.to_string())
            }
            NavItem::Recording => SharedString::from(self.recording_sessions_header_count()),
            NavItem::SyncBackupHistory => SharedString::from(""),
            _ => SharedString::from(""),
        }
    }

    fn side_panel_header_actions(
        &mut self,
        panel: NavItem,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        match panel {
            NavItem::Connections if !self.connection_catalog.connections().is_empty() => Some(
                div()
                    .text_size(px(11.))
                    .text_color(rgb(self.theme_palette().text_dimmed))
                    .child(self.connection_catalog.connections().len().to_string())
                    .into_any_element(),
            ),
            NavItem::AiAssistant => {
                let palette = self.theme_palette();
                let ai_running = self.ai.chat_or_agent_is_running();
                Some(
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .child(header_svg_icon_button(
                            palette,
                            "ai-header-execution-mode-toggle",
                            match self.ai.settings_config().agent_command_execution_mode {
                                AgentCommandExecutionMode::Auto => "icons/ai/exec-auto.svg",
                                AgentCommandExecutionMode::Smart => "icons/ai/exec-smart.svg",
                                AgentCommandExecutionMode::ConfirmEach => {
                                    "icons/ai/exec-confirm.svg"
                                }
                            },
                            self.tr("ai.agentCommandExecutionMode"),
                            !ai_running,
                            cx.listener(|this, _, _, cx| {
                                this.ai.toggle_execution_menu();
                                cx.notify();
                            }),
                        ))
                        .child(header_svg_icon_button(
                            palette,
                            "ai-header-history-toggle",
                            "icons/ai/history.svg",
                            self.tr("ai.history"),
                            true,
                            cx.listener(|this, _, window, cx| {
                                if this.ai.toggle_history() {
                                    this.refresh_ai_session_list(cx);
                                    let query = this.ai.history_query().to_string();
                                    this.reset_text_input("ai.history-search", &query, cx);
                                    let field = this.text_input(
                                        "ai.history-search",
                                        &query,
                                        TextInputSetup::placeholder("Search history..."),
                                        cx,
                                    );
                                    window.focus(&field.read(cx).focus_handle());
                                } else {
                                    this.forget_text_inputs("ai.history-search");
                                }
                                cx.notify();
                            }),
                        ))
                        .child(header_svg_icon_button(
                            palette,
                            "ai-header-open-settings",
                            "icons/ai/settings.svg",
                            self.tr("ai.settings"),
                            true,
                            cx.listener(|this, _, _, cx| {
                                this.ai.close_transient_menus();
                                this.shell.navigation.settings.active_tab = SettingsTab::AiGeneral;
                                this.open_page(NavItem::Settings, cx);
                            }),
                        ))
                        .child(header_svg_icon_button(
                            palette,
                            "ai-header-new-chat",
                            "icons/ai/new.svg",
                            self.tr("ai.newChat"),
                            !ai_running,
                            cx.listener(|this, _, _, cx| {
                                this.start_new_ai_chat(cx);
                            }),
                        ))
                        .into_any_element(),
                )
            }
            NavItem::Stats => {
                let palette = self.theme_palette();
                let can_refresh = self.session.active_ssh_config().is_some()
                    && !self.remote_ops.stats_is_pending();
                Some(
                    header_svg_icon_button(
                        palette,
                        "stats-header-refresh",
                        "icons/fe/refresh.svg",
                        self.tr("resourceMonitor.refresh"),
                        can_refresh,
                        cx.listener(|this, _, window, cx| {
                            this.refresh_stats(window, cx);
                        }),
                    )
                    .into_any_element(),
                )
            }
            NavItem::Processes => {
                let palette = self.theme_palette();
                let can_refresh = self.session.active_ssh_config().is_some()
                    && !self.remote_ops.process_is_pending();
                Some(
                    header_svg_icon_button(
                        palette,
                        "process-header-refresh",
                        "icons/fe/refresh.svg",
                        self.tr("common.refresh"),
                        can_refresh,
                        cx.listener(|this, _, window, cx| {
                            this.refresh_processes(window, cx);
                        }),
                    )
                    .into_any_element(),
                )
            }
            NavItem::Docker => {
                let palette = self.theme_palette();
                let can_refresh = self.session.active_ssh_config().is_some()
                    && !self.remote_ops.docker_is_pending();
                let can_prune = can_refresh && self.remote_ops.docker_can_prune();
                let more_label = self.tr("dockerManager.moreActions").to_string();
                let prune_label = self.tr("dockerManager.prune");
                Some(
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .child(header_svg_icon_button(
                            palette,
                            "docker-header-refresh",
                            "icons/fe/refresh.svg",
                            self.tr("common.refresh"),
                            can_refresh,
                            cx.listener(|this, _, window, cx| {
                                this.refresh_docker(window, cx);
                            }),
                        ))
                        .child(
                            div()
                                .relative()
                                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                                .child(header_svg_icon_button(
                                    palette,
                                    "docker-header-more",
                                    "icons/session/more.svg",
                                    more_label,
                                    can_prune,
                                    cx.listener(|this, _, _, cx| {
                                        this.remote_ops.toggle_docker_header_menu();
                                        cx.notify();
                                    }),
                                ))
                                .when(self.remote_ops.docker_header_menu_open(), |this| {
                                    this.child(
                                        div()
                                            .id("docker-header-more-menu")
                                            .absolute()
                                            .top(px(30.))
                                            .right_0()
                                            .w(px(160.))
                                            .rounded_md()
                                            .border_1()
                                            .border_color(rgb(palette.border))
                                            .bg(self.shell_surface_color(palette.surface))
                                            .shadow_lg()
                                            .py_1()
                                            .child(
                                                div()
                                                    .id("docker-header-prune")
                                                    .h(px(30.))
                                                    .px_3()
                                                    .flex()
                                                    .items_center()
                                                    .gap_2()
                                                    .text_size(px(11.))
                                                    .text_color(rgb(palette.danger))
                                                    .cursor_pointer()
                                                    .hover(|this| this.bg(rgb(palette.hover)))
                                                    .child(
                                                        svg()
                                                            .size(px(14.))
                                                            .flex_none()
                                                            .path("icons/fe/delete.svg")
                                                            .text_color(rgb(palette.danger)),
                                                    )
                                                    .child(prune_label)
                                                    .on_click(cx.listener(|this, _, _, cx| {
                                                        this.remote_ops.close_docker_menus();
                                                        this.prune_docker_system(cx);
                                                    })),
                                            ),
                                    )
                                }),
                        )
                        .into_any_element(),
                )
            }
            NavItem::SyncBackupHistory => {
                let palette = self.theme_palette();
                Some(
                    header_svg_icon_button(
                        palette,
                        "sync-history-header-refresh",
                        "icons/fe/refresh.svg",
                        self.tr("resourceMonitor.refresh"),
                        true,
                        cx.listener(|this, _, _, cx| {
                            this.refresh_cloud_sync_history();
                            this.terminal.view.status = "cloud sync history refreshed".to_string();
                            cx.notify();
                        }),
                    )
                    .into_any_element(),
                )
            }
            _ => None,
        }
    }

    fn panel_stack_resize_handle(
        &self,
        side: PanelSide,
        above_id: String,
        below_id: String,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let above = above_id.clone();
        let below = below_id.clone();
        div()
            .id(SharedString::from(format!(
                "panel-stack-resize-{}-{}-{}",
                match side {
                    PanelSide::Left => "left",
                    PanelSide::Right => "right",
                },
                above_id,
                below_id
            )))
            .h(px(3.))
            .flex_none()
            .w_full()
            .bg(rgb(palette.border))
            .cursor_row_resize()
            .hover(|this| this.bg(rgb(0x58a6ff)))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                    let open = this.side_open_panel_ids(side);
                    let total_weight: f32 = open.iter().map(|id| this.panel_stack_weight(id)).sum();
                    let container_height = 480.0_f32.max(total_weight * 120.);
                    this.start_panel_stack_resize(
                        side,
                        above.clone(),
                        below.clone(),
                        event,
                        container_height,
                        cx,
                    );
                }),
            )
    }
}

fn header_svg_icon_button(
    palette: ThemePalette,
    id: impl Into<String>,
    icon_path: &'static str,
    tooltip: impl Into<String>,
    enabled: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let tooltip = tooltip.into();
    div()
        .id(SharedString::from(id.into()))
        .size(px(28.))
        .flex()
        .items_center()
        .justify_center()
        .rounded_md()
        .text_color(rgb(if enabled {
            palette.text_muted
        } else {
            palette.text_dimmed
        }))
        .when(enabled, |this| {
            this.cursor_pointer().hover(|this| {
                this.bg(rgb(palette.surface_elevated))
                    .text_color(rgb(palette.text))
            })
        })
        .when(!enabled, |this| this.opacity(0.45))
        .tooltip(move |_, cx| cx.new(|_| ChromeTooltip::new(tooltip.clone())).into())
        .child(
            svg()
                .size(px(16.))
                .flex_none()
                .path(icon_path)
                .text_color(rgb(if enabled {
                    palette.text_muted
                } else {
                    palette.text_dimmed
                })),
        )
        .on_click(move |event, window, cx| {
            if enabled {
                on_click(event, window, cx);
            }
        })
}
