//! Everything Sofa keeps about the user's sites: Quick access tiles, bookmarks and their folders, and history, in
//! one SQLite file. It stays on the device: no sync, no upload.
//!
//! The tables are those of Sofa on Android (`tiles`, `bookmarks`, `folders`, `history`), so the rules ported from
//! there (what "most visited" means, how an import merges) read the same.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::path::Path;

use crate::sqlite::{self, Connection, Value};

use crate::bookmark_file::{Entry, FolderEntry};
use crate::folders::{Folder, FolderId, FolderTree};
use crate::sites;

/// A site on the start page or in a list: a tile, a bookmark, a history visit or a most-visited site.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Site {
    /// Its row id (negative for a most-visited site, which has no row of its own).
    pub id: i64,
    /// Its title.
    pub title: String,
    /// Its address.
    pub url: String,
}

/// One visit in History.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Visit {
    /// Its row id.
    pub id: i64,
    /// The page's title.
    pub title: String,
    /// Its address.
    pub url: String,
    /// When, in milliseconds since 1970.
    pub visited: i64,
}

/// What an import added: bookmarks, and the folders they went into.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Imported {
    /// New bookmarks.
    pub bookmarks: usize,
    /// Folders that received them.
    pub folders: usize,
}

/// A database failure.
#[derive(Debug)]
pub struct Error(sqlite::Error);

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "the library database failed: {}", self.0)
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.0)
    }
}

impl From<sqlite::Error> for Error {
    fn from(e: sqlite::Error) -> Self {
        Self(e)
    }
}

/// A database result.
pub type Result<T> = std::result::Result<T, Error>;

/// The tiles a new library starts with (as on Android): sites people often watch on a TV.
pub const DEFAULT_TILES: &[(&str, &str)] = &[
    ("YouTube", "https://www.youtube.com/"),
    ("Crunchyroll", "https://www.crunchyroll.com/videos/anime"),
    ("IGN", "https://www.ign.com/videos"),
    ("Plex", "https://app.plex.tv/desktop/#!/"),
    ("Wikipedia", "https://www.wikipedia.org/"),
];

/// A visit to the same address within this many milliseconds of the last one updates it instead of adding a row.
const SAME_VISIT_MS: i64 = 60_000;
const DAY_MS: i64 = 24 * 60 * 60 * 1000;

/// How long a Most visited list is kept while History and the tiles stay as they are (the 30 days move slowly).
const MOST_VISITED_KEPT_MS: i64 = 60 * 60 * 1000;

/// The library, open on one file.
#[derive(Debug)]
pub struct Library {
    db: Connection,
    // The start page asks for Most visited on every Home and every return to it; the list is kept until History or
    // the tiles change.
    most_visited: RefCell<Option<MostVisited>>,
}

#[derive(Debug)]
struct MostVisited {
    at: i64,
    limit: usize,
    min_visits: usize,
    sites: Vec<Site>,
}

impl Library {
    /// Opens (or creates) the library at `path`.
    ///
    /// # Errors
    ///
    /// The file can't be opened or its tables can't be made.
    pub fn open(path: &Path) -> Result<Self> {
        Self::with(Connection::open(path)?)
    }

    /// A library in memory, for tests and private checks.
    ///
    /// # Errors
    ///
    /// SQLite can't start.
    pub fn in_memory() -> Result<Self> {
        Self::with(Connection::open_in_memory()?)
    }

    fn with(db: Connection) -> Result<Self> {
        // What is deleted is overwritten as well, so a removed visit or bookmark can't be read back from the file.
        db.execute_batch("PRAGMA journal_mode = WAL; PRAGMA foreign_keys = OFF; PRAGMA secure_delete = ON;")?;
        let version: i64 = db.query_one("PRAGMA user_version", &[], |r| r.get(0))?;
        if version < 1 {
            db.execute_batch(
                "BEGIN;
                 CREATE TABLE tiles (id INTEGER PRIMARY KEY, title TEXT NOT NULL, url TEXT NOT NULL, position INTEGER NOT NULL);
                 CREATE TABLE bookmarks (id INTEGER PRIMARY KEY, title TEXT NOT NULL, url TEXT NOT NULL UNIQUE,
                                         created INTEGER NOT NULL, folder INTEGER);
                 CREATE INDEX bookmarks_folder ON bookmarks (folder);
                 CREATE TABLE folders (id INTEGER PRIMARY KEY, parent INTEGER, title TEXT NOT NULL, created INTEGER NOT NULL);
                 CREATE TABLE history (id INTEGER PRIMARY KEY, title TEXT NOT NULL, url TEXT NOT NULL, visited INTEGER NOT NULL);
                 CREATE INDEX history_visited ON history (visited);
                 CREATE INDEX history_url ON history (url);
                 PRAGMA user_version = 1;
                 COMMIT;",
            )?;
            for (i, (title, url)) in DEFAULT_TILES.iter().enumerate() {
                db.execute(
                    "INSERT INTO tiles (title, url, position) VALUES (?1, ?2, ?3)",
                    &[(*title).into(), (*url).into(), i.into()],
                )?;
            }
        }
        Ok(Self { db, most_visited: RefCell::new(None) })
    }

