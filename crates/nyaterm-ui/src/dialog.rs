use gpui::{App, ClickEvent, IntoElement, ParentElement as _, SharedString, Window, px};
use gpui_component::{
    WindowExt as _,
    button::ButtonVariant,
    dialog::{Dialog, DialogButtonProps},
};

pub struct NyaDialog {
    inner: Dialog,
}

impl NyaDialog {
    fn from_component(inner: Dialog) -> Self {
        Self { inner }
    }

    pub fn title(mut self, title: impl IntoElement) -> Self {
        self.inner = self.inner.title(title);
        self
    }

    pub fn content(mut self, content: impl IntoElement) -> Self {
        self.inner = self.inner.child(content);
        self
    }

    pub fn width(mut self, width: f32) -> Self {
        self.inner = self.inner.width(px(width));
        self
    }

    pub fn max_width(mut self, max_width: f32) -> Self {
        self.inner = self.inner.max_w(px(max_width));
        self
    }

    pub fn overlay_closable(mut self, overlay_closable: bool) -> Self {
        self.inner = self.inner.overlay_closable(overlay_closable);
        self
    }

    pub fn close_button(mut self, close_button: bool) -> Self {
        self.inner = self.inner.close_button(close_button);
        self
    }

    pub fn keyboard(mut self, keyboard: bool) -> Self {
        self.inner = self.inner.keyboard(keyboard);
        self
    }

    pub fn confirm(mut self, footer: NyaDialogFooter) -> Self {
        self.inner = self.inner.confirm().button_props(footer.into_component());
        self
    }

    pub fn alert(mut self, action_label: impl Into<SharedString>) -> Self {
        self.inner = self
            .inner
            .alert()
            .button_props(DialogButtonProps::default().ok_text(action_label));
        self
    }

    pub fn on_ok(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) -> bool + 'static,
    ) -> Self {
        self.inner = self.inner.on_ok(handler);
        self
    }

    pub fn on_cancel(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) -> bool + 'static,
    ) -> Self {
        self.inner = self.inner.on_cancel(handler);
        self
    }

    pub fn on_close(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.inner = self.inner.on_close(handler);
        self
    }

    fn into_component(self) -> Dialog {
        self.inner
    }
}

pub struct NyaConfirmDialog {
    dialog: NyaDialog,
}

impl NyaConfirmDialog {
    pub fn new(dialog: NyaDialog, footer: NyaDialogFooter) -> Self {
        Self {
            dialog: dialog.confirm(footer),
        }
    }

    pub fn content(mut self, content: impl IntoElement) -> Self {
        self.dialog = self.dialog.content(content);
        self
    }

    pub fn on_confirm(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) -> bool + 'static,
    ) -> Self {
        self.dialog = self.dialog.on_ok(handler);
        self
    }

    pub fn on_cancel(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) -> bool + 'static,
    ) -> Self {
        self.dialog = self.dialog.on_cancel(handler);
        self
    }

    pub fn into_dialog(self) -> NyaDialog {
        self.dialog
    }
}

pub struct NyaDialogFooter {
    cancel_label: SharedString,
    action_label: SharedString,
    danger: bool,
}

impl NyaDialogFooter {
    pub fn new(
        cancel_label: impl Into<SharedString>,
        action_label: impl Into<SharedString>,
    ) -> Self {
        Self {
            cancel_label: cancel_label.into(),
            action_label: action_label.into(),
            danger: false,
        }
    }

    pub fn danger(mut self) -> Self {
        self.danger = true;
        self
    }

    fn into_component(self) -> DialogButtonProps {
        DialogButtonProps::default()
            .cancel_text(self.cancel_label)
            .ok_text(self.action_label)
            .ok_variant(if self.danger {
                ButtonVariant::Danger
            } else {
                ButtonVariant::Primary
            })
    }
}

pub trait NyaDialogWindowExt {
    fn open_nya_dialog<F>(&mut self, cx: &mut App, build: F)
    where
        F: Fn(NyaDialog, &mut Window, &mut App) -> NyaDialog + 'static;

    fn has_active_nya_dialog(&mut self, cx: &mut App) -> bool;
    fn close_nya_dialog(&mut self, cx: &mut App);
    fn close_all_nya_dialogs(&mut self, cx: &mut App);
}

impl NyaDialogWindowExt for Window {
    fn open_nya_dialog<F>(&mut self, cx: &mut App, build: F)
    where
        F: Fn(NyaDialog, &mut Window, &mut App) -> NyaDialog + 'static,
    {
        self.open_dialog(cx, move |dialog, window, cx| {
            build(NyaDialog::from_component(dialog), window, cx).into_component()
        });
    }

    fn has_active_nya_dialog(&mut self, cx: &mut App) -> bool {
        self.has_active_dialog(cx)
    }

    fn close_nya_dialog(&mut self, cx: &mut App) {
        self.close_dialog(cx);
    }

    fn close_all_nya_dialogs(&mut self, cx: &mut App) {
        self.close_all_dialogs(cx);
    }
}
