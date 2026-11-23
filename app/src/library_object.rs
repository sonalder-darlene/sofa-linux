//! `Library`, the QML singleton for Quick access tiles, bookmarks with folders, and history. Lists go to QML as JSON
//! text (read with `JSON.parse`), which keeps the bridge small; the rules are all in `sofa_core::library`.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        /// Qt's string.
        type QString = cxx_qt_lib::QString;
    }

    extern "RustQt" {
        /// The user's sites, as QML sees them.
        #[qobject]
        #[qml_element]
        #[qml_singleton]
        type Library = super::LibraryRust;

        /// `[{id, title, url}]`, in order.
        #[qinvokable]
        fn tiles(&self) -> QString;
        #[qinvokable]
        #[cxx_name = "addTile"]
        fn add_tile(&self, title: &QString, url: &QString);
        #[qinvokable]
        #[cxx_name = "updateTile"]
        fn update_tile(&self, id: i64, title: &QString, url: &QString);
        #[qinvokable]
        #[cxx_name = "removeTile"]
        fn remove_tile(&self, id: i64);
        /// `ids` is a JSON array of tile ids in their new order.
        #[qinvokable]
        #[cxx_name = "reorderTiles"]
        fn reorder_tiles(self: Pin<&mut Self>, ids: &QString);

        /// The start page's Bookmarks cards: `{folders: [{id, title, folders, bookmarks}], bookmarks: [{id, title,
        /// url}], total}` for the top level, at most `count` bookmarks.
        #[qinvokable]
        #[cxx_name = "startBookmarks"]
        fn start_bookmarks(&self, count: i32) -> QString;
        /// Folder `id` (0: the top level): `{id, title, path: [{id, title}], folders: [...], bookmarks: [...]}`.
        #[qinvokable]
        fn folder(&self, id: i64) -> QString;
        #[qinvokable]
        #[cxx_name = "isBookmarked"]
        fn is_bookmarked(&self, url: &QString) -> bool;
        /// Bookmarks the page, or removes its bookmark; returns whether it is bookmarked now.
        #[qinvokable]
        #[cxx_name = "toggleBookmark"]
        fn toggle_bookmark(&self, title: &QString, url: &QString) -> bool;
        /// Makes a folder in `parent` (0: the top level): `{id}` or `{error: "empty" | "exists" | "deep"}`.
        #[qinvokable]
        #[cxx_name = "addFolder"]
        fn add_folder(&self, parent: i64, title: &QString) -> QString;
        /// "" when renamed, else "exists" (another folder there has that name) or "missing".
        #[qinvokable]
        #[cxx_name = "renameFolder"]
        fn rename_folder(&self, id: i64, title: &QString) -> QString;
        /// Where a folder (`folder` true) or a bookmark `id` can move: `[{id, title, depth, allowed, here}]`, the
        /// top level first.
        #[qinvokable]
        #[cxx_name = "moveTargets"]
        fn move_targets(&self, folder: bool, id: i64) -> QString;
        #[qinvokable]
        #[cxx_name = "moveBookmark"]
        fn move_bookmark(&self, id: i64, to: i64);
        /// False when the move isn't allowed.
        #[qinvokable]
        #[cxx_name = "moveFolder"]
        fn move_folder(&self, id: i64, to: i64) -> bool;
        /// What folder `id` holds at any depth: `{folders, bookmarks}`.
        #[qinvokable]
        #[cxx_name = "folderContents"]
        fn folder_contents(&self, id: i64) -> QString;
        #[qinvokable]
        #[cxx_name = "deleteFolder"]
        fn delete_folder(self: Pin<&mut Self>, id: i64);
        #[qinvokable]
        #[cxx_name = "deleteBookmark"]
        fn delete_bookmark(&self, id: i64);
        #[qinvokable]
        #[cxx_name = "clearBookmarks"]
        fn clear_bookmarks(&self);

        /// Reads a browser's bookmarks file at `path` (a path or a `file:` URL) and adds what's new:
        /// `{added, folders, already, skipped}` or `{error: "unreadable" | "large" | "empty" | "failed"}`.
        #[qinvokable]
        #[cxx_name = "importFile"]
        fn import_file(self: Pin<&mut Self>, path: &QString) -> QString;
        /// Writes every bookmark as a bookmarks file into the Downloads folder: `{name, folder, count}` or `{error}`.
        #[qinvokable]
        #[cxx_name = "exportFile"]
        fn export_file(&self) -> QString;
        /// The newest bookmarks files in the Downloads folder: `[{path, name}]`, at most 5.
        #[qinvokable]
        #[cxx_name = "downloadedFiles"]
        fn downloaded_files(&self) -> QString;

        /// The newest `limit` visits: `[{id, title, url, visited}]`.
        #[qinvokable]
        fn history(&self, limit: i32) -> QString;
        #[qinvokable]
        #[cxx_name = "recordVisit"]
        fn record_visit(&self, title: &QString, url: &QString);
        #[qinvokable]
        #[cxx_name = "updateTitle"]
        fn update_title(&self, url: &QString, title: &QString);
        #[qinvokable]
        #[cxx_name = "removeVisit"]
        fn remove_visit(&self, id: i64);
        #[qinvokable]
        #[cxx_name = "clearHistory"]
        fn clear_history(&self);
        /// Settings → Keep history for changed to `days` (as `history_days` stores it): older visits go at once.
        #[qinvokable]
        #[cxx_name = "keepHistoryFor"]
        fn keep_history_for(&self, days: i32);
        /// `[{id, title, url}]`: sites visited at least three times in the last 30 days.
        #[qinvokable]
        #[cxx_name = "mostVisited"]
        fn most_visited(&self, limit: i32) -> QString;
        /// `[{id, title, url}]`: bookmarks, then history, matching `text`.
        #[qinvokable]
        fn suggest(&self, text: &QString) -> QString;
    }
}

