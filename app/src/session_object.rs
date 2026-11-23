//! `Session`, the QML singleton for settings, the saved tab list, "Open at start" and the small rules the browser
//! window asks about tabs; and `TabOrder`, one side's tab order (normal or private), as QML sees them.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        /// Qt's string.
        type QString = cxx_qt_lib::QString;
    }

    extern "RustQt" {
        /// Settings and the session.
        #[qobject]
        #[qml_element]
        #[qml_singleton]
        type Session = super::SessionRust;

        /// A setting (`search_engine`, `open_at_start`, `start_page`, `adblock`, `history_days`); "" for an unknown key.
        #[qinvokable]
        fn value(&self, key: &QString) -> QString;
        /// Sets a setting after checking it; false when the value doesn't fit.
        #[qinvokable]
        #[cxx_name = "setValue"]
        fn set_value(self: Pin<&mut Self>, key: &QString, value: &QString) -> bool;
        /// Whether the site of the page at `url` is switched off from ad blocking (Sofa's menu, "Ad block").
        #[qinvokable]
        #[cxx_name = "adBlockOffHere"]
        fn ad_block_off_here(&self, url: &QString) -> bool;
        /// Switches ad blocking off (`off`) or back on for the site of the page at `url`, until private browsing
        /// ends when `in_private`. False for a page without a site.
        #[qinvokable]
        #[cxx_name = "setAdBlockOffHere"]
        fn set_ad_block_off_here(&self, url: &QString, off: bool, in_private: bool) -> bool;
        /// Private browsing began or ended: the choices made in it are forgotten.
        #[qinvokable]
        #[cxx_name = "forgetPrivateChoices"]
        fn forget_private_choices(&self);
        /// `[{id, name, paid}]`, A to Z.
        #[qinvokable]
        fn engines(&self) -> QString;
        /// What the user typed, as an address to load with the chosen search engine; "" when there is nothing.
        #[qinvokable]
        #[cxx_name = "resolveAddress"]
        fn resolve_address(&self, text: &QString) -> QString;

        /// The tab list kept from the last run: `{addresses: [...], current}`.
        #[qinvokable]
        #[cxx_name = "savedTabs"]
        fn saved_tabs(&self) -> QString;
        /// Keeps the normal side's tabs (`addresses` a JSON array; "" for an empty tab) and the one in sight.
        #[qinvokable]
        #[cxx_name = "saveTabs"]
        fn save_tabs(self: Pin<&mut Self>, addresses: &QString, current: i32);
        /// What "Open at start" opens: an address, or "" for the start page.
        #[qinvokable]
        #[cxx_name = "startAddress"]
        fn start_address(&self) -> QString;
        /// Keeps what was in sight for "Where I left off" (`page` "" when the start page showed).
        #[qinvokable]
        #[cxx_name = "rememberLeftOff"]
        fn remember_left_off(self: Pin<&mut Self>, page: &QString, in_private: bool);
        /// A typed address as "A web page" keeps it, or "" when it isn't a web page.
        #[qinvokable]
        #[cxx_name = "webAddress"]
        fn web_address(&self, typed: &QString) -> QString;
        /// How Settings names a page.
        #[qinvokable]
        fn label(&self, url: &QString) -> QString;
        /// Whether two addresses are on the same site.
        #[qinvokable]
        #[cxx_name = "sameSite"]
        fn same_site(&self, a: &QString, b: &QString) -> bool;
        /// The strip's chip names for these tabs (`titles`, `addresses`: JSON arrays, "" for none).
        #[qinvokable]
        #[cxx_name = "tabNames"]
        fn tab_names(&self, titles: &QString, addresses: &QString, new_tab: &QString) -> QString;
        /// How many tabs out of sight keep their page loaded on this computer.
        #[qinvokable]
        #[cxx_name = "liveBackground"]
        fn live_background(&self) -> i32;
        /// Whether `url` is a web page Sofa keeps in history and tabs.
        #[qinvokable]
        #[cxx_name = "isWebUrl"]
        fn is_web_url(&self, url: &QString) -> bool;

        /// The web engine for About, as JSON: `{chromium, fixes, qt}` (Chromium major version it is built from, the
        /// major version its security fixes reach, Qt version).
        #[qinvokable]
        #[cxx_name = "engineInfo"]
        fn engine_info(&self) -> QString;
        /// Whether to tell the user now that the web engine is out of date (once per engine version, again after
        /// about three months). True also records it as told.
        #[qinvokable]
        #[cxx_name = "takeEngineNotice"]
        fn take_engine_notice(self: Pin<&mut Self>) -> bool;
        /// Whether Sofa runs as a Flatpak (the engine then comes with the Flatpak runtime).
        #[qinvokable]
        #[cxx_name = "isFlatpak"]
        fn is_flatpak(&self) -> bool;
    }

    extern "RustQt" {
        /// One side's tabs, by the ids QML gives them.
        #[qobject]
        #[qml_element]
        type TabOrder = super::TabOrderRust;

        /// Adds `id` after `after` (-1: at the end); false when full.
        #[qinvokable]
        fn add(self: Pin<&mut Self>, id: i32, after: i32) -> bool;
        #[qinvokable]
        fn select(self: Pin<&mut Self>, id: i32);
        /// Takes `id` out; the tab now in sight, or -1.
        #[qinvokable]
        fn remove(self: Pin<&mut Self>, id: i32) -> i32;
        /// The tab in sight, or -1.
        #[qinvokable]
        fn current(&self) -> i32;
        /// The ids in strip order, as a JSON array.
        #[qinvokable]
        fn order(&self) -> QString;
        #[qinvokable]
        #[cxx_name = "isFull"]
        fn is_full(&self) -> bool;
        #[qinvokable]
        fn count(&self) -> i32;
        /// The tab `steps` places from the one in sight in strip order, going round at the ends (Ctrl+Tab and
        /// Ctrl+Shift+Tab); -1 while there are none.
        #[qinvokable]
        fn neighbour(&self, steps: i32) -> i32;
        /// Of the loaded tabs out of sight (`live`: JSON array of ids), those to unload: past the `keep` most recent,
        /// or out of sight for ten minutes. A JSON array.
        #[qinvokable]
        #[cxx_name = "toUnload"]
        fn to_unload(&self, keep: i32, live: &QString) -> QString;
        /// Milliseconds until the next loaded tab out of sight has rested long enough, or -1.
        #[qinvokable]
        #[cxx_name = "nextRest"]
        fn next_rest(&self, live: &QString) -> i32;
    }
}

