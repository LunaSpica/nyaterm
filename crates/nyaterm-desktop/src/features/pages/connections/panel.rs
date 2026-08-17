use gpui::{Context, Entity, IntoElement, Render, Subscription, Window};

use crate::features::NyaTermApp;

pub(in crate::features) struct ConnectionPanel {
    app: Entity<NyaTermApp>,
    _app_subscription: Subscription,
}

impl ConnectionPanel {
    pub(in crate::features) fn new(app: Entity<NyaTermApp>, cx: &mut Context<Self>) -> Self {
        let app_subscription = cx.observe(&app, |_, _, cx| cx.notify());
        Self {
            app,
            _app_subscription: app_subscription,
        }
    }
}

impl Render for ConnectionPanel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.app
            .update(cx, |app, cx| app.connections_view(window, cx))
    }
}