use std::cell::Cell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::time::{SystemTime, UNIX_EPOCH};

use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use serde_json::{Value, json};
use sofa_core::bookmark_file;
use sofa_core::folders::{self, FolderId, FolderTree};
use sofa_core::history_days;
use sofa_core::library::{self, Library, Site};
use sofa_core::settings::{KeyFile, SETTINGS};
use sofa_core::sites;

use crate::paths;

/// State behind [`qobject::Library`]: the open database, or `None` when it couldn't be opened (then every list is
/// empty and changes are dropped, and the reason was printed once).
#[derive(Debug)]
pub struct LibraryRust {
    db: Option<Library>,
    // Settings → Keep history for (sofa_core::history_days).
    history_days: Cell<i32>,
}

impl Default for LibraryRust {
    fn default() -> Self {
        let opened = paths::data_dir().ok_or_else(|| "no home folder".to_owned()).and_then(|dir| {
            sofa_core::settings::create_private_dir(&dir).map_err(|e| e.to_string())?;
            Library::open(&dir.join("library.db")).map_err(|e| e.to_string())
        });
        // Read once here; Settings tells about a change through keep_history_for().
        let settings = KeyFile::load(paths::config_dir().map(|d| d.join("settings.conf")), SETTINGS);
        let days = history_days::from_stored(settings.get("history_days"));
        match opened {
            Ok(db) => {
                let library = Self { db: Some(db), history_days: Cell::new(days) };
                library.prune();
                library
            }
            Err(e) => {
                eprintln!("sofa-tv-browser: bookmarks and history are off for this run: {e}");
                Self { db: None, history_days: Cell::new(days) }
            }
        }
    }
}

impl LibraryRust {
    /// Removes the visits Keep history for no longer keeps.
    fn prune(&self) {
        if let (Some(db), Some(cutoff)) = (&self.db, history_days::cutoff(self.history_days.get(), now())) {
            or_log(db.prune_history(cutoff), ());
        }
    }
}

fn now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
}

fn folder_id(id: i64) -> Option<FolderId> {
    (id > 0).then_some(FolderId(id))
}

fn text(value: &Value) -> QString {
    QString::from(value.to_string().as_str())
}

