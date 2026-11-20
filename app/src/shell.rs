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

        /// Creates the persistent profile, routes its requests through `policy`, runs `page_script` in every frame
        /// of its pages (and of private ones), and passes the profile and the start options to the QML root object.
        /// False when a Sofa already running took `start_url` instead.
        #[namespace = "sofa"]
        #[cxx_name = "prepareEngine"]
        fn prepare_engine(
            engine: Pin<&mut QQmlApplicationEngine>,
            storage_name: &QString,
            policy: Box<RequestPolicy>,
            start_url: &QString,
            full_screen: bool,
            page_script: &QString,
        ) -> bool;

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

    /// What a request is for. The C++ side maps Qt's resource types onto these.
    #[namespace = "sofa"]
    #[derive(Debug)]
    enum ResourceKind {
        Page,
        Frame,
        Stylesheet,
        Script,
        Image,
        Font,
        Media,
        Object,
        Xhr,
        Ping,
        WebSocket,
        CspReport,
        Other,
    }

    extern "Rust" {
        /// The request rules, owned by the C++ interceptor.
        #[namespace = "sofa"]
        type RequestPolicy;

        /// Whether the request for `url` (the encoded address), made for the page `first_party` (encoded, empty
        /// when there is none), must not be sent.
        #[namespace = "sofa"]
        #[cxx_name = "shouldBlock"]
        fn should_block(self: &RequestPolicy, url: &[u8], first_party: &[u8], kind: ResourceKind) -> bool;
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

    fn should_block(&self, url: &[u8], first_party: &[u8], kind: ffi::ResourceKind) -> bool {
        // Qt hands over the encoded form, which is ASCII. Anything else is let through for the engine to refuse.
        let Ok(url) = std::str::from_utf8(url) else { return false };
        let first_party = std::str::from_utf8(first_party).unwrap_or_default();
        self.0.should_block(&requests::Request { url, first_party, kind: kind.into() })
    }
}

impl From<ffi::ResourceKind> for requests::ResourceKind {
    fn from(kind: ffi::ResourceKind) -> Self {
        use ffi::ResourceKind as K;
        match kind {
            K::Page => Self::Page,
            K::Frame => Self::Frame,
            K::Stylesheet => Self::Stylesheet,
            K::Script => Self::Script,
            K::Image => Self::Image,
            K::Font => Self::Font,
            K::Media => Self::Media,
            K::Object => Self::Object,
            K::Xhr => Self::Xhr,
            K::Ping => Self::Ping,
            K::WebSocket => Self::WebSocket,
            K::CspReport => Self::CspReport,
            // A cxx enum can hold any number: one the C++ side doesn't name counts as "anything else".
            _ => Self::Other,
        }
    }
}
