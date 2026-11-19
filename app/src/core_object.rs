//! `Core`, the QML singleton through which the QML screens reach the Rust core.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        /// Qt's string.
        type QString = cxx_qt_lib::QString;
    }

    extern "RustQt" {
        /// The Rust core, as QML sees it.
        #[qobject]
        #[qml_element]
        #[qml_singleton]
        #[qproperty(QString, version)]
        type Core = super::CoreRust;

        /// The address to load for what the user typed, or an empty string when there is nothing to load.
        #[qinvokable]
        #[cxx_name = "resolveAddress"]
        fn resolve_address(&self, text: &QString) -> QString;

        /// A short form of `url` for the address field when it isn't being edited.
        #[qinvokable]
        #[cxx_name = "displayAddress"]
        fn display_address(&self, url: &QString) -> QString;

        /// Prints one line of the `--check` report on standard output.
        #[qinvokable]
        fn report(&self, line: &QString);
    }
}

use cxx_qt_lib::QString;
use sofa_core::address::{self, SearchEngine};

/// State behind [`qobject::Core`].
#[derive(Debug)]
pub struct CoreRust {
    version: QString,
    search_engine: SearchEngine,
}

impl Default for CoreRust {
    fn default() -> Self {
        Self { version: QString::from(env!("CARGO_PKG_VERSION")), search_engine: SearchEngine::default() }
    }
}

impl qobject::Core {
    fn resolve_address(&self, text: &QString) -> QString {
        address::resolve(&String::from(text), &self.search_engine)
            .map(|url| QString::from(url.as_str()))
            .unwrap_or_default()
    }

    fn display_address(&self, url: &QString) -> QString {
        QString::from(address::display(&String::from(url)))
    }

    fn report(&self, line: &QString) {
        println!("{line}");
    }
}