fn sites_json(list: &[Site]) -> Value {
    Value::Array(list.iter().map(|s| json!({ "id": s.id, "title": s.title, "url": s.url })).collect())
}

/// Logs a failed database call without the user's data in it, and gives the fallback.
fn or_log<T>(result: library::Result<T>, fallback: T) -> T {
    result.unwrap_or_else(|e| {
        eprintln!("sofa-tv-browser: {e}");
        fallback
    })
}

/// The folders in `parent` as cards: name, and how many folders and bookmarks each holds (`counts`, from
/// [`Library::bookmark_counts`]).
fn folder_cards(tree: &FolderTree, counts: &HashMap<Option<FolderId>, usize>, parent: Option<FolderId>) -> Value {
    Value::Array(
        tree.children(parent)
            .iter()
            .map(|f| {
                json!({
                    "id": f.id.0,
                    "title": f.title,
                    "folders": tree.children(Some(f.id)).len(),
                    "bookmarks": counts.get(&Some(f.id)).copied().unwrap_or(0),
                })
            })
            .collect(),
    )
}

impl qobject::Library {
    fn tiles(&self) -> QString {
        let Some(db) = &self.db else { return QString::from("[]") };
        text(&sites_json(&or_log(db.tiles(), Vec::new())))
    }

    fn add_tile(&self, title: &QString, url: &QString) {
        let url = String::from(url);
        if let Some(db) = self.db.as_ref().filter(|_| sites::is_web_url(&url)) {
            or_log(db.add_tile(&String::from(title), &url), ());
        }
    }

    fn update_tile(&self, id: i64, title: &QString, url: &QString) {
        let url = String::from(url);
        if let Some(db) = self.db.as_ref().filter(|_| sites::is_web_url(&url)) {
            or_log(db.update_tile(id, &String::from(title), &url), ());
        }
    }

    fn remove_tile(&self, id: i64) {
        if let Some(db) = &self.db {
            or_log(db.remove_tile(id), ());
        }
    }

    fn reorder_tiles(self: Pin<&mut Self>, ids: &QString) {
        let ids: Vec<i64> = serde_json::from_str(&String::from(ids)).unwrap_or_default();
        if let Some(db) = self.rust_mut().get_mut().db.as_mut() {
            or_log(db.reorder_tiles(&ids), ());
        }
    }

    fn start_bookmarks(&self, count: i32) -> QString {
        let Some(db) = &self.db else { return text(&json!({ "folders": [], "bookmarks": [], "total": 0 })) };
        let tree = or_log(db.folder_tree(), FolderTree::default());
        let counts = or_log(db.bookmark_counts(), Default::default());
        let slots = usize::try_from(count).unwrap_or(0);
        let total = tree.children(None).len() + counts.get(&None).copied().unwrap_or(0);
        text(&json!({
            "folders": folder_cards(&tree, &counts, None),
            "bookmarks": sites_json(&or_log(db.bookmarks(None, slots), Vec::new())),
            "total": total,
        }))
    }

    fn folder(&self, id: i64) -> QString {
        let Some(db) = &self.db else {
            return text(&json!({ "id": 0, "title": "", "path": [], "folders": [], "bookmarks": [] }));
        };
        let tree = or_log(db.folder_tree(), FolderTree::default());
        // A folder deleted meanwhile shows the top level.
        let id = folder_id(id).filter(|f| tree.get(Some(*f)).is_some());
        let path: Vec<Value> = tree.path(id).iter().map(|f| json!({ "id": f.id.0, "title": f.title })).collect();
        text(&json!({
            "id": id.map_or(0, |f| f.0),
            "title": tree.get(id).map_or("", |f| f.title.as_str()),
            "path": path,
            "folders": folder_cards(&tree, &or_log(db.bookmark_counts(), HashMap::new()), id),
            "bookmarks": sites_json(&or_log(db.bookmarks(id, 0), Vec::new())),
        }))
    }

    fn is_bookmarked(&self, url: &QString) -> bool {
        self.db.as_ref().is_some_and(|db| or_log(db.is_bookmarked(&String::from(url)), false))
    }

