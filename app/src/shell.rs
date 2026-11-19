//! The bridge to the thin C++ layer (`cpp/sofa/shell.*`): engine setup in C++, request decisions in Rust.

use sofa_core::requests;

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        /// Qt's string.
        type QString = cxx_qt_lib::QString;

        include!("cxx-qt-lib/qqmlapplicationengine.h");
        /// Qt's QML engine.
        type QQmlApplicationEngine = cxx_qt_lib::QQmlApplicationEngine;

        include!("sofa/shell.h");

        /// Sets up QtWebEngine. Must run before the application object exists.
        #[namespace = "sofa"]
        #[cxx_name = "initWebEngine"]
        fn init_web_engine();

        /// Creates the persistent profile, routes its requests through `policy`, and passes the profile and the
        /// start options to the QML root object.
        #[namespace = "sofa"]
        #[cxx_name = "prepareEngine"]
        fn prepare_engine(
            engine: Pin<&mut QQmlApplicationEngine>,
            storage_name: &QString,
            policy: Box<RequestPolicy>,
            start_url: &QString,
            full_screen: bool,
        );

        /// Whether loading the QML gave a root object (a window).
        #[namespace = "sofa"]
        #[cxx_name = "hasWindow"]
        fn has_window(engine: &QQmlApplicationEngine) -> bool;

        /// Qt, QtWebEngine and Chromium versions, for `--check`.
        #[namespace = "sofa"]
        #[cxx_name = "engineVersions"]
        fn engine_versions() -> QString;

        /// The Qt platform plugin in use.
        #[namespace = "sofa"]
        #[cxx_name = "platformName"]
        fn platform_name() -> QString;
    }

    extern "Rust" {
        /// The request rules, owned by the C++ interceptor.
        #[namespace = "sofa"]
        type RequestPolicy;

        /// Whether the request for `url` (the encoded address) must not be sent.
        #[namespace = "sofa"]
        #[cxx_name = "shouldBlock"]
        fn should_block(self: &RequestPolicy, url: &[u8]) -> bool;
    }
}

/// [`requests::RequestPolicy`] as the C++ side sees it.
#[derive(Debug)]
pub struct RequestPolicy(requests::RequestPolicy);

impl RequestPolicy {
    /// Wraps the core policy for the interceptor.
    pub fn new(policy: requests::RequestPolicy) -> Box<Self> {
        Box::new(Self(policy))
    }

    fn should_block(&self, url: &[u8]) -> bool {
        // Qt hands over the encoded form, which is ASCII. Anything else is let through for the engine to refuse.
        std::str::from_utf8(url).is_ok_and(|url| self.0.should_block(url))
    }
}
