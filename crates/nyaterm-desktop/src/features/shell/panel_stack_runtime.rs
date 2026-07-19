use super::*;

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
        self.panel_multi_open = !self.panel_multi_open;
        if self.panel_multi_open {
            if self.left_open_panels.is_empty() {
                if let Some(panel) = self.active_left_panel {
                    let id = panel.persistence_id().to_string();
                    if Self::is_stackable_panel_id(&id) {
                        self.left_open_panels.push(id);
                    }
                }
            }
            if self.right_open_panels.is_empty() {
                if let Some(panel) = self.active_right_panel {
                    let id = panel.persistence_id().to_string();
                    if Self::is_stackable_panel_id(&id) {
                        self.right_open_panels.push(id);
                    }
                }
            }
            self.terminal_status = "multi-open panels enabled".to_string();
        } else {
            // Collapse to active-only mode.
            if self.active_left_panel.is_none() {
                self.active_left_panel = self
                    .left_open_panels
                    .first()
                    .and_then(|id| NavItem::from_persistence_id(id));
            }
            if self.active_right_panel.is_none() {
                self.active_right_panel = self
                    .right_open_panels
                    .first()
                    .and_then(|id| NavItem::from_persistence_id(id));
            }
            self.left_open_panels.clear();
            self.right_open_panels.clear();
            self.terminal_status = "single panel mode".to_string();
        }
        self.persist_ui_layout();
        cx.notify();
    }

    pub(in crate::features) fn side_open_panel_ids(&self, side: PanelSide) -> Vec<String> {
        if !self.panel_multi_open {
            let active = match side {
                PanelSide::Left => self.active_left_panel,
                PanelSide::Right => self.active_right_panel,
            };
            return active
                .map(|item| item.persistence_id().to_string())
                .into_iter()
                .collect();
        }

        let open = match side {
            PanelSide::Left => &self.left_open_panels,
            PanelSide::Right => &self.right_open_panels,
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
            for id in self.activity_bar_layout.zone(zone) {
                if open_set.contains(id) && Self::is_stackable_panel_id(id) {
                    ordered.push(id.clone());
                }
            }
        }
        ordered
    }

    pub(in crate::features) fn side_overlay_panel(&self, side: PanelSide) -> Option<NavItem> {
        if !self.panel_multi_open {
            return None;
        }
        let active = match side {
            PanelSide::Left => self.active_left_panel,
            PanelSide::Right => self.active_right_panel,
        }?;
        let id = active.persistence_id();
        Self::is_exclusive_panel_id(id).then_some(active)
    }

    pub(in crate::features) fn panel_stack_weight(&self, panel_id: &str) -> f32 {
        self.panel_stack_sizes
            .get(panel_id)
            .copied()
            .filter(|value| value.is_finite() && *value > 0.)
            .unwrap_or(1.)
    }

    pub(in crate::features) fn panel_side_for_item(&self, item: NavItem) -> Option<PanelSide> {
        self.activity_bar_layout
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
        if !self.panel_multi_open {
            self.open_panel(item, cx);
            return;
        }

        let id = item.persistence_id().to_string();
        let Some(side) = self.panel_side_for_item(item) else {
            self.open_panel(item, cx);
            return;
        };

        self.main_mode = MainMode::Workspace;
        self.selected_nav = item;
        if item == NavItem::Recording {
            self.right_focus = RightFocus::Recording;
        } else {
            self.right_focus = RightFocus::Default;
        }

        if Self::is_exclusive_panel_id(&id) {
            let active = match side {
                PanelSide::Left => self.active_left_panel,
                PanelSide::Right => self.active_right_panel,
            };
            if active == Some(item) {
                // Dismiss exclusive overlay to stack.
                let fallback = self
                    .side_open_panel_ids(side)
                    .into_iter()
                    .find_map(|open_id| NavItem::from_persistence_id(&open_id));
                match side {
                    PanelSide::Left => {
                        self.active_left_panel = fallback;
                        self.left_sidebar_collapsed = fallback.is_none();
                    }
                    PanelSide::Right => {
                        self.active_right_panel = fallback;
                        self.right_inspector_collapsed = fallback.is_none();
                    }
                }
                self.terminal_status = format!("{} closed", item.label());
            } else {
                match side {
                    PanelSide::Left => {
                        self.active_left_panel = Some(item);
                        self.left_sidebar_collapsed = false;
                    }
                    PanelSide::Right => {
                        self.active_right_panel = Some(item);
                        self.right_inspector_collapsed = false;
                    }
                }
                self.terminal_status = format!("{} opened", item.label());
            }
            self.persist_ui_layout();
            cx.notify();
            return;
        }

        let open_list = match side {
            PanelSide::Left => &mut self.left_open_panels,
            PanelSide::Right => &mut self.right_open_panels,
        };
        let is_open = open_list.iter().any(|value| value == &id);
        let active = match side {
            PanelSide::Left => self.active_left_panel,
            PanelSide::Right => self.active_right_panel,
        };

        // If exclusive overlay is showing and stacked panel already open, reveal stack.
        if is_open
            && active
                .map(|item| Self::is_exclusive_panel_id(item.persistence_id()))
                .unwrap_or(false)
        {
            match side {
                PanelSide::Left => {
                    self.active_left_panel = Some(item);
                    self.left_sidebar_collapsed = false;
                }
                PanelSide::Right => {
                    self.active_right_panel = Some(item);
                    self.right_inspector_collapsed = false;
                }
            }
            self.terminal_status = format!("{} focused", item.label());
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
                    self.active_left_panel = next_active;
                    self.left_sidebar_collapsed =
                        next_active.is_none() && self.left_open_panels.is_empty();
                }
                PanelSide::Right => {
                    self.active_right_panel = next_active;
                    self.right_inspector_collapsed =
                        next_active.is_none() && self.right_open_panels.is_empty();
                }
            }
            self.terminal_status = format!("{} closed", item.label());
        } else {
            open_list.push(id);
            match side {
                PanelSide::Left => {
                    self.active_left_panel = Some(item);
                    self.left_sidebar_collapsed = false;
                }
                PanelSide::Right => {
                    self.active_right_panel = Some(item);
                    self.right_inspector_collapsed = false;
                }
            }
            self.terminal_status = format!("{} opened", item.label());
        }
        self.persist_ui_layout();
        cx.notify();
    }

    pub(in crate::features) fn ensure_panel_in_stack(&mut self, item: NavItem) {
        self.main_mode = MainMode::Workspace;
        self.selected_nav = item;
        if !self.panel_multi_open {
            self.ensure_panel_open(item);
            return;
        }
        let id = item.persistence_id().to_string();
        match self.panel_side_for_item(item) {
            Some(PanelSide::Left) => {
                self.left_sidebar_collapsed = false;
                if Self::is_exclusive_panel_id(&id) {
                    self.active_left_panel = Some(item);
                } else {
                    if !self.left_open_panels.iter().any(|value| value == &id) {
                        self.left_open_panels.push(id);
                    }
                    self.active_left_panel = Some(item);
                }
            }
            Some(PanelSide::Right) => {
                self.right_inspector_collapsed = false;
                self.right_focus = if item == NavItem::Recording {
                    RightFocus::Recording
                } else {
                    RightFocus::Default
                };
                if Self::is_exclusive_panel_id(&id) {
                    self.active_right_panel = Some(item);
                } else {
                    if !self.right_open_panels.iter().any(|value| value == &id) {
                        self.right_open_panels.push(id);
                    }
                    self.active_right_panel = Some(item);
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
        event: &gpui::MouseDownEvent,
        container_height: f32,
        cx: &mut Context<Self>,
    ) {
        self.panel_stack_resize = Some(PanelStackResizeState {
            side,
            above_id: above_id.clone(),
            below_id: below_id.clone(),
            start_y: event.position.y,
            above_weight: self.panel_stack_weight(&above_id),
            below_weight: self.panel_stack_weight(&below_id),
            container_height: container_height.max(1.),
        });
        self.terminal_status = "resizing panel stack".to_string();
        cx.notify();
    }

    pub(in crate::features) fn update_panel_stack_resize(
        &mut self,
        event: &gpui::MouseMoveEvent,
        cx: &mut Context<Self>,
    ) {
        let Some(state) = self.panel_stack_resize.clone() else {
            return;
        };
        let delta_px = f32::from(event.position.y - state.start_y);
        let pair = state.above_weight + state.below_weight;
        if pair <= 0. || state.container_height <= 0. {
            return;
        }
        let px_per_weight = state.container_height / pair;
        let min_weight = (48. / px_per_weight).min(pair / 2.).max(0.05);
        let next_above =
            (state.above_weight + delta_px / px_per_weight).clamp(min_weight, pair - min_weight);
        let next_below = pair - next_above;
        self.panel_stack_sizes
            .insert(state.above_id.clone(), next_above);
        self.panel_stack_sizes
            .insert(state.below_id.clone(), next_below);
        cx.notify();
    }

    pub(in crate::features) fn finish_panel_stack_resize(
        &mut self,
        _event: &gpui::MouseUpEvent,
        cx: &mut Context<Self>,
    ) {
        if self.panel_stack_resize.take().is_some() {
            self.persist_ui_layout();
            self.terminal_status = "panel stack sizes saved".to_string();
            cx.notify();
        }
    }

    pub(in crate::features) fn sync_panel_stack_to_settings(&mut self) {
        self.settings.ui_panel_multi_open = self.panel_multi_open;
        self.settings.ui_left_open_panels = self.left_open_panels.clone();
        self.settings.ui_right_open_panels = self.right_open_panels.clone();
        self.settings.ui_panel_stack_sizes = self
            .panel_stack_sizes
            .iter()
            .filter_map(|(key, value)| {
                let scaled = (*value * 1000.).round();
                (scaled.is_finite() && scaled > 0.).then(|| (key.clone(), scaled as u32))
            })
            .collect();
    }

    pub(in crate::features) fn apply_panel_stack_from_settings(&mut self) {
        self.panel_multi_open = self.settings.ui_panel_multi_open;
        self.left_open_panels = self.settings.ui_left_open_panels.clone();
        self.right_open_panels = self.settings.ui_right_open_panels.clone();
        self.panel_stack_sizes = self
            .settings
            .ui_panel_stack_sizes
            .iter()
            .filter_map(|(key, value)| (*value > 0).then(|| (key.clone(), (*value as f32) / 1000.)))
            .collect();
        if self.panel_multi_open {
            if self.left_open_panels.is_empty() {
                if let Some(panel) = self.active_left_panel {
                    let id = panel.persistence_id().to_string();
                    if Self::is_stackable_panel_id(&id) {
                        self.left_open_panels.push(id);
                    }
                }
            }
            if self.right_open_panels.is_empty() {
                if let Some(panel) = self.active_right_panel {
                    let id = panel.persistence_id().to_string();
                    if Self::is_stackable_panel_id(&id) {
                        self.right_open_panels.push(id);
                    }
                }
            }
        }
    }

    pub(in crate::features) fn side_panel_stack(
        &mut self,
        side: PanelSide,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        use gpui::relative;

        let open_ids = self.side_open_panel_ids(side);
        let mut stack = if open_ids.is_empty() {
            let fallback = self
                .activity_bar_layout
                .first_panel_on_side(side)
                .unwrap_or(NavItem::Workspace);
            self.single_side_panel(side, fallback, cx)
        } else if open_ids.len() == 1 || !self.panel_multi_open {
            let panel = open_ids
                .first()
                .and_then(|id| NavItem::from_persistence_id(id))
                .or_else(|| self.activity_bar_layout.first_panel_on_side(side))
                .unwrap_or(NavItem::Workspace);
            self.single_side_panel(side, panel, cx)
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
                    PanelSide::Left => self.left_panel_body(panel, cx),
                    PanelSide::Right => self.right_panel_body(panel, cx),
                };
                stack = stack.child(
                    div()
                        .flex_none()
                        .flex_basis(relative(basis))
                        .min_h(px(96.))
                        .flex()
                        .flex_col()
                        .overflow_hidden()
                        .child(panel_header_with_actions(title, meta, palette, actions))
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
                    self.single_side_panel(side, overlay, cx)
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
            PanelSide::Left => self.left_panel_body(panel, cx),
            PanelSide::Right => self.right_panel_body(panel, cx),
        };
        div()
            .size_full()
            .flex()
            .flex_col()
            .child(panel_header_with_actions(title, meta, palette, actions))
            .child(div().flex_1().min_h_0().overflow_hidden().child(body))
    }

    /// Tauri PanelHeader meta/actions: Connections shows total count; AI shows model name.
    fn side_panel_meta(&self, _side: PanelSide, panel: NavItem) -> SharedString {
        match panel {
            NavItem::Connections => SharedString::from(""),
            NavItem::AiAssistant => {
                let label = if !self.ai_model_draft.trim().is_empty() {
                    truncate_preview(self.ai_model_draft.trim(), 28)
                } else if let Some(id) = self
                    .ai_settings
                    .default_model_id
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                {
                    truncate_preview(id, 28)
                } else if !self.ai_settings.enabled {
                    "disabled".to_string()
                } else {
                    "not configured".to_string()
                };
                SharedString::from(label)
            }
            NavItem::ActiveSessions => {
                let count = self.ordered_session_count();
                SharedString::from(count.to_string())
            }
            // Tauri NetworkPanel header shows active tab profile count.
            NavItem::Tunnels => {
                let count = match self.network_tab {
                    NetworkTab::Tunnels => self.tunnels.len(),
                    NetworkTab::Proxies => self.proxies.len(),
                };
                SharedString::from(count.to_string())
            }
            NavItem::Transfers => {
                if self.transfer_browser_entries.is_empty() {
                    SharedString::from("")
                } else {
                    SharedString::from(self.transfer_browser_entries.len().to_string())
                }
            }
            NavItem::Processes => {
                if self.processes.is_empty() {
                    SharedString::from("")
                } else {
                    SharedString::from(self.processes.len().to_string())
                }
            }
            NavItem::Docker => {
                let Some(overview) = self
                    .docker_overview
                    .as_ref()
                    .filter(|overview| overview.available)
                else {
                    return SharedString::from("");
                };
                let version = if overview.version.trim().is_empty() {
                    "-"
                } else {
                    overview.version.trim()
                };
                SharedString::from(format!("Engine {}", truncate_preview(version, 24)))
            }
            // Tauri SecurityAuthPanel header actions show active-tab count.
            NavItem::SecurityAuth => {
                let count = match self.security_auth_tab {
                    SecurityAuthTab::Keys => self.connection_ssh_keys.len(),
                    SecurityAuthTab::Passwords => self.connection_saved_passwords.len(),
                    SecurityAuthTab::Credentials => self.connection_saved_credentials.len(),
                    SecurityAuthTab::Otp => self.connection_otp_entries.len(),
                };
                if count == 0 {
                    SharedString::from("")
                } else {
                    SharedString::from(count.to_string())
                }
            }
            NavItem::Recording => {
                // Badge reflects open session panes (local metadata), not transport lock.
                let count = self.ordered_session_count();
                SharedString::from(count.to_string())
            }
            NavItem::SyncBackupHistory => {
                let count = self.cloud_sync_history.len();
                if count == 0 {
                    SharedString::from("")
                } else {
                    SharedString::from(count.to_string())
                }
            }
            _ => SharedString::from(""),
        }
    }

    fn side_panel_header_actions(
        &mut self,
        panel: NavItem,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        match panel {
            NavItem::Connections if !self.connections.is_empty() => Some(
                div()
                    .text_size(px(11.))
                    .text_color(rgb(self.theme_palette().text_dimmed))
                    .child(self.connections.len().to_string())
                    .into_any_element(),
            ),
            NavItem::AiAssistant => {
                let palette = self.theme_palette();
                let ai_running = self.ai_chat_pending || self.ai_agent_loop.is_some();
                Some(
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .child(header_svg_icon_button(
                            palette,
                            "ai-header-execution-mode-toggle",
                            match self.ai_settings.agent_command_execution_mode {
                                AgentCommandExecutionMode::Auto => "icons/ai/exec-auto.svg",
                                AgentCommandExecutionMode::Smart => "icons/ai/exec-smart.svg",
                                AgentCommandExecutionMode::ConfirmEach => {
                                    "icons/ai/exec-confirm.svg"
                                }
                            },
                            self.tr("ai.agentCommandExecutionMode"),
                            !ai_running,
                            cx.listener(|this, _, _, cx| {
                                this.ai_history_open = false;
                                this.ai_history_query.clear();
                                this.ai_execution_menu_open = !this.ai_execution_menu_open;
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
                                this.ai_execution_menu_open = false;
                                this.ai_history_open = !this.ai_history_open;
                                if this.ai_history_open {
                                    this.refresh_ai_session_list(cx);
                                    window.focus(&this.ai_history_search_focus);
                                } else {
                                    this.ai_history_query.clear();
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
                                this.ai_history_open = false;
                                this.ai_execution_menu_open = false;
                                this.settings_active_tab = SettingsTab::AiGeneral;
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
                let can_refresh = self.active_ssh_config.is_some() && !self.stats_pending;
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
                let can_refresh = self.active_ssh_config.is_some() && !self.process_pending;
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
                let can_refresh = self.active_ssh_config.is_some() && !self.docker_pending;
                let can_prune = can_refresh
                    && self
                        .docker_overview
                        .as_ref()
                        .is_some_and(|overview| overview.available);
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
                        .child(header_svg_icon_button(
                            palette,
                            "docker-header-prune",
                            "icons/fe/delete.svg",
                            self.tr("dockerManager.prune"),
                            can_prune,
                            cx.listener(|this, _, _, cx| {
                                this.prune_docker_system(cx);
                            }),
                        ))
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
                cx.listener(move |this, event: &gpui::MouseDownEvent, _, cx| {
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
        .child(svg().size(px(16.)).flex_none().path(icon_path))
        .on_click(move |event, window, cx| {
            if enabled {
                on_click(event, window, cx);
            }
        })
}