    fn toggle_bookmark(&self, title: &QString, url: &QString) -> bool {
        let url = String::from(url);
        let Some(db) = self.db.as_ref().filter(|_| sites::is_web_url(&url)) else { return false };
        if or_log(db.is_bookmarked(&url), false) {
            or_log(db.remove_bookmark(&url), ());
            false
        } else {
            or_log(db.add_bookmark(&String::from(title), &url, now()), ());
            true
        }
    }

    fn add_folder(&self, parent: i64, title: &QString) -> QString {
        let Some(db) = &self.db else { return text(&json!({ "error": "failed" })) };
        let name = folders::clean_name(&String::from(title));
        let tree = or_log(db.folder_tree(), FolderTree::default());
        let parent = folder_id(parent);
        let error = if name.is_empty() {
            Some("empty")
        } else if tree.find(parent, &name).is_some() {
            Some("exists")
        } else if !tree.can_hold(parent) {
            Some("deep")
        } else {
            None
        };
        if let Some(error) = error {
            return text(&json!({ "error": error }));
        }
        match db.add_folder(parent, &name, now()) {
            Ok(id) => text(&json!({ "id": id.0 })),
            Err(e) => {
                eprintln!("sofa-tv-browser: {e}");
                text(&json!({ "error": "failed" }))
            }
        }
    }

    fn rename_folder(&self, id: i64, title: &QString) -> QString {
        let Some(db) = &self.db else { return QString::from("missing") };
        let tree = or_log(db.folder_tree(), FolderTree::default());
        let Some(folder) = tree.get(folder_id(id)) else { return QString::from("missing") };
        let name = folders::clean_name(&String::from(title));
        // An empty name keeps the old one.
        if name.is_empty() || name == folder.title {
            return QString::default();
        }
        if tree.find(folder.parent, &name).is_some_and(|other| other.id != folder.id) {
            return QString::from("exists");
        }
        or_log(db.rename_folder(folder.id, &name), ());
        QString::default()
    }

    fn move_targets(&self, folder: bool, id: i64) -> QString {
        let Some(db) = &self.db else { return QString::from("[]") };
        let tree = or_log(db.folder_tree(), FolderTree::default());
        let (here, moving) = if folder {
            let f = tree.get(folder_id(id));
            (f.and_then(|f| f.parent), f)
        } else {
            (or_log(db.bookmark_folder(id), None), None)
        };
        let allowed = |to: Option<FolderId>| match moving {
            // A folder can't go into itself, below the deepest level, or next to a folder with its name.
            Some(f) => {
                tree.can_move(f.id, to)
                    && tree.find(to, &f.title).is_none_or(|same| same.id == f.id)
                    && to != Some(f.id)
            }
            None => true,
        };
        let mut out =
            vec![json!({ "id": 0, "title": "", "depth": 0, "allowed": allowed(None), "here": here.is_none() })];
        out.extend(tree.outline().iter().map(|(f, depth)| {
            json!({
                "id": f.id.0,
                "title": f.title,
                "depth": depth,
                "allowed": allowed(Some(f.id)),
                "here": here == Some(f.id),
            })
        }));
        text(&Value::Array(out))
    }

    fn move_bookmark(&self, id: i64, to: i64) {
        if let Some(db) = &self.db {
            or_log(db.move_bookmark(id, folder_id(to)), ());
        }
    }

    fn move_folder(&self, id: i64, to: i64) -> bool {
        let Some(db) = &self.db else { return false };
        let tree = or_log(db.folder_tree(), FolderTree::default());
        let (Some(f), to) = (tree.get(folder_id(id)), folder_id(to)) else { return false };
        if !tree.can_move(f.id, to) || tree.find(to, &f.title).is_some_and(|same| same.id != f.id) {
            return false;
        }
        or_log(db.move_folder(f.id, to).map(|()| true), false)
    }

