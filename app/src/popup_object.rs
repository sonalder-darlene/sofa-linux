//! `PopupGuard`, the pop-up and redirect rules of one tab ([`sofa_core::popups`]) as QML sees them. Each page view
//! has one; the view hands it its presses, its windows and its navigations, and acts on the answers.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        /// Qt's string.
        type QString = cxx_qt_lib::QString;
    }

    extern "RustQt" {
        /// One tab's pop-up and redirect rules.
        #[qobject]
        #[qml_element]
        type PopupGuard = super::PopupGuardRust;

        /// A press reached the page (the host's `pressed`).
        #[qinvokable]
        fn press(self: Pin<&mut Self>);
        /// A console message from the page: true when it was the press script saying what a press landed on.
        #[qinvokable]
        fn report(self: Pin<&mut Self>, message: &QString) -> bool;
        /// The link under the latest press, or "".
        #[qinvokable]
        fn link(&self) -> QString;
        /// The page asked for a window, on a press if `gesture`; false when it must not open at all.
        #[qinvokable]
        #[cxx_name = "windowRequested"]
        fn window_requested(self: Pin<&mut Self>, gesture: bool) -> bool;
        /// Whether the window to `url` that the page at `opener` asked for (with `link` under the press then, or "")
        /// may open.
        #[qinvokable]
        fn window(self: Pin<&mut Self>, url: &QString, opener: &QString, link: &QString) -> bool;
        /// Sofa itself loads an address in the tab: it and its redirects are the user's.
        #[qinvokable]
        #[cxx_name = "userNavigation"]
        fn user_navigation(self: Pin<&mut Self>);
        /// Whether a main-frame navigation the page started (or a redirect hop of one) from `page` to `url` may go
        /// on. `gesture`: 1 a press started it, 0 none did, -1 the engine doesn't say.
        #[qinvokable]
        fn navigation(self: Pin<&mut Self>, url: &QString, page: &QString, gesture: i32, redirect: bool) -> bool;
        /// `url` was just blocked: "Allow once" offers it for a while.
        #[qinvokable]
        fn blocked(self: Pin<&mut Self>, url: &QString);
        /// The address "Allow once" would open now, or "".
        #[qinvokable]
        fn pending(&self) -> QString;
        /// Like `pending`, and forgets it.
        #[qinvokable]
        #[cxx_name = "takePending"]
        fn take_pending(self: Pin<&mut Self>) -> QString;
        /// A new page started loading in the tab.
        #[qinvokable]
        #[cxx_name = "pageStarted"]
        fn page_started(self: Pin<&mut Self>);
    }
}

use std::fmt;
use std::pin::Pin;
use std::sync::OnceLock;
use std::time::Instant;

use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use sofa_core::popups::{Navigation, PopupGuard, Verdict, Window};

/// State behind [`qobject::PopupGuard`].
#[derive(Debug)]
pub struct PopupGuardRust {
    guard: PopupGuard,
    started: Instant,
}

impl Default for PopupGuardRust {
    fn default() -> Self {
        Self { guard: PopupGuard::new(), started: Instant::now() }
    }
}

impl PopupGuardRust {
    /// Milliseconds since this guard was made.
    fn now(&self) -> u64 {
        u64::try_from(self.started.elapsed().as_millis()).unwrap_or(u64::MAX)
    }
}

/// With `SOFA_DEBUG_POPUPS=1`, every decision goes to the error output, for sites that misbehave.
fn debug(message: fmt::Arguments<'_>) {
    static ON: OnceLock<bool> = OnceLock::new();
    if *ON.get_or_init(|| std::env::var_os("SOFA_DEBUG_POPUPS").is_some_and(|v| v == "1")) {
        eprintln!("sofa-popups: {message}");
    }
}

fn some(text: &str) -> Option<&str> {
    (!text.is_empty()).then_some(text)
}

impl qobject::PopupGuard {
    fn press(self: Pin<&mut Self>) {
        let mut this = self.rust_mut();
        debug(format_args!("press"));
        let now = this.now();
        this.guard.press(now);
    }

    fn report(self: Pin<&mut Self>, message: &QString) -> bool {
        let message = String::from(message);
        let Some(link) = crate::press_tag().read(&message) else { return false };
        debug(format_args!("press on link={link:?}"));
        self.rust_mut().guard.pressed_link(link);
        true
    }

    fn link(&self) -> QString {
        QString::from(self.guard.link().unwrap_or_default())
    }

    fn window_requested(self: Pin<&mut Self>, gesture: bool) -> bool {
        debug(format_args!("window asked: gesture={gesture}"));
        self.rust_mut().guard.window_requested(gesture)
    }

    fn window(self: Pin<&mut Self>, url: &QString, opener: &QString, link: &QString) -> bool {
        let (url, opener, link) = (String::from(url), String::from(opener), String::from(link));
        let window = Window { url: &url, opener: &opener, link: some(&link) };
        let verdict = self.rust_mut().guard.window(&window, crate::ad_blocker().as_ref());
        debug(format_args!("window {verdict:?}: {url} (from {opener}, link={link:?})"));
        verdict == Verdict::Allow
    }

    fn user_navigation(self: Pin<&mut Self>) {
        debug(format_args!("Sofa's own load"));
        self.rust_mut().guard.user_navigation();
    }

    fn navigation(self: Pin<&mut Self>, url: &QString, page: &QString, gesture: i32, redirect: bool) -> bool {
        let (url, page) = (String::from(url), String::from(page));
        let gesture = match gesture {
            0 => Some(false),
            1 => Some(true),
            _ => None,
        };
        let mut this = self.rust_mut();
        let now = this.now();
        let nav = Navigation { url: &url, page: &page, gesture, redirect };
        let verdict = this.guard.navigation(&nav, now, crate::ad_blocker().as_ref());
        debug(format_args!("navigation {verdict:?}: {url} gesture={gesture:?} redirect={redirect}"));
        verdict == Verdict::Allow
    }

    fn blocked(self: Pin<&mut Self>, url: &QString) {
        let mut this = self.rust_mut();
        let now = this.now();
        this.guard.blocked(&String::from(url), now);
    }

    fn pending(&self) -> QString {
        QString::from(self.guard.pending(self.now()).unwrap_or_default())
    }

    fn take_pending(self: Pin<&mut Self>) -> QString {
        let mut this = self.rust_mut();
        let now = this.now();
        QString::from(this.guard.take_pending(now).unwrap_or_default().as_str())
    }

    fn page_started(self: Pin<&mut Self>) {
        self.rust_mut().guard.page_started();
    }
}
