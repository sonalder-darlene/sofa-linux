//! Bookmark folders as a tree: what is in a folder, its path, and what may move where.
//!
//! The tree holds folders only (bookmarks stay in the [`crate::library`] database), so even thousands of folders are a
//! small list. Names are matched without regard to case: "News" and "news" in one folder would look the same on a TV.

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet, VecDeque};

/// A folder's id in the database.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FolderId(pub i64);

/// A bookmarks folder; `parent` is `None` at the top level.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Folder {
    /// Its id.
    pub id: FolderId,
    /// The folder it is in, `None` at the top.
    pub parent: Option<FolderId>,
    /// Its name.
    pub title: String,
    /// When it was made, in milliseconds since 1970.
    pub created: i64,
}

/// Folders nest this deep at most; a file's deeper folders go into the one at this depth.
pub const MAX_DEPTH: usize = 20;

/// A folder name is at most this many characters long.
pub const MAX_NAME: usize = 200;

/// The folders, indexed by id and by parent.
#[derive(Debug, Default, Clone)]
pub struct FolderTree {
    by_id: HashMap<FolderId, Folder>,
    children: HashMap<Option<FolderId>, Vec<FolderId>>,
}

impl FolderTree {
    /// The tree of `folders`.
    #[must_use]
    pub fn new(folders: impl IntoIterator<Item = Folder>) -> Self {
        let mut tree = Self::default();
        for f in folders {
            tree.add(f);
        }
        tree
    }

    /// How many folders there are.
    #[must_use]
    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    /// Whether there are none.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }

    /// The folder `id`, if it is there.
    #[must_use]
    pub fn get(&self, id: Option<FolderId>) -> Option<&Folder> {
        id.and_then(|id| self.by_id.get(&id))
    }

    /// The folders in `parent`, A to Z.
    #[must_use]
    pub fn children(&self, parent: Option<FolderId>) -> Vec<&Folder> {
        let mut out: Vec<&Folder> =
            self.children.get(&parent).into_iter().flatten().filter_map(|id| self.by_id.get(id)).collect();
        out.sort_by(|a, b| by_name(&a.title, &b.title));
        out
    }

    /// The folders from the top down to `id`, `id` last; empty for the top level or a folder that isn't there.
    #[must_use]
    pub fn path(&self, id: Option<FolderId>) -> Vec<&Folder> {
        let mut out = Vec::new();
        let mut f = self.get(id);
        // A loop can only come from a damaged database; it stops at the size of the tree.
        while let Some(folder) = f {
            if out.len() > self.by_id.len() {
                break;
            }
            out.push(folder);
            f = self.get(folder.parent);
        }
        out.reverse();
        out
    }

    /// How deep `id` sits: 0 for the top level, 1 for a folder at the top.
    #[must_use]
    pub fn depth(&self, id: Option<FolderId>) -> usize {
        self.path(id).len()
    }

    /// `id` and every folder inside it, at any depth.
    #[must_use]
    pub fn subtree(&self, id: FolderId) -> Vec<FolderId> {
        let mut seen = HashSet::new();
        let mut out = Vec::new();
        let mut todo = VecDeque::from([id]);
        while let Some(f) = todo.pop_front() {
            if seen.insert(f) {
                out.push(f);
                todo.extend(self.children.get(&Some(f)).into_iter().flatten().copied());
            }
        }
        out
    }

    /// How many levels `id` spans: 1 for a folder with no folders in it.
    fn height(&self, id: FolderId) -> usize {
        fn h(tree: &FolderTree, f: FolderId, seen: &mut HashSet<FolderId>) -> usize {
            if !seen.insert(f) {
                return 0;
            }
            1 + tree.children.get(&Some(f)).into_iter().flatten().map(|c| h(tree, *c, seen)).max().unwrap_or(0)
        }
        h(self, id, &mut HashSet::new())
    }

    /// The folder called `title` in `parent`, whatever its case.
    #[must_use]
    pub fn find(&self, parent: Option<FolderId>, title: &str) -> Option<&Folder> {
        let title = title.trim().to_lowercase();
        self.children
            .get(&parent)
            .into_iter()
            .flatten()
            .filter_map(|id| self.by_id.get(id))
            .find(|f| f.title.to_lowercase() == title)
    }

    /// Whether a new folder can go in `parent`: it exists and isn't already at the deepest level.
    #[must_use]
    pub fn can_hold(&self, parent: Option<FolderId>) -> bool {
        (parent.is_none() || self.get(parent).is_some()) && self.depth(parent) < MAX_DEPTH
    }

    /// Whether folder `id` can move into `to`: never into itself or a folder inside it, never past the deepest level.
    #[must_use]
    pub fn can_move(&self, id: FolderId, to: Option<FolderId>) -> bool {
        self.by_id.contains_key(&id)
            && to.is_none_or(|t| self.by_id.contains_key(&t) && !self.subtree(id).contains(&t))
            && self.depth(to) + self.height(id) <= MAX_DEPTH
    }