    /// History or the tiles changed: Most visited is counted again next time.
    fn forget_most_visited(&self) {
        self.most_visited.replace(None);
    }

    // Tiles

    /// The Quick access tiles, in their order.
    ///
    /// # Errors
    ///
    /// A database failure.
    pub fn tiles(&self) -> Result<Vec<Site>> {
        self.sites("SELECT id, title, url FROM tiles ORDER BY position", &[])
    }

    /// Adds a tile at the end.
    ///
    /// # Errors
    ///
    /// A database failure.
    pub fn add_tile(&self, title: &str, url: &str) -> Result<()> {
        self.forget_most_visited();
        self.db.execute(
            "INSERT INTO tiles (title, url, position) VALUES (?1, ?2, (SELECT COALESCE(MAX(position) + 1, 0) FROM tiles))",
            &[non_blank(title, url).into(), url.into()],
        )?;
        Ok(())
    }

    /// Changes a tile's title and address.
    ///
    /// # Errors
    ///
    /// A database failure.
    pub fn update_tile(&self, id: i64, title: &str, url: &str) -> Result<()> {
        self.forget_most_visited();
        self.db.execute(
            "UPDATE tiles SET title = ?1, url = ?2 WHERE id = ?3",
            &[non_blank(title, url).into(), url.into(), id.into()],
        )?;
        Ok(())
    }

    /// Removes a tile.
    ///
    /// # Errors
    ///
    /// A database failure.
    pub fn remove_tile(&self, id: i64) -> Result<()> {
        self.forget_most_visited();
        self.db.execute("DELETE FROM tiles WHERE id = ?1", &[id.into()])?;
        Ok(())
    }

    /// Stores `ids` as the new tile order.
    ///
    /// # Errors
    ///
    /// A database failure.
    pub fn reorder_tiles(&mut self, ids: &[i64]) -> Result<()> {
        let tx = self.db.transaction()?;
        for (i, id) in ids.iter().enumerate() {
            tx.execute("UPDATE tiles SET position = ?1 WHERE id = ?2", &[i.into(), (*id).into()])?;
        }
        tx.commit()?;
        Ok(())
    }

    // Bookmarks and folders

    /// The bookmarks in `folder` (`None`: the top level), newest first; at most `limit` when it isn't 0.
    ///
    /// # Errors
    ///
    /// A database failure.
    pub fn bookmarks(&self, folder: Option<FolderId>, limit: usize) -> Result<Vec<Site>> {
        let limit = if limit == 0 { -1 } else { i64::try_from(limit).unwrap_or(i64::MAX) };
        self.sites(
            "SELECT id, title, url FROM bookmarks WHERE folder IS ?1 ORDER BY created DESC, id DESC LIMIT ?2",
            &[folder.map(|f| f.0).into(), limit.into()],
        )
    }

    /// How many bookmarks each folder holds itself (not counting its folders); the top level under `None`.
    ///
    /// # Errors
    ///
    /// A database failure.
    pub fn bookmark_counts(&self) -> Result<HashMap<Option<FolderId>, usize>> {
        let rows = self.db.query_all("SELECT folder, COUNT(*) FROM bookmarks GROUP BY folder", &[], |r| {
            Ok((r.get::<Option<i64>>(0)?.map(FolderId), usize::try_from(r.get::<i64>(1)?).unwrap_or(0)))
        })?;
        Ok(rows.into_iter().collect())
    }

    /// All folders as a tree.
    ///
    /// # Errors
    ///
    /// A database failure.
    pub fn folder_tree(&self) -> Result<FolderTree> {
        let rows = self.db.query_all("SELECT id, parent, title, created FROM folders", &[], |r| {
            Ok(Folder {
                id: FolderId(r.get(0)?),
                parent: r.get::<Option<i64>>(1)?.map(FolderId),
                title: r.get(2)?,
                created: r.get(3)?,
            })
        })?;
        Ok(FolderTree::new(rows))
    }

    /// Whether `url` is bookmarked.
    ///
    /// # Errors
    ///
    /// A database failure.
    pub fn is_bookmarked(&self, url: &str) -> Result<bool> {
        Ok(self.db.query_row("SELECT 1 FROM bookmarks WHERE url = ?1", &[url.into()], |_| Ok(()))?.is_some())
    }

    /// Bookmarks `url` at the top level; one already bookmarked stays where it is, with its title.
    ///
    /// # Errors
    ///
    /// A database failure.
    pub fn add_bookmark(&self, title: &str, url: &str, now: i64) -> Result<()> {
        self.db.execute(
            "INSERT OR IGNORE INTO bookmarks (title, url, created) VALUES (?1, ?2, ?3)",
            &[non_blank(title, url).into(), url.into(), now.into()],
        )?;
        Ok(())
    }