    fn folder_contents(&self, id: i64) -> QString {
        let Some(db) = &self.db else { return text(&json!({ "folders": 0, "bookmarks": 0 })) };
        let tree = or_log(db.folder_tree(), FolderTree::default());
        let Some(f) = folder_id(id) else { return text(&json!({ "folders": 0, "bookmarks": 0 })) };
        let inside = tree.subtree(f);
        let counts = or_log(db.bookmark_counts(), Default::default());
        let bookmarks: usize = inside.iter().map(|f| counts.get(&Some(*f)).copied().unwrap_or(0)).sum();
        text(&json!({ "folders": inside.len().saturating_sub(1), "bookmarks": bookmarks }))
    }

    fn delete_folder(self: Pin<&mut Self>, id: i64) {
        if let (Some(db), Some(f)) = (self.rust_mut().get_mut().db.as_mut(), folder_id(id)) {
            or_log(db.delete_folder(f), ());
        }
    }

    fn delete_bookmark(&self, id: i64) {
        if let Some(db) = &self.db {
            or_log(db.delete_bookmark(id), ());
        }
    }

    fn clear_bookmarks(&self) {
        if let Some(db) = &self.db {
            or_log(db.clear_bookmarks(), ());
        }
    }

    fn import_file(self: Pin<&mut Self>, path: &QString) -> QString {
        let error = |e: &str| text(&json!({ "error": e }));
        let Some(path) = local_path(&String::from(path)) else { return error("unreadable") };
        let size = std::fs::metadata(&path).map(|m| m.len());
        match size {
            Err(_) => return error("unreadable"),
            Ok(n) if n > u64::try_from(bookmark_file::MAX_BYTES).unwrap_or(u64::MAX) => return error("large"),
            Ok(_) => {}
        }
        let Ok(bytes) = std::fs::read(&path) else { return error("unreadable") };
        let contents = bookmark_file::read(&String::from_utf8_lossy(&bytes), now());
        if contents.entries.is_empty() && contents.folders.is_empty() && contents.skipped == 0 {
            return error("empty");
        }
        let Some(db) = self.rust_mut().get_mut().db.as_mut() else { return error("failed") };
        match db.import_bookmarks(&contents.entries, &contents.folders, now()) {
            Ok(done) => text(&json!({
                "added": done.bookmarks,
                "folders": done.folders,
                "already": contents.entries.len() - done.bookmarks,
                "skipped": contents.skipped,
            })),
            Err(e) => {
                eprintln!("sofa-tv-browser: {e}");
                error("failed")
            }
        }
    }

    fn export_file(&self) -> QString {
        let error = |e: &str| text(&json!({ "error": e }));
        let Some(db) = &self.db else { return error("failed") };
        let Some(dir) = paths::downloads_dir() else { return error("nowhere") };
        let entries = or_log(db.bookmarks_to_export(), Vec::new());
        let folders = or_log(db.folders_to_export(), Vec::new());
        let file = bookmark_file::write(&entries, &folders);
        let path = free_name(&dir, &format!("sofa-bookmarks-{}", utc_date(now())), "html");
        let written = std::fs::create_dir_all(&dir).and_then(|()| std::fs::write(&path, file));
        match written {
            Ok(()) => text(&json!({
                "name": path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
                "folder": dir.to_string_lossy(),
                "count": entries.len(),
            })),
            Err(e) => {
                eprintln!("sofa-tv-browser: the bookmarks file could not be written: {e}");
                error("failed")
            }
        }
    }

    fn downloaded_files(&self) -> QString {
        let Some(dir) = paths::downloads_dir() else { return QString::from("[]") };
        let mut files: Vec<(SystemTime, PathBuf)> = std::fs::read_dir(&dir)
            .into_iter()
            .flatten()
            .flatten()
            .filter(|e| {
                let name = e.file_name().to_string_lossy().to_ascii_lowercase();
                (name.ends_with(".html") || name.ends_with(".htm")) && e.file_type().is_ok_and(|t| t.is_file())
            })
            .map(|e| (e.metadata().and_then(|m| m.modified()).unwrap_or(UNIX_EPOCH), e.path()))
            .collect();
        files.sort_by_key(|f| std::cmp::Reverse(f.0));
        let list: Vec<Value> = files
            .into_iter()
            .take(5)
            .map(|(_, p)| {
                json!({
                    "path": p.to_string_lossy(),
                    "name": p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
                })
            })
            .collect();
        text(&Value::Array(list))
    }

