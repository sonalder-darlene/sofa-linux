//! Sofa for Linux: a TV web browser for KDE Plasma Bigscreen, driven by a remote.

mod core_object;
mod library_object;
mod options;
mod paths;
mod session_object;
mod shell;

use std::process::ExitCode;

use cxx_qt_lib::{QGuiApplication, QQmlApplicationEngine, QString, QUrl};
use sofa_core::address::{self, SearchEngine};
use sofa_core::requests;
use sofa_core::settings::{KeyFile, SETTINGS};

use options::{Mode, Options};

/// The reverse-DNS app ID: Wayland app ID, `.desktop` file name and data folder name.
pub const APP_ID: &str = env!("SOFA_APP_ID");
/// QtWebEngine's name for the profile that keeps cookies, cache and site data.
const PROFILE_NAME: &str = "default";
const MAIN_QML: &str = "qrc:/qt/qml/Sofa/qml/Main.qml";
const CHECK_QML: &str = "qrc:/qt/qml/Sofa/qml/Check.qml";

fn main() -> ExitCode {
    let options = match Options::parse(std::env::args().skip(1)) {
        Ok(options) => options,
        Err(message) => {
            eprintln!("sofa-tv-browser: {message}\n\n{}", options::USAGE);
            return ExitCode::from(2);
        }
    };
    match options.mode {
        Mode::Help => {
            println!("{}", options::USAGE);
            ExitCode::SUCCESS
        }
        Mode::Version => {
            println!("Sofa for Linux {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Mode::Browse | Mode::Check => run(&options),
    }
}

fn run(options: &Options) -> ExitCode {
    shell::ffi::init_web_engine();

    let mut app = QGuiApplication::new();
    let Some(mut app_ref) = app.as_mut() else {
        eprintln!("sofa-tv-browser: Qt could not start");
        return ExitCode::FAILURE;
    };
    // No organisation and the app ID as name: data goes to ~/.local/share/<app ID>, settings to ~/.config.
    app_ref.as_mut().set_organization_name(&QString::default());
    app_ref.as_mut().set_application_name(&QString::from(APP_ID));
    app_ref.as_mut().set_application_version(&QString::from(env!("CARGO_PKG_VERSION")));
    app_ref.as_mut().set_application_display_name(&QString::from("Sofa"));
    // Lets the compositor match the window to its .desktop file (icon, name, Bigscreen's task list).
    QGuiApplication::set_desktop_file_name(&QString::from(APP_ID));

    let mut engine = QQmlApplicationEngine::new();
    let Some(mut engine_ref) = engine.as_mut() else {
        eprintln!("sofa-tv-browser: the QML engine could not start");
        return ExitCode::FAILURE;
    };

    let page = match options.mode {
        Mode::Check => {
            report_versions();
            CHECK_QML
        }
        _ => {
            let start_url = options.start.as_deref().map(start_address).unwrap_or_default();
            let first = shell::ffi::prepare_engine(
                engine_ref.as_mut(),
                &QString::from(PROFILE_NAME),
                shell::RequestPolicy::new(request_policy()),
                &QString::from(start_url.as_str()),
                !options.windowed,
            );
            if !first {
                // The Sofa already running opens it.
                return ExitCode::SUCCESS;
            }
            MAIN_QML
        }
    };
    engine_ref.as_mut().load(&QUrl::from(page));
    if !shell::ffi::has_window(&engine_ref) {
        eprintln!("sofa-tv-browser: the window could not be loaded (see the QML errors above)");
        return ExitCode::FAILURE;
    }

    let code = app_ref.exec();
    // Qt's exit codes are small; anything outside 0–255 is a failure all the same.
    u8::try_from(code).map_or(ExitCode::FAILURE, ExitCode::from)
}

/// The address to open at start for a command-line argument, as if it had been typed. Local files (`file:` URLs,
/// as a file manager passes them) open as they are.
fn start_address(arg: &str) -> String {
    if arg.get(..5).is_some_and(|s| s.eq_ignore_ascii_case("file:")) {
        return arg.to_owned();
    }
    address::resolve(arg, chosen_engine()).unwrap_or_default()
}

/// The search engine chosen in Settings, or the default until one is.
fn chosen_engine() -> &'static SearchEngine {
    let settings = KeyFile::load(paths::config_dir().map(|d| d.join("settings.conf")), SETTINGS);
    address::engine(settings.get("search_engine")).unwrap_or(&SearchEngine::DUCKDUCKGO)
}

/// The request rules. Only a development aid exists so far: `SOFA_BLOCK_HOSTS`, a comma-separated host list,
/// to see the interceptor at work. Ad blocking and site filters come later through the same policy.
fn request_policy() -> requests::RequestPolicy {
    match std::env::var("SOFA_BLOCK_HOSTS") {
        Ok(list) => requests::RequestPolicy::with_blocked_hosts(list.split(',')),
        Err(_) => requests::RequestPolicy::default(),
    }
}

/// The first lines of `--check`: what this build runs on.
fn report_versions() {
    println!("Sofa for Linux {} ({APP_ID})", env!("CARGO_PKG_VERSION"));
    println!("Engine: {}", shell::ffi::engine_versions());
    println!("Qt platform: {}", shell::ffi::platform_name());
    let session = std::env::var("XDG_SESSION_TYPE").unwrap_or_else(|_| "unknown".to_owned());
    println!("Session: {session}");
    println!("Engine sandbox: {}", sandbox_state());
    println!("Flatpak: {}", if std::path::Path::new("/.flatpak-info").exists() { "yes" } else { "no" });
}

/// Whether Chromium's sandbox was switched off through the environment.
fn sandbox_state() -> &'static str {
    let disabled_by_env = std::env::var_os("QTWEBENGINE_DISABLE_SANDBOX").is_some_and(|v| v != "0");
    let disabled_by_flag = std::env::var("QTWEBENGINE_CHROMIUM_FLAGS").is_ok_and(|f| f.contains("--no-sandbox"));
    if disabled_by_env || disabled_by_flag { "OFF (switched off in the environment)" } else { "on" }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_argument_is_read_as_typed_input() {
        assert_eq!(start_address("example.com"), "https://example.com");
        assert_eq!(start_address("https://example.com/x"), "https://example.com/x");
        assert_eq!(start_address("FILE:///tmp/page.html"), "FILE:///tmp/page.html");
        assert_eq!(start_address("  "), "");
    }
}