    /// Every folder depth first, each folder's own folders A to Z under it, with its depth (1 at the top).
    #[must_use]
    pub fn outline(&self) -> Vec<(&Folder, usize)> {
        fn walk<'a>(tree: &'a FolderTree, parent: Option<FolderId>, depth: usize, out: &mut Vec<(&'a Folder, usize)>) {
            for f in tree.children(parent) {
                out.push((f, depth));
                if depth < MAX_DEPTH {
                    walk(tree, Some(f.id), depth + 1, out);
                }
            }
        }
        let mut out = Vec::with_capacity(self.by_id.len());
        walk(self, None, 1, &mut out);
        out
    }

    /// The folder at `path` (names from the top), made where missing with `make`, which stores a folder in `parent`
    /// and gives its id. A folder already there with the same name is used, so the same path twice gives the same
    /// folder. Names are cleaned as a typed name is; an empty one is skipped, and a path deeper than [`MAX_DEPTH`]
    /// ends at that depth.
    ///
    /// # Errors
    ///
    /// Whatever `make` returns.
    pub fn resolve<E>(
        &mut self,
        path: &[String],
        mut make: impl FnMut(Option<FolderId>, &str) -> Result<FolderId, E>,
    ) -> Result<Option<FolderId>, E> {
        let mut at = None;
        for raw in path {
            let name = clean_name(raw);
            if name.is_empty() {
                continue;
            }
            if self.depth(at) >= MAX_DEPTH {
                break;
            }
            at = Some(match self.find(at, &name) {
                Some(f) => f.id,
                None => {
                    let id = make(at, &name)?;
                    self.add(Folder { id, parent: at, title: name, created: 0 });
                    id
                }
            });
        }
        Ok(at)
    }

    /// A folder made or changed after the tree was read.
    pub fn add(&mut self, f: Folder) {
        self.remove(f.id);
        self.children.entry(f.parent).or_default().push(f.id);
        self.by_id.insert(f.id, f);
    }

    /// Takes folder `id` out of the tree (not what is inside it).
    pub fn remove(&mut self, id: FolderId) {
        if let Some(old) = self.by_id.remove(&id) {
            if let Some(list) = self.children.get_mut(&old.parent) {
                list.retain(|c| *c != id);
            }
        }
    }

    /// The names along `id`'s path, from the top: the folder as a bookmarks file has it.
    #[must_use]
    pub fn path_names(&self, id: Option<FolderId>) -> Vec<String> {
        self.path(id).into_iter().map(|f| f.title.clone()).collect()
    }
}

/// A to Z as people read it: accents and case don't split the list (`"école"` sorts with the e's), then the plain
/// text decides, so the order is stable.
#[must_use]
pub fn by_name(a: &str, b: &str) -> Ordering {
    let fold = |s: &str| s.chars().flat_map(fold_char).collect::<String>();
    fold(a).cmp(&fold(b)).then_with(|| a.cmp(b))
}

/// `c` in lower case without its accent, for the Latin letters names use most.
fn fold_char(c: char) -> impl Iterator<Item = char> {
    let base = match c {
        'À'..='Å' | 'à'..='å' | 'Ā' | 'ā' | 'Ă' | 'ă' | 'Ą' | 'ą' => 'a',
        'Ç' | 'ç' | 'Ć' | 'ć' | 'Č' | 'č' => 'c',
        'Ď' | 'ď' | 'Đ' | 'đ' => 'd',
        'È'..='Ë' | 'è'..='ë' | 'Ē' | 'ē' | 'Ė' | 'ė' | 'Ę' | 'ę' | 'Ě' | 'ě' => 'e',
        'Ì'..='Ï' | 'ì'..='ï' | 'Ī' | 'ī' | 'Į' | 'į' | 'İ' | 'ı' => 'i',
        'Ñ' | 'ñ' | 'Ń' | 'ń' | 'Ň' | 'ň' => 'n',
        'Ò'..='Ö' | 'Ø' | 'ò'..='ö' | 'ø' | 'Ō' | 'ō' | 'Ő' | 'ő' => 'o',
        'Ś' | 'ś' | 'Š' | 'š' | 'Ş' | 'ş' => 's',
        'Ť' | 'ť' | 'Ţ' | 'ţ' => 't',
        'Ù'..='Ü' | 'ù'..='ü' | 'Ū' | 'ū' | 'Ů' | 'ů' | 'Ű' | 'ű' | 'Ų' | 'ų' => 'u',
        'Ý' | 'ý' | 'ÿ' | 'Ÿ' => 'y',
        'Ź' | 'ź' | 'Ż' | 'ż' | 'Ž' | 'ž' => 'z',
        'Ł' | 'ł' => 'l',
        _ => return Fold::Lower(c.to_lowercase()),
    };
    Fold::One(Some(base))
}