use std::pin::Pin;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use serde_json::{Value, json};
use sofa_core::address::{self, SearchEngine};
use sofa_core::settings::{KeyFile, SESSION, SETTINGS};
use sofa_core::start::{self, Choice};
use sofa_core::{engine_age, sites, tabs};

use crate::{paths, shell};

/// State behind [`qobject::Session`].
#[derive(Debug)]
pub struct SessionRust {
    settings: KeyFile,
    session: KeyFile,
}

impl Default for SessionRust {
    fn default() -> Self {
        Self {
            settings: KeyFile::load(paths::config_dir().map(|d| d.join("settings.conf")), SETTINGS),
            session: KeyFile::load(paths::state_dir().map(|d| d.join("session.conf")), SESSION),
        }
    }
}

fn text(value: &Value) -> QString {
    QString::from(value.to_string().as_str())
}

fn report(result: std::io::Result<bool>) -> bool {
    result.unwrap_or_else(|e| {
        eprintln!("sofa-tv-browser: a setting could not be written: {e}");
        true
    })
}

impl SessionRust {
    fn engine(&self) -> &'static SearchEngine {
        address::engine(self.settings.get("search_engine")).unwrap_or(&SearchEngine::DUCKDUCKGO)
    }
}

impl qobject::Session {
    fn value(&self, key: &QString) -> QString {
        QString::from(self.settings.get(&String::from(key)))
    }

