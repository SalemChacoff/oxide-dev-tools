//! `notifications` — the shell's notification policy (README Phase 7.4).
//!
//! Notifications are for asynchronous outcomes the user is not already
//! looking at: exports, regenerations, and long runs (tool pages use this
//! from Phase 7.5 on). Never announce what the screen already shows — a
//! visible result is its own feedback. Copy follows the interface lexicon;
//! no "successfully" filler, and `Clear history`-style confirmations stay
//! dialogs, not toasts.

use gpui_kit::component::WindowExt as _;
use gpui_kit::component::notification::Notification;
use gpui_kit::{App, SharedString, Window};

/// Pushes an in-app, auto-hiding notification.
pub fn push(window: &mut Window, cx: &mut App, message: impl Into<SharedString>) {
    window.push_notification(Notification::info(message).autohide(true), cx);
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::component::Root;
    use gpui_kit::{
        AppContext as _, Context, IntoElement, Render, Styled as _, TestAppContext, VisualTestContext, Window, div,
    };

    struct NotificationHost;

    impl Render for NotificationHost {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().size_full()
        }
    }

    #[gpui_kit::test]
    fn push_adds_an_auto_hiding_notification(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        // Notifications live on the gpui-kit `Root`; the host view sits under
        // it exactly as it does at bootstrap.
        let (_view, cx) = cx.add_window_view(|window, cx| {
            let host = cx.new(|_| NotificationHost);
            Root::new(host, window, cx)
        });
        let cx: &mut VisualTestContext = cx;

        cx.update(|window, cx| {
            assert_eq!(window.notifications(cx).len(), 0);
            push(window, cx, "Done");
            assert_eq!(window.notifications(cx).len(), 1);
        });
    }
}