    fn history(&self, limit: i32) -> QString {
        let Some(db) = &self.db else { return QString::from("[]") };
        let visits = or_log(db.history(usize::try_from(limit).unwrap_or(0)), Vec::new());
        let list: Vec<Value> = visits
            .iter()
            .map(|v| json!({ "id": v.id, "title": v.title, "url": v.url, "visited": v.visited }))
            .collect();
        text(&Value::Array(list))
    }

    fn record_visit(&self, title: &QString, url: &QString) {
        if !history_days::records(self.history_days.get()) {
            return;
        }
        let url = String::from(url);
        if let Some(db) = self.db.as_ref().filter(|_| sites::is_web_url(&url)) {
            or_log(db.record_visit(&String::from(title), &url, now()), ());
        }
    }

    fn update_title(&self, url: &QString, title: &QString) {
        let title = String::from(title);
        if let Some(db) = self.db.as_ref().filter(|_| !title.trim().is_empty()) {
            or_log(db.update_history_title(&String::from(url), &title), ());
        }
    }

    fn remove_visit(&self, id: i64) {
        if let Some(db) = &self.db {
            or_log(db.remove_visit(id), ());
        }
    }

    fn clear_history(&self) {
        if let Some(db) = &self.db {
            or_log(db.clear_history(), ());
        }
    }

    fn keep_history_for(&self, days: i32) {
        self.history_days.set(history_days::from_stored(&days.to_string()));
        self.prune();
    }

    fn most_visited(&self, limit: i32) -> QString {
        let Some(db) = &self.db else { return QString::from("[]") };
        text(&sites_json(&or_log(db.most_visited(usize::try_from(limit).unwrap_or(0), 3, now()), Vec::new())))
    }

    fn suggest(&self, query: &QString) -> QString {
        let Some(db) = &self.db else { return QString::from("[]") };
        text(&sites_json(&or_log(db.suggest(&String::from(query), 6), Vec::new())))
    }
}

/// A local file from a path or a `file:` URL (as a file dialog gives it).
fn local_path(path: &str) -> Option<PathBuf> {
    if path.starts_with("file:") {
        return url::Url::parse(path).ok()?.to_file_path().ok();
    }
    let p = PathBuf::from(path);
    p.is_absolute().then_some(p)
}

/// `dir/stem.ext`, or `dir/stem (1).ext` and so on when the name is taken.
fn free_name(dir: &Path, stem: &str, ext: &str) -> PathBuf {
    let first = dir.join(format!("{stem}.{ext}"));
    if !first.exists() {
        return first;
    }
    (1..1000).map(|n| dir.join(format!("{stem} ({n}).{ext}"))).find(|p| !p.exists()).unwrap_or(first)
}

/// `YYYY-MM-DD` of `ms` in UTC (the civil-from-days algorithm), for file names.
fn utc_date(ms: i64) -> String {
    let days = ms.div_euclid(86_400_000);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates_name_files() {
        assert_eq!(utc_date(0), "1970-01-01");
        assert_eq!(utc_date(1_763_510_400_000), "2025-11-19");
        assert_eq!(utc_date(951_782_400_000), "2000-02-29");
    }

    #[test]
    fn file_urls_and_paths_are_read() {
        assert_eq!(local_path("file:///tmp/a%20b.html"), Some(PathBuf::from("/tmp/a b.html")));
        assert_eq!(local_path("/tmp/x.html"), Some(PathBuf::from("/tmp/x.html")));
        assert_eq!(local_path("relative.html"), None);
        assert_eq!(local_path("https://example.com/x.html"), None);
    }

    #[test]
    fn taken_names_get_a_number() {
        let dir = std::env::temp_dir().join(format!("sofa-free-name-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        assert_eq!(free_name(&dir, "a", "html"), dir.join("a.html"));
        std::fs::write(dir.join("a.html"), "").expect("write");
        assert_eq!(free_name(&dir, "a", "html"), dir.join("a (1).html"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
