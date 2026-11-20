//! Builds the QML module `Sofa`, the Rust ↔ C++ bridges and the thin C++ layer, and links Qt.

use cxx_qt_build::{CxxQtBuilder, QmlModule};

fn main() {
    // The app ID lives in one file at the top of the repository (see README, "App ID").
    println!("cargo::rerun-if-changed=../APP_ID");
    println!("cargo::rerun-if-env-changed=SOFA_APP_ID");
    let app_id = std::env::var("SOFA_APP_ID")
        .ok()
        .or_else(|| std::fs::read_to_string("../APP_ID").ok())
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
        .expect("the app ID: set SOFA_APP_ID or keep the APP_ID file at the top of the repository");
    println!("cargo::rustc-env=SOFA_APP_ID={app_id}");

    let qml = [
        "qml/Main.qml",
        "qml/Check.qml",
        "qml/BookmarksScreen.qml",
        "qml/BrowseMenu.qml",
        "qml/CardRow.qml",
        "qml/ChoiceDialog.qml",
        "qml/HistoryScreen.qml",
        "qml/IconButton.qml",
        "qml/ListRow.qml",
        "qml/Pointer.qml",
        "qml/Pressable.qml",
        "qml/SearchScreen.qml",
        "qml/SettingsScreen.qml",
        "qml/SetupScreen.qml",
        "qml/SiteCard.qml",
        "qml/SiteIcon.qml",
        "qml/StartPage.qml",
        "qml/TextButton.qml",
        "qml/TextDialog.qml",
        "qml/words.js",
    ];
    CxxQtBuilder::new_qml_module(QmlModule::new("Sofa").qml_files(qml).depend("QtQuick"))
        .files([
            "src/core_object.rs",
            "src/shell.rs",
            "src/library_object.rs",
            "src/session_object.rs",
            "src/popup_object.rs",
        ])
        .cpp_file("cpp/sofa/host.h")
        .cpp_file("cpp/sofa/host.cpp")
        .qrc("resources.qrc")
        .include_dir("cpp")
        .cpp_file("cpp/sofa/shell.cpp")
        .qt_module("Quick")
        .qt_module("WebEngineCore")
        .qt_module("WebEngineQuick")
        .qt_module("DBus")
        .build();
}