    fn set_value(self: Pin<&mut Self>, key: &QString, value: &QString) -> bool {
        let key = String::from(key);
        let value = String::from(value);
        let mut this = self.rust_mut();
        let taken = report(this.settings.set(&key, &value));
        // "Where I left off" forgets its page when another choice is made.
        if taken && key == "adblock" {
            crate::ad_blocker().set_enabled(this.settings.get("adblock") == "true");
        }
        if taken && key == "open_at_start" && Choice::from_id(&value) != Choice::LastPage {
            report(this.session.set("last_page", ""));
        }
        taken
    }

    fn ad_block_off_here(&self, url: &QString) -> bool {
        crate::ad_blocker().allows_site(&String::from(url))
    }

    fn set_ad_block_off_here(&self, url: &QString, off: bool, in_private: bool) -> bool {
        crate::ad_blocker().allow_site(&String::from(url), off, in_private).unwrap_or_else(|e| {
            eprintln!("sofa-tv-browser: the sites without ad blocking could not be written: {e}");
            true
        })
    }

    fn forget_private_choices(&self) {
        crate::ad_blocker().forget_private();
    }

    fn engines(&self) -> QString {
        let list: Vec<Value> =
            address::ENGINES.iter().map(|e| json!({ "id": e.id, "name": e.name, "paid": e.paid })).collect();
        text(&Value::Array(list))
    }

    fn resolve_address(&self, text: &QString) -> QString {
        address::resolve(&String::from(text), self.engine()).map(|u| QString::from(u.as_str())).unwrap_or_default()
    }

    fn saved_tabs(&self) -> QString {
        let (addresses, current) = tabs::decode(self.session.get("tabs"), tabs::MAX_TABS);
        text(&json!({ "addresses": addresses, "current": current }))
    }

    fn save_tabs(self: Pin<&mut Self>, addresses: &QString, current: i32) {
        let addresses: Vec<String> = serde_json::from_str(&String::from(addresses)).unwrap_or_default();
        let encoded = tabs::encode(&addresses, usize::try_from(current).unwrap_or(0));
        report(self.rust_mut().session.set("tabs", &encoded));
    }

    fn start_address(&self) -> QString {
        let choice = Choice::from_id(self.settings.get("open_at_start"));
        let address = start::address(choice, self.settings.get("start_page"), self.session.get("last_page"));
        QString::from(address.unwrap_or_default().as_str())
    }

    fn remember_left_off(self: Pin<&mut Self>, page: &QString, in_private: bool) {
        let mut this = self.rust_mut();
        let choice = Choice::from_id(this.settings.get("open_at_start"));
        let page = String::from(page);
        let keep = start::to_remember(choice, (!page.is_empty()).then_some(page.as_str()), in_private);
        report(this.session.set("last_page", &keep));
    }

    fn web_address(&self, typed: &QString) -> QString {
        QString::from(start::web_address(&String::from(typed)).unwrap_or_default().as_str())
    }

    fn label(&self, url: &QString) -> QString {
        QString::from(start::label(&String::from(url)).as_str())
    }

    fn same_site(&self, a: &QString, b: &QString) -> bool {
        sites::same_site(&String::from(a), &String::from(b))
    }

    fn tab_names(&self, titles: &QString, addresses: &QString, new_tab: &QString) -> QString {
        let read = |s: &QString| -> Vec<Option<String>> {
            serde_json::from_str::<Vec<String>>(&String::from(s))
                .unwrap_or_default()
                .into_iter()
                .map(|v| (!v.is_empty()).then_some(v))
                .collect()
        };
        let (titles, addresses) = (read(titles), read(addresses));
        if titles.len() != addresses.len() {
            return QString::from("[]");
        }
        text(&json!(tabs::names(&titles, &addresses, &String::from(new_tab))))
    }

    fn live_background(&self) -> i32 {
        let total = std::fs::read_to_string("/proc/meminfo").ok().and_then(|m| {
            let line = m.lines().find(|l| l.starts_with("MemTotal:"))?;
            let kb: u64 = line.split_whitespace().nth(1)?.parse().ok()?;
            Some(kb * 1024)
        });
        // Unknown memory: keep none loaded, the light choice.
        i32::try_from(total.map_or(0, tabs::live_background)).unwrap_or(0)
    }