/// Either a folded letter or the lower case of any other character.
enum Fold {
    One(Option<char>),
    Lower(std::char::ToLowercase),
}

impl Iterator for Fold {
    type Item = char;

    fn next(&mut self) -> Option<char> {
        match self {
            Self::One(c) => c.take(),
            Self::Lower(it) => it.next(),
        }
    }
}

/// A name on one line, without spaces around it, at most [`MAX_NAME`] characters.
#[must_use]
pub fn clean_name(name: &str) -> String {
    let one_line = name.split_whitespace().collect::<Vec<_>>().join(" ");
    one_line.chars().take(MAX_NAME).collect::<String>().trim().to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn folder(id: i64, parent: Option<i64>, title: &str) -> Folder {
        Folder { id: FolderId(id), parent: parent.map(FolderId), title: title.to_owned(), created: 0 }
    }

    fn sample() -> FolderTree {
        FolderTree::new([
            folder(1, None, "News"),
            folder(2, Some(1), "Tech"),
            folder(3, Some(2), "Gadgets"),
            folder(4, None, "école"),
            folder(5, None, "Zoo"),
        ])
    }

    #[test]
    fn children_sort_a_to_z_without_accents_or_case() {
        let tree = sample();
        let names: Vec<&str> = tree.children(None).iter().map(|f| f.title.as_str()).collect();
        assert_eq!(names, ["école", "News", "Zoo"]);
    }

    #[test]
    fn paths_and_depths_follow_parents() {
        let tree = sample();
        assert_eq!(tree.path_names(Some(FolderId(3))), ["News", "Tech", "Gadgets"]);
        assert_eq!(tree.depth(Some(FolderId(3))), 3);
        assert_eq!(tree.depth(None), 0);
        assert!(tree.path(Some(FolderId(99))).is_empty());
    }

    #[test]
    fn a_folder_never_moves_into_itself() {
        let tree = sample();
        assert!(!tree.can_move(FolderId(1), Some(FolderId(3))));
        assert!(!tree.can_move(FolderId(1), Some(FolderId(1))));
        assert!(tree.can_move(FolderId(3), None));
        assert!(tree.can_move(FolderId(5), Some(FolderId(3))));
        assert_eq!(tree.subtree(FolderId(1)), [FolderId(1), FolderId(2), FolderId(3)]);
    }

    #[test]
    fn a_damaged_loop_doesnt_hang() {
        let tree = FolderTree::new([folder(1, Some(2), "A"), folder(2, Some(1), "B")]);
        assert!(tree.path(Some(FolderId(1))).len() <= 3);
        assert!(tree.subtree(FolderId(1)).len() == 2);
    }

    #[test]
    fn resolve_reuses_folders_by_name_and_makes_the_rest() {
        let mut tree = sample();
        let mut next = 10;
        let mut made = Vec::new();
        let path = ["news".to_owned(), "  New   one ".to_owned(), String::new()];
        let got = tree
            .resolve(&path, |parent, name| -> Result<FolderId, ()> {
                made.push((parent, name.to_owned()));
                next += 1;
                Ok(FolderId(next))
            })
            .expect("no error");
        assert_eq!(made, [(Some(FolderId(1)), "New one".to_owned())]);
        assert_eq!(got, Some(FolderId(11)));
        // The same path again makes nothing.
        let again = tree.resolve(&path, |_, _| -> Result<FolderId, ()> { Err(()) }).expect("nothing made");
        assert_eq!(again, Some(FolderId(11)));
    }

    #[test]
    fn deep_paths_stop_at_the_deepest_level() {
        let mut tree = FolderTree::default();
        let path: Vec<String> = (0..30).map(|i| format!("F{i}")).collect();
        let mut next = 0;
        let got = tree
            .resolve(&path, |_, _| -> Result<FolderId, ()> {
                next += 1;
                Ok(FolderId(next))
            })
            .expect("no error");
        assert_eq!(tree.depth(got), MAX_DEPTH);
        assert!(!tree.can_hold(got));
    }

    #[test]
    fn outline_lists_depth_first() {
        let tree = sample();
        let outline: Vec<(&str, usize)> = tree.outline().iter().map(|(f, d)| (f.title.as_str(), *d)).collect();
        assert_eq!(outline, [("école", 1), ("News", 1), ("Tech", 2), ("Gadgets", 3), ("Zoo", 1)]);
    }

    #[test]
    fn names_are_cleaned() {
        assert_eq!(clean_name("  a \n b\t "), "a b");
        assert_eq!(clean_name(&"x".repeat(300)).len(), MAX_NAME);
    }
}