    /// Removes the bookmark of `url`.
    ///
    /// # Errors
    ///
    /// A database failure.
    pub fn remove_bookmark(&self, url: &str) -> Result<()> {
        self.db.execute("DELETE FROM bookmarks WHERE url = ?1", &[url.into()])?;
        Ok(())
    }

    /// Every bookmark and every folder.
    ///
    /// # Errors
    ///
    /// A database failure.
    pub fn clear_bookmarks(&self) -> Result<()> {
        self.db.execute_batch("BEGIN; DELETE FROM bookmarks; DELETE FROM folders; COMMIT;")?;
        Ok(())
    }

    /// A new folder called `title` in `parent`; its id.
    ///
    /// # Errors
    ///
    /// A database failure.
    pub fn add_folder(&self, parent: Option<FolderId>, title: &str, now: i64) -> Result<FolderId> {
        self.db.execute(
            "INSERT INTO folders (parent, title, created) VALUES (?1, ?2, ?3)",
            &[parent.map(|p| p.0).into(), title.into(), now.into()],
        )?;
        Ok(FolderId(self.db.last_insert_rowid()))
    }

    /// Renames folder `id`.
    ///
    /// # Errors
    ///
    /// A database failure.
    pub fn rename_folder(&self, id: FolderId, title: &str) -> Result<()> {
        self.db.execute("UPDATE folders SET title = ?1 WHERE id = ?2", &[title.into(), id.0.into()])?;
        Ok(())
    }

    /// Moves folder `id` into `to`; the caller checked [`FolderTree::can_move`].
    ///
    /// # Errors
    ///
    /// A database failure.
    pub fn move_folder(&self, id: FolderId, to: Option<FolderId>) -> Result<()> {
        self.db.execute("UPDATE folders SET parent = ?1 WHERE id = ?2", &[to.map(|t| t.0).into(), id.0.into()])?;
        Ok(())
    }

    /// The folder bookmark `id` is in (`None`: the top level, or no such bookmark).
    ///
    /// # Errors
    ///
    /// A database failure.
    pub fn bookmark_folder(&self, id: i64) -> Result<Option<FolderId>> {
        let folder: Option<Option<i64>> =
            self.db.query_row("SELECT folder FROM bookmarks WHERE id = ?1", &[id.into()], |r| r.get(0))?;
        Ok(folder.flatten().map(FolderId))
    }

    /// Moves bookmark `id` into `to`.
    ///
    /// # Errors
    ///
    /// A database failure.
    pub fn move_bookmark(&self, id: i64, to: Option<FolderId>) -> Result<()> {
        self.db.execute("UPDATE bookmarks SET folder = ?1 WHERE id = ?2", &[to.map(|t| t.0).into(), id.into()])?;
        Ok(())
    }

    /// Removes bookmark `id`.
    ///
    /// # Errors
    ///
    /// A database failure.
    pub fn delete_bookmark(&self, id: i64) -> Result<()> {
        self.db.execute("DELETE FROM bookmarks WHERE id = ?1", &[id.into()])?;
        Ok(())
    }

