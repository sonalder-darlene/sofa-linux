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

    CxxQtBuilder::new_qml_module(QmlModule::new("Sofa").qml_files(["qml/Main.qml", "qml/Check.qml"]).depend("QtQuick"))
        .files(["src/core_object.rs", "src/shell.rs"])
        .qrc("resources.qrc")
        .include_dir("cpp")
        .cpp_file("cpp/sofa/shell.cpp")
        .qt_module("Quick")
        .qt_module("WebEngineCore")
        .qt_module("WebEngineQuick")
        .build();
}