    fn is_web_url(&self, url: &QString) -> bool {
        sites::is_web_url(&String::from(url))
    }

    fn engine_info(&self) -> QString {
        let major = |v: QString| {
            let v = String::from(&v);
            engine_age::major(&v).map_or(v, |m| m.to_string())
        };
        text(&json!({
            "chromium": major(shell::ffi::chromium_version()),
            "fixes": major(shell::ffi::security_patch_version()),
            "qt": String::from(&shell::ffi::qt_version()),
        }))
    }

    fn take_engine_notice(self: Pin<&mut Self>) -> bool {
        let version = String::from(&shell::ffi::security_patch_version());
        let now = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs());
        let mut this = self.rust_mut();
        let warned_at = this.session.get("engine_warned_at").parse().unwrap_or(0);
        if !engine_age::should_warn(&version, this.session.get("engine_warned_version"), warned_at, now) {
            return false;
        }
        report(this.session.set("engine_warned_version", version.trim()));
        report(this.session.set("engine_warned_at", &now.to_string()));
        true
    }

    fn is_flatpak(&self) -> bool {
        paths::is_flatpak()
    }
}

/// State behind [`qobject::TabOrder`].
#[derive(Debug)]
pub struct TabOrderRust {
    list: tabs::TabList<i32>,
    started: Instant,
    rest_ms: i64,
}

impl Default for TabOrderRust {
    fn default() -> Self {
        // A development aid, as on Android: SOFA_TAB_REST_SECONDS shortens the ten minutes to see unloading at work.
        let rest_ms = std::env::var("SOFA_TAB_REST_SECONDS")
            .ok()
            .and_then(|s| s.parse::<i64>().ok())
            .filter(|s| *s > 0)
            .map_or(tabs::REST_MS, |s| s * 1000);
        Self { list: tabs::TabList::new(tabs::MAX_TABS), started: Instant::now(), rest_ms }
    }
}

impl TabOrderRust {
    /// Milliseconds since this list was made, on a clock that doesn't run while the computer sleeps.
    fn now(&self) -> i64 {
        i64::try_from(self.started.elapsed().as_millis()).unwrap_or(i64::MAX)
    }
}

fn ids(json: &QString) -> Vec<i32> {
    serde_json::from_str(&String::from(json)).unwrap_or_default()
}

impl qobject::TabOrder {
    fn add(self: Pin<&mut Self>, id: i32, after: i32) -> bool {
        let mut this = self.rust_mut();
        let now = this.now();
        this.list.add(id, (after >= 0).then_some(after), now)
    }

    fn select(self: Pin<&mut Self>, id: i32) {
        let mut this = self.rust_mut();
        let now = this.now();
        this.list.select(id, now);
    }

    fn remove(self: Pin<&mut Self>, id: i32) -> i32 {
        self.rust_mut().list.remove(id).unwrap_or(-1)
    }

    fn current(&self) -> i32 {
        self.list.current().unwrap_or(-1)
    }

    fn order(&self) -> QString {
        text(&json!(self.list.items()))
    }

    fn is_full(&self) -> bool {
        self.list.is_full()
    }

    fn count(&self) -> i32 {
        i32::try_from(self.list.len()).unwrap_or(i32::MAX)
    }

    fn neighbour(&self, steps: i32) -> i32 {
        isize::try_from(steps).ok().and_then(|s| self.list.neighbour(s)).unwrap_or(-1)
    }

    fn to_unload(&self, keep: i32, live: &QString) -> QString {
        let live = ids(live);
        let is_live = |t: i32| live.contains(&t);
        let mut out = self.list.beyond(usize::try_from(keep).unwrap_or(0), is_live);
        for t in self.list.rested(self.rest_ms, self.now(), is_live) {
            if !out.contains(&t) {
                out.push(t);
            }
        }
        text(&json!(out))
    }

    fn next_rest(&self, live: &QString) -> i32 {
        let live = ids(live);
        self.list
            .next_rest(self.rest_ms, self.now(), |t| live.contains(&t))
            .map_or(-1, |ms| i32::try_from(ms).unwrap_or(i32::MAX))
    }
}