    /// Folder `id` with every folder and bookmark in it.
    ///
    /// # Errors
    ///
    /// A database failure.
    pub fn delete_folder(&mut self, id: FolderId) -> Result<()> {
        let ids = self.folder_tree()?.subtree(id);
        let tx = self.db.transaction()?;
        for f in ids {
            tx.execute("DELETE FROM bookmarks WHERE folder = ?1", &[f.0.into()])?;
            tx.execute("DELETE FROM folders WHERE id = ?1", &[f.0.into()])?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Every bookmark with when it was added and its folder, newest first, for a bookmarks file.
    ///
    /// # Errors
    ///
    /// A database failure.
    pub fn bookmarks_to_export(&self) -> Result<Vec<Entry>> {
        let tree = self.folder_tree()?;
        let rows = self.db.query_all(
            "SELECT title, url, created, folder FROM bookmarks ORDER BY created DESC, id DESC",
            &[],
            |r| {
                Ok(Entry {
                    title: r.get(0)?,
                    url: r.get(1)?,
                    added: Some(r.get(2)?),
                    folder: tree.path_names(r.get::<Option<i64>>(3)?.map(FolderId)),
                })
            },
        )?;
        Ok(rows)
    }

    /// Every folder by its path, so a file keeps empty ones and the dates they were made.
    ///
    /// # Errors
    ///
    /// A database failure.
    pub fn folders_to_export(&self) -> Result<Vec<FolderEntry>> {
        let tree = self.folder_tree()?;
        Ok(tree
            .outline()
            .into_iter()
            .map(|(f, _)| FolderEntry { path: tree.path_names(Some(f.id)), added: Some(f.created) })
            .collect())
    }

    /// Adds `entries` that aren't bookmarked yet, each in its folder, made where missing. A folder already there
    /// with the same name in the same place takes them, so importing a file twice adds nothing. A bookmark already
    /// there stays in its folder with its title. Entries without a date sort as just added, in file order. `folders`
    /// are made too even when empty, and give made folders their dates.
    ///
    /// # Errors
    ///
    /// A database failure; then nothing is added.
    pub fn import_bookmarks(&mut self, entries: &[Entry], folders: &[FolderEntry], now: i64) -> Result<Imported> {
        let mut tree = self.folder_tree()?;
        let dates: HashMap<&[String], i64> =
            folders.iter().filter_map(|f| f.added.map(|a| (f.path.as_slice(), a))).collect();
        let tx = self.db.transaction()?;
        let mut added = 0;
        let mut filled = HashSet::new();
        {
            let make = |tree: &mut FolderTree, path: &[String]| {
                tree.resolve(path, |parent, title| {
                    let mut full = tree_path(parent, &tx)?;
                    full.push(title.to_owned());
                    let made = dates.get(full.as_slice()).copied().unwrap_or(now);
                    tx.execute(
                        "INSERT INTO folders (parent, title, created) VALUES (?1, ?2, ?3)",
                        &[parent.map(|p| p.0).into(), title.into(), made.into()],
                    )?;
                    Ok::<_, sqlite::Error>(FolderId(tx.last_insert_rowid()))
                })
            };
            for f in folders {
                make(&mut tree, &f.path)?;
            }
            for (i, e) in entries.iter().enumerate() {
                let there = tx.query_row("SELECT 1 FROM bookmarks WHERE url = ?1", &[(&e.url).into()], |_| Ok(()))?;
                if there.is_some() {
                    continue;
                }
                let to = make(&mut tree, &e.folder)?;
                let created = e.added.unwrap_or_else(|| now - i64::try_from(i).unwrap_or(0));
                tx.execute(
                    "INSERT INTO bookmarks (title, url, created, folder) VALUES (?1, ?2, ?3, ?4)",
                    &[non_blank(&e.title, &e.url).into(), (&e.url).into(), created.into(), to.map(|t| t.0).into()],
                )?;
                added += 1;
                if let Some(to) = to {
                    filled.insert(to);
                }
            }
        }
        tx.commit()?;
        Ok(Imported { bookmarks: added, folders: filled.len() })
    }

    // History

    /// The newest `limit` visits.
    ///
    /// # Errors
    ///
    /// A database failure.
    pub fn history(&self, limit: usize) -> Result<Vec<Visit>> {
        Ok(self.db.query_all(
            "SELECT id, title, url, visited FROM history ORDER BY visited DESC, id DESC LIMIT ?1",
            &[limit.into()],
            |r| Ok(Visit { id: r.get(0)?, title: r.get(1)?, url: r.get(2)?, visited: r.get(3)? }),
        )?)
    }

    /// Records a visit, folding a reload or a return to the same page within a minute into one row.
    ///
    /// # Errors
    ///
    /// A database failure.
    pub fn record_visit(&self, title: &str, url: &str, now: i64) -> Result<()> {
        self.forget_most_visited();
        let updated = self.db.execute(
            "UPDATE history SET title = ?1, visited = ?2 WHERE url = ?3 AND visited > ?4",
            &[title.into(), now.into(), url.into(), (now - SAME_VISIT_MS).into()],
        )?;
        if updated == 0 {
            self.db.execute(
                "INSERT INTO history (title, url, visited) VALUES (?1, ?2, ?3)",
                &[title.into(), url.into(), now.into()],
            )?;
        }
        Ok(())
    }

    /// Gives the newest visit of `url` the title the page announced after it loaded.
    ///
    /// # Errors
    ///
    /// A database failure.
    pub fn update_history_title(&self, url: &str, title: &str) -> Result<()> {
        self.forget_most_visited();
        self.db.execute(
            "UPDATE history SET title = ?1 WHERE id = (SELECT id FROM history WHERE url = ?2 ORDER BY visited DESC LIMIT 1)",
            &[title.into(), url.into()],
        )?;
        Ok(())
    }

    /// Removes one visit.
    ///
    /// # Errors
    ///
    /// A database failure.
    pub fn remove_visit(&self, id: i64) -> Result<()> {
        self.forget_most_visited();
        self.db.execute("DELETE FROM history WHERE id = ?1", &[id.into()])?;
        self.erase_log()
    }

    /// Removes all of History.
    ///
    /// # Errors
    ///
    /// A database failure.
    pub fn clear_history(&self) -> Result<()> {
        self.forget_most_visited();
        self.db.execute("DELETE FROM history", &[])?;
        self.erase_log()
    }

    /// Removes the visits made before `cutoff` (milliseconds since 1970, as [`crate::history_days::cutoff`] gives).
    ///
    /// # Errors
    ///
    /// A database failure.
    pub fn prune_history(&self, cutoff: i64) -> Result<()> {
        self.forget_most_visited();
        if self.db.execute("DELETE FROM history WHERE visited < ?1", &[cutoff.into()])? > 0 {
            self.erase_log()?;
        }
        Ok(())
    }

    /// Writes what was just deleted into the database file, where secure delete overwrites it, and empties the
    /// write-ahead log, which holds the old pages until then.
    fn erase_log(&self) -> Result<()> {
        Ok(self.db.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")?)
    }

    /// Sites visited at least `min_visits` times in the last 30 days, grouped by host so that many pages of one site
    /// count as that site, most visited first. Sites already pinned as tiles are left out.
    ///
    /// # Errors
    ///
    /// A database failure.
    pub fn most_visited(&self, limit: usize, min_visits: usize, now: i64) -> Result<Vec<Site>> {
        let kept = self.most_visited.borrow().as_ref().and_then(|kept| {
            let same = kept.limit == limit
                && kept.min_visits == min_visits
                && (kept.at..kept.at + MOST_VISITED_KEPT_MS).contains(&now);
            same.then(|| kept.sites.clone())
        });
        if let Some(sites) = kept {
            return Ok(sites);
        }
        let sites = self.count_most_visited(limit, min_visits, now)?;
        self.most_visited.replace(Some(MostVisited { at: now, limit, min_visits, sites: sites.clone() }));
        Ok(sites)
    }

    fn count_most_visited(&self, limit: usize, min_visits: usize, now: i64) -> Result<Vec<Site>> {
        struct Agg {
            scheme: String,
            latest: i64,
            count: usize,
            root_title: Option<String>,
        }
        let pinned: HashSet<String> = self.tiles()?.iter().filter_map(|t| sites::host_of(&t.url)).collect();
        let mut by_host: Vec<(String, Agg)> = Vec::new();
        let mut index: HashMap<String, usize> = HashMap::new();
        // The start page asks on every Home, and a month of history can be tens of thousands of visits: each row is
        // read in place, and only each new `host[:port]` is parsed as an address (giving the host the rest of Sofa
        // uses), so the cost per visit stays a split and a lookup.
        let mut host_of: HashMap<String, Option<usize>> = HashMap::new();
        let mut stmt =
            self.db.prepare("SELECT url, visited, title FROM history WHERE visited > ?1 ORDER BY visited DESC")?;
        let mut rows = stmt.query(&[(now - 30 * DAY_MS).into()])?;
        while let Some(row) = rows.next()? {
            let Some(url) = row.text(0)? else { continue };
            let Some((scheme, rest)) = url.split_once("://") else { continue };
            if !scheme.eq_ignore_ascii_case("http") && !scheme.eq_ignore_ascii_case("https") {
                continue;
            }
            let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
            let (authority, tail) = rest.split_at(end);
            let i = match host_of.get(authority) {
                Some(i) => *i,
                None => {
                    let host = url::Url::parse(&format!("{scheme}://{authority}/"))
                        .ok()
                        .and_then(|u| u.host_str().map(str::to_ascii_lowercase));
                    let i = host.map(|host| {
                        *index.entry(host.clone()).or_insert_with(|| {
                            let latest = row.get::<i64>(1).unwrap_or(0);
                            let scheme = scheme.to_ascii_lowercase();
                            by_host.push((host, Agg { scheme, latest, count: 0, root_title: None }));
                            by_host.len() - 1
                        })
                    });
                    host_of.insert(authority.to_owned(), i);
                    i
                }
            };
            let Some(i) = i else { continue };
            let agg = &mut by_host[i].1;
            agg.count += 1;
            // The site's own front page (path "/", any query) names it.
            let front = matches!(tail, "" | "/") || ["/?", "/#", "?", "#"].iter().any(|p| tail.starts_with(p));
            if agg.root_title.is_none() && front {
                agg.root_title = Some(row.get(2)?);
            }
        }
        by_host.retain(|(host, agg)| {
            agg.count >= min_visits && !pinned.contains(host.strip_prefix("www.").unwrap_or(host))
        });
        by_host.sort_by(|a, b| b.1.count.cmp(&a.1.count).then(b.1.latest.cmp(&a.1.latest)));
        Ok(by_host
            .into_iter()
            .take(limit)
            .zip(1_i64..)
            .map(|((host, agg), n)| Site {
                id: -n,
                title: agg.root_title.filter(|t| !t.trim().is_empty()).unwrap_or_else(|| sites::pretty_host(&host)),
                url: format!("{}://{host}/", agg.scheme),
            })
            .collect())
    }

    /// Bookmarks and History matching `text` (in the address or the title), for the search screen: bookmarks first,
    /// then the most recent visits.
    ///
    /// # Errors
    ///
    /// A database failure.
    pub fn suggest(&self, text: &str, limit: usize) -> Result<Vec<Site>> {
        let text = text.trim();
        if text.is_empty() {
            return Ok(Vec::new());
        }
        // `%`, `_` and `\` are matched as typed, not as LIKE wildcards.
        let escaped = text.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_");
        let like = format!("%{escaped}%");
        self.sites(
            "SELECT id, title, url FROM (
               SELECT id, title, url, 2 AS rank, created AS t FROM bookmarks
                 WHERE url LIKE ?1 ESCAPE '\\' OR title LIKE ?1 ESCAPE '\\'
               UNION ALL
               SELECT MAX(id), title, url, 1 AS rank, MAX(visited) AS t FROM history
                 WHERE url LIKE ?1 ESCAPE '\\' OR title LIKE ?1 ESCAPE '\\' GROUP BY url
             ) GROUP BY url ORDER BY MAX(rank) DESC, MAX(t) DESC LIMIT ?2",
            &[(&like).into(), limit.into()],
        )
    }

    fn sites(&self, sql: &str, values: &[Value<'_>]) -> Result<Vec<Site>> {
        Ok(self.db.query_all(sql, values, |r| Ok(Site { id: r.get(0)?, title: r.get(1)?, url: r.get(2)? }))?)
    }
}

/// The folder names from the top to `id`, read inside a transaction (folders made a moment ago included).
fn tree_path(id: Option<FolderId>, tx: &Connection) -> sqlite::Result<Vec<String>> {
    let mut names = Vec::new();
    let mut at = id;
    while let Some(f) = at {
        if names.len() > crate::folders::MAX_DEPTH {
            break;
        }
        let (title, parent): (String, Option<i64>) =
            tx.query_one("SELECT title, parent FROM folders WHERE id = ?1", &[f.0.into()], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })?;
        names.push(title);
        at = parent.map(FolderId);
    }
    names.reverse();
    Ok(names)
}

fn non_blank<'a>(title: &'a str, url: &'a str) -> &'a str {
    if title.trim().is_empty() { url } else { title }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 1_800_000_000_000;

    fn lib() -> Library {
        Library::in_memory().expect("an in-memory library")
    }

    fn entry(title: &str, url: &str, folder: &[&str]) -> Entry {
        Entry {
            title: title.into(),
            url: url.into(),
            added: None,
            folder: folder.iter().map(|s| (*s).into()).collect(),
        }
    }

    #[test]
    fn a_new_library_has_the_default_tiles() {
        let l = lib();
        let titles: Vec<String> = l.tiles().expect("tiles").into_iter().map(|t| t.title).collect();
        assert_eq!(titles, ["YouTube", "Crunchyroll", "IGN", "Plex", "Wikipedia"]);
    }

    #[test]
    fn tiles_reorder_and_go() -> Result<()> {
        let mut l = lib();
        l.add_tile("", "https://example.com/")?;
        let ids: Vec<i64> = l.tiles()?.iter().map(|t| t.id).rev().collect();
        l.reorder_tiles(&ids)?;
        let tiles = l.tiles()?;
        assert_eq!(tiles[0].title, "https://example.com/");
        l.remove_tile(tiles[0].id)?;
        assert_eq!(l.tiles()?.len(), DEFAULT_TILES.len());
        Ok(())
    }

    #[test]
    fn bookmarks_keep_their_folder_and_newest_comes_first() -> Result<()> {
        let mut l = lib();
        l.add_bookmark("A", "https://a.example/", NOW)?;
        l.add_bookmark("B", "https://b.example/", NOW + 1)?;
        l.add_bookmark("Again", "https://a.example/", NOW + 2)?;
        let top: Vec<String> = l.bookmarks(None, 0)?.into_iter().map(|b| b.title).collect();
        assert_eq!(top, ["B", "A"]);
        let news = l.add_folder(None, "News", NOW)?;
        let a = l.bookmarks(None, 0)?.into_iter().find(|b| b.title == "A").map(|b| b.id).expect("A");
        l.move_bookmark(a, Some(news))?;
        assert_eq!(l.bookmarks(Some(news), 0)?.len(), 1);
        assert_eq!(l.bookmark_counts()?.get(&None), Some(&1));
        l.delete_folder(news)?;
        assert!(!l.is_bookmarked("https://a.example/")?);
        assert!(l.folder_tree()?.is_empty());
        Ok(())
    }

    #[test]
    fn importing_twice_adds_nothing_the_second_time() -> Result<()> {
        let mut l = lib();
        let entries = [
            entry("A", "https://a.example/", &["News", "Tech"]),
            entry("B", "https://b.example/", &[]),
            entry("C", "https://c.example/", &["news"]),
        ];
        let folders = [FolderEntry { path: vec!["Empty".into()], added: Some(5) }];
        let first = l.import_bookmarks(&entries, &folders, NOW)?;
        assert_eq!(first, Imported { bookmarks: 3, folders: 2 });
        let tree = l.folder_tree()?;
        assert_eq!(tree.len(), 3);
        assert_eq!(tree.find(None, "Empty").map(|f| f.created), Some(5));
        let second = l.import_bookmarks(&entries, &folders, NOW)?;
        assert_eq!(second, Imported::default());
        assert_eq!(l.folder_tree()?.len(), 3);
        // Undated entries keep the file's order: the first one shows first.
        let top: Vec<String> = l.bookmarks(None, 0)?.into_iter().map(|b| b.title).collect();
        assert_eq!(top, ["B"]);
        let exported = l.bookmarks_to_export()?;
        assert_eq!(exported[0].folder, ["News", "Tech"]);
        assert_eq!(l.folders_to_export()?.len(), 3);
        Ok(())
    }

    #[test]
    fn an_exported_file_imports_with_its_empty_folders_and_dates() -> Result<()> {
        let from = lib();
        let news = from.add_folder(None, "News", 1_000_000)?;
        from.add_folder(Some(news), "Empty", 2_000_000)?;
        from.add_bookmark("A", "https://a.example/", 3_000_000)?;
        let a = from.bookmarks(None, 0)?[0].id;
        from.move_bookmark(a, Some(news))?;
        let file = crate::bookmark_file::write(&from.bookmarks_to_export()?, &from.folders_to_export()?);

        let contents = crate::bookmark_file::read(&file, NOW);
        let mut to = lib();
        let done = to.import_bookmarks(&contents.entries, &contents.folders, NOW)?;

        assert_eq!(done, Imported { bookmarks: 1, folders: 1 });
        let tree = to.folder_tree()?;
        let news = tree.find(None, "News").expect("News");
        assert_eq!(news.created, 1_000_000);
        assert_eq!(tree.find(Some(news.id), "Empty").map(|f| f.created), Some(2_000_000));
        Ok(())
    }

    #[test]
    fn visits_within_a_minute_fold_into_one() -> Result<()> {
        let l = lib();
        l.record_visit("Home", "https://example.com/", NOW)?;
        l.record_visit("Home again", "https://example.com/", NOW + 10_000)?;
        l.record_visit("Later", "https://example.com/", NOW + 120_000)?;
        let h = l.history(10)?;
        assert_eq!(h.len(), 2);
        assert_eq!(h[0].title, "Later");
        assert_eq!(h[1].title, "Home again");
        l.update_history_title("https://example.com/", "Titled")?;
        assert_eq!(l.history(1)?[0].title, "Titled");
        l.remove_visit(h[1].id)?;
        assert_eq!(l.history(10)?.len(), 1);
        l.clear_history()?;
        assert!(l.history(10)?.is_empty());
        Ok(())
    }

    #[test]
    fn most_visited_groups_by_host_and_skips_tiles() -> Result<()> {
        let l = lib();
        for i in 0..3 {
            let at = NOW - i * 120_000;
            l.record_visit("Page", &format!("https://news.example/p{i}"), at)?;
            l.record_visit("YT", "https://www.youtube.com/watch", at)?;
        }
        l.record_visit("Once", "https://once.example/", NOW)?;
        l.record_visit("Old", "https://old.example/", NOW - 40 * DAY_MS)?;
        let mv = l.most_visited(10, 3, NOW)?;
        assert_eq!(mv.len(), 1);
        assert_eq!(mv[0].url, "https://news.example/");
        assert_eq!(mv[0].title, "News");
        Ok(())
    }

    #[test]
    fn most_visited_is_kept_until_history_or_the_tiles_change() -> Result<()> {
        let l = lib();
        let visit = |n: i64, url: &str| l.record_visit("Page", url, NOW - n * 120_000);
        for n in 0..3 {
            visit(n, &format!("https://a.example/{n}"))?;
        }
        assert_eq!(l.most_visited(10, 3, NOW)?.len(), 1);
        // A row written behind the library's back doesn't show while the list is kept…
        for n in 0..3 {
            l.db.execute(
                "INSERT INTO history (title, url, visited) VALUES ('B', ?1, ?2)",
                &[format!("https://b.example/{n}").as_str().into(), (NOW - n * 120_000).into()],
            )?;
        }
        assert_eq!(l.most_visited(10, 3, NOW + 1)?.len(), 1);
        // …but after an hour, or any change through the library, the list is counted again.
        assert_eq!(l.most_visited(10, 3, NOW + MOST_VISITED_KEPT_MS)?.len(), 2);
        visit(9, "https://c.example/")?;
        l.add_tile("A", "https://a.example/")?;
        let urls: Vec<String> = l.most_visited(10, 3, NOW)?.into_iter().map(|s| s.url).collect();
        assert_eq!(urls, ["https://b.example/"]);
        l.clear_history()?;
        assert!(l.most_visited(10, 3, NOW)?.is_empty());
        Ok(())
    }

    #[test]
    fn most_visited_merges_one_host_and_titles_it_from_its_newest_front_page() -> Result<()> {
        let l = lib();
        // One site under several spellings: case, a port, http then https, a front page with a query.
        l.record_visit("Old front", "http://Video.example/", NOW - 9 * DAY_MS)?;
        l.record_visit("Clip", "https://video.example:443/watch?v=1", NOW - 8 * DAY_MS)?;
        l.record_visit("New front", "https://video.example/?lang=en", NOW - 7 * DAY_MS)?;
        l.record_visit("Clip 2", "https://VIDEO.example/watch?v=2", NOW - DAY_MS)?;
        // As many visits but older ones: second. Never on its front page: named after its host.
        for i in 0..4 {
            l.record_visit("Article", &format!("https://www.paper.example/a/{i}"), NOW - 2 * DAY_MS - i * 120_000)?;
        }
        l.record_visit("Not web", "file:///x.html", NOW)?;
        let mv = l.most_visited(10, 3, NOW)?;
        let got: Vec<(&str, &str)> = mv.iter().map(|s| (s.title.as_str(), s.url.as_str())).collect();
        assert_eq!(got, [("New front", "https://video.example/"), ("Paper", "https://www.paper.example/")]);
        assert_eq!(l.most_visited(1, 3, NOW)?.len(), 1);
        assert!(l.most_visited(10, 5, NOW)?.is_empty());
        Ok(())
    }

    #[test]
    fn suggestions_put_bookmarks_first() -> Result<()> {
        let l = lib();
        l.record_visit("Example visit", "https://example.com/v", NOW)?;
        l.add_bookmark("Example mark", "https://example.com/b", NOW)?;
        let s: Vec<String> = l.suggest("exam", 6)?.into_iter().map(|s| s.title).collect();
        assert_eq!(s, ["Example mark", "Example visit"]);
        assert!(l.suggest("  ", 6)?.is_empty());
        // Wildcard characters match only themselves.
        assert!(l.suggest("%", 6)?.is_empty());
        l.record_visit("Mine", "https://my_site.example/", NOW)?;
        l.record_visit("Other", "https://myxsite.example/", NOW)?;
        let s: Vec<String> = l.suggest("my_site", 6)?.into_iter().map(|s| s.title).collect();
        assert_eq!(s, ["Mine"]);
        Ok(())
    }

    #[test]
    fn a_library_file_reopens_with_its_data() -> Result<()> {
        let dir = std::env::temp_dir().join(format!("sofa-lib-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a test folder");
        let path = dir.join("library.db");
        {
            let l = Library::open(&path)?;
            l.add_bookmark("A", "https://a.example/", NOW)?;
            l.remove_tile(l.tiles()?[0].id)?;
        }
        let l = Library::open(&path)?;
        assert!(l.is_bookmarked("https://a.example/")?);
        assert_eq!(l.tiles()?.len(), DEFAULT_TILES.len() - 1);
        drop(l);
        let _ = std::fs::remove_dir_all(&dir);
        Ok(())
    }

    #[test]
    fn pruning_removes_only_the_visits_before_the_cutoff() -> Result<()> {
        let l = lib();
        l.record_visit("Old", "https://old.example/", NOW - 91 * DAY_MS)?;
        l.record_visit("Recent", "https://recent.example/", NOW - 89 * DAY_MS)?;
        l.prune_history(crate::history_days::cutoff(90, NOW).unwrap_or(i64::MIN))?;
        let left: Vec<String> = l.history(10)?.into_iter().map(|v| v.title).collect();
        assert_eq!(left, ["Recent"]);
        l.prune_history(crate::history_days::cutoff(crate::history_days::NEVER, NOW).unwrap_or(i64::MIN))?;
        assert!(l.history(10)?.is_empty());
        Ok(())
    }

    #[test]
    fn a_removed_visit_is_gone_from_the_files() -> Result<()> {
        let dir = std::env::temp_dir().join(format!("sofa-lib-erase-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a test folder");
        let path = dir.join("library.db");
        let marker = "https://erase-marker-5f2c.example/";
        let found = |dir: &Path| {
            std::fs::read_dir(dir).expect("the test folder").flatten().any(|f| {
                let bytes = std::fs::read(f.path()).unwrap_or_default();
                bytes.windows(marker.len()).any(|w| w == marker.as_bytes())
            })
        };
        let l = Library::open(&path)?;
        l.record_visit("Kept", "https://kept.example/", NOW)?;
        l.record_visit("Marker", marker, NOW)?;
        l.db.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")?;
        assert!(found(&dir), "the marker should be in the file before it is removed");
        let id = l.history(10)?.into_iter().find(|v| v.url == marker).map(|v| v.id).expect("the marker visit");
        l.remove_visit(id)?;
        assert!(!found(&dir), "a removed visit left its address in the files");
        l.record_visit("Marker", marker, NOW)?;
        l.clear_history()?;
        assert!(!found(&dir), "a cleared history left an address in the files");
        drop(l);
        let _ = std::fs::remove_dir_all(&dir);
        Ok(())
    }
}
