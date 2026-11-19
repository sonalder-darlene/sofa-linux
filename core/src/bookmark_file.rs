//! The bookmarks file every desktop browser imports and exports: the "Netscape bookmark file", an HTML page of
//! `<A HREF>` links in nested `<DL>` lists, each folder named by the `<H3>` before its list.
//!
//! Reading keeps each link's folder as a path of names; writing gives the same tree back, which browsers and Sofa
//! (on Android too) import as it is. The file comes from anywhere, so the reader is a small hand-written scanner with
//! limits on everything, not an HTML parser.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::Write as _;

use crate::folders::{self, MAX_DEPTH};
use crate::sites::is_web_url;

/// A bookmark as the file has it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// Its title (the address when the file gives none).
    pub title: String,
    /// Its address.
    pub url: String,
    /// When it was added, in milliseconds since 1970; `None` when the file gives no date.
    pub added: Option<i64>,
    /// The folder names it sits in, from the top; empty at the top.
    pub folder: Vec<String>,
}

/// A folder to write even when nothing is in it: its path from the top, and when it was made.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FolderEntry {
    /// The names from the top.
    pub path: Vec<String>,
    /// When it was made, in milliseconds since 1970.
    pub added: Option<i64>,
}

/// What a file held: its web links in file order (the first copy of each address), its folders, and how many links
/// were left out.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Contents {
    /// The bookmarks.
    pub entries: Vec<Entry>,
    /// Every folder taken, in file order, with the date its `<H3>` gives; empty ones too.
    pub folders: Vec<FolderEntry>,
    /// Links that weren't web addresses, copies, or past the limits.
    pub skipped: usize,
}

/// Larger files are refused before they are read; a real bookmarks file is far smaller.
pub const MAX_BYTES: usize = 8 * 1024 * 1024;
/// At most this many bookmarks are taken from one file.
pub const MAX_ENTRIES: usize = 20_000;
/// At most this many folders are taken from one file; what is in any after them goes to the folder above.
pub const MAX_FOLDERS: usize = 2_000;
const MAX_TITLE: usize = 500;

/// The bookmarks in `html`, each with its folder; `now` (milliseconds) caps dates in the future.
///
/// Links that aren't web addresses (`javascript:`, `place:`, `file:`, a browser's own pages) are counted in
/// [`Contents::skipped`], and so are copies of an address already taken. The browser's bookmarks bar (the top folder
/// marked `PERSONAL_TOOLBAR_FOLDER`) is opened up: what is in it goes to the top, as it is what the browser showed
/// first. A folder without a name is opened up into the one above it. Folders are listed in [`Contents::folders`]
/// even when empty, so a file Sofa wrote gives back the same tree with the same dates.
#[must_use]
pub fn read(html: &str, now: i64) -> Contents {
    let mut out = Contents::default();
    let mut seen = HashSet::new();
    let mut path: Vec<String> = Vec::new();
    // For each open list, whether it added a name to the path (the file's outer list doesn't).
    let mut named: Vec<bool> = Vec::new();
    // A folder's name and date wait for the list that follows it.
    let mut pending: Option<(String, Option<i64>)> = None;
    let mut folders: HashSet<Vec<String>> = HashSet::new();

    let mut at = 0;
    while let Some(tag) = next_tag(html, at) {
        at = tag.end;
        match (tag.name.as_str(), tag.closing) {
            ("a", false) => {
                pending = None;
                let (text, after) = text_until_end(html, tag.end, "a");
                at = after;
                let url = attribute(tag.attrs, "href").map(|u| decode(&u).trim().to_owned());
                let Some(url) = url.filter(|u| is_web_url(u) && seen.insert(u.clone())) else {
                    out.skipped += 1;
                    continue;
                };
                if out.entries.len() >= MAX_ENTRIES {
                    out.skipped += 1;
                    continue;
                }
                let title: String = clean_text(text).chars().take(MAX_TITLE).collect();
                let added = attribute(tag.attrs, "add_date").and_then(|d| date(&d, now));
                let title = if title.is_empty() { url.clone() } else { title };
                out.entries.push(Entry { title, url, added, folder: path.clone() });
            }
            ("h3", false) => {
                let (text, after) = text_until_end(html, tag.end, "h3");
                at = after;
                let toolbar = path.is_empty()
                    && attribute(tag.attrs, "personal_toolbar_folder").is_some_and(|v| v.eq_ignore_ascii_case("true"));
                let name = if toolbar { String::new() } else { folders::clean_name(&clean_text(text)) };
                pending = Some((name, attribute(tag.attrs, "add_date").and_then(|d| date(&d, now))));
            }
            ("dl", false) => {
                let name = pending.take();
                let opens = match name {
                    Some((name, added)) if !name.is_empty() && path.len() < MAX_DEPTH => {
                        let mut candidate = path.clone();
                        candidate.push(name);
                        if folders.contains(&candidate) || folders.len() < MAX_FOLDERS {
                            if folders.insert(candidate.clone()) {
                                out.folders.push(FolderEntry { path: candidate.clone(), added });
                            }
                            path = candidate;
                            true
                        } else {
                            false
                        }
                    }
                    _ => false,
                };
                named.push(opens);
            }
            ("dl", true) => {
                pending = None;
                if named.pop() == Some(true) {
                    path.pop();
                }
            }
            _ => {}
        }
    }
    out
}

/// `entries` as a bookmarks file that Chrome, Firefox, Edge, Safari and Sofa read: each folder an `<H3>` followed by
/// its list, folders first (A to Z, as Sofa shows them), then the folder's bookmarks in the order given. `folders`
/// adds the dates folders were made, and the folders with nothing in them.
#[must_use]
pub fn write(entries: &[Entry], folders: &[FolderEntry]) -> String {
    #[derive(Default)]
    struct Node<'a> {
        added: Option<i64>,
        children: BTreeMap<String, Node<'a>>,
        entries: Vec<&'a Entry>,
    }
    fn node<'a, 'b>(root: &'b mut Node<'a>, path: &[String]) -> &'b mut Node<'a> {
        path.iter().fold(root, |n, name| n.children.entry(name.clone()).or_default())
    }
    fn list(out: &mut String, n: &Node<'_>, indent: &str) {
        out.push_str(indent);
        out.push_str("<DL><p>\n");
        let inner = format!("{indent}    ");
        let mut names: Vec<&String> = n.children.keys().collect();
        names.sort_by(|a, b| folders::by_name(a, b));
        for name in names {
            let child = &n.children[name];
            out.push_str(&inner);
            out.push_str("<DT><H3");
            if let Some(added) = child.added {
                let _ = write!(out, " ADD_DATE=\"{}\"", added / 1000);
            }
            let _ = writeln!(out, ">{}</H3>", escape(name));
            list(out, child, &inner);
        }
        for e in &n.entries {
            let _ = write!(out, "{inner}<DT><A HREF=\"{}\"", escape(&e.url));
            if let Some(added) = e.added {
                let _ = write!(out, " ADD_DATE=\"{}\"", added / 1000);
            }
            let _ = writeln!(out, ">{}</A>", escape(&e.title));
        }
        out.push_str(indent);
        out.push_str("</DL><p>\n");
    }

    let mut root = Node::default();
    for f in folders.iter().filter(|f| !f.path.is_empty()) {
        node(&mut root, &f.path).added = f.added;
    }
    for e in entries {
        node(&mut root, &e.folder).entries.push(e);
    }
    let mut out = String::from(
        "<!DOCTYPE NETSCAPE-Bookmark-file-1>\n<!-- This is an automatically generated file.\n     It will be read and \
         overwritten.\n     DO NOT EDIT! -->\n<META HTTP-EQUIV=\"Content-Type\" CONTENT=\"text/html; \
         charset=UTF-8\">\n<TITLE>Bookmarks</TITLE>\n<H1>Bookmarks</H1>\n",
    );
    list(&mut out, &root, "");
    out
}

/// One tag: its lower-case name, whether it closes (`</dl>`), its attribute text and where it ends.
struct Tag<'a> {
    name: String,
    closing: bool,
    attrs: &'a str,
    start: usize,
    end: usize,
}

/// The next tag at or after byte `from`, skipping comments.
fn next_tag(html: &str, from: usize) -> Option<Tag<'_>> {
    let bytes = html.as_bytes();
    let mut i = from;
    while let Some(off) = html.get(i..)?.find('<') {
        let start = i + off;
        if html[start..].starts_with("<!--") {
            i = html[start..].find("-->").map_or(html.len(), |e| start + e + 3);
            continue;
        }
        let mut j = start + 1;
        let closing = bytes.get(j) == Some(&b'/');
        if closing {
            j += 1;
        }
        let name_start = j;
        while j < bytes.len() && bytes[j].is_ascii_alphanumeric() {
            j += 1;
        }
        if j == name_start {
            i = start + 1;
            continue;
        }
        let name = html[name_start..j].to_ascii_lowercase();
        // Attribute values may hold '>' only inside quotes.
        let mut k = j;
        let mut quote = None;
        while k < bytes.len() {
            match (quote, bytes[k]) {
                (None, b'"' | b'\'') => quote = Some(bytes[k]),
                (Some(q), b) if b == q => quote = None,
                (None, b'>') => break,
                _ => {}
            }
            k += 1;
        }
        let end = (k + 1).min(html.len());
        return Some(Tag { name, closing, attrs: &html[j..k.min(html.len())], start, end });
    }
    None
}

/// The text after an opening `<a>` or `<h3>` up to its closing tag, or up to the next link, folder or list line when
/// a file forgets to close it; and where reading goes on.
fn text_until_end<'a>(html: &'a str, from: usize, name: &str) -> (&'a str, usize) {
    let mut at = from;
    while let Some(tag) = next_tag(html, at) {
        if tag.closing && tag.name == name {
            return (&html[from..tag.start], tag.end);
        }
        let ends = matches!((tag.name.as_str(), tag.closing), ("a" | "dt" | "h3", false) | ("dl", _));
        if ends {
            return (&html[from..tag.start], tag.start);
        }
        at = tag.end;
    }
    (&html[from..], html.len())
}

/// Text between tags as one line: inner tags dropped, entities decoded, runs of white space made one space.
fn clean_text(html: &str) -> String {
    let mut plain = String::with_capacity(html.len());
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' if in_tag => in_tag = false,
            c if !in_tag => plain.push(c),
            _ => {}
        }
    }
    decode(&plain).split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The value of attribute `name` in a tag's attribute text: double, single or no quotes.
fn attribute(attrs: &str, name: &str) -> Option<String> {
    let bytes = attrs.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        let key_start = i;
        while i < bytes.len() && !bytes[i].is_ascii_whitespace() && bytes[i] != b'=' {
            i += 1;
        }
        let key = &attrs[key_start..i];
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        let value = if bytes.get(i) == Some(&b'=') {
            i += 1;
            while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            match bytes.get(i) {
                Some(&q @ (b'"' | b'\'')) => {
                    let start = i + 1;
                    let end = attrs[start..].find(char::from(q)).map_or(attrs.len(), |e| start + e);
                    i = (end + 1).min(attrs.len());
                    &attrs[start..end]
                }
                _ => {
                    let start = i;
                    while i < bytes.len() && !bytes[i].is_ascii_whitespace() {
                        i += 1;
                    }
                    &attrs[start..i]
                }
            }
        } else {
            ""
        };
        if key.eq_ignore_ascii_case(name) {
            return Some(value.to_owned());
        }
        if i == key_start {
            i += 1;
        }
    }
    None
}

/// `ADD_DATE` is in seconds; a few tools write milliseconds or microseconds, told apart by size.
fn date(value: &str, now: i64) -> Option<i64> {
    let n: i64 = value.trim().parse().ok().filter(|n| *n > 0)?;
    let ms = if n >= 100_000_000_000_000 {
        n / 1000
    } else if n >= 100_000_000_000 {
        n
    } else {
        n.saturating_mul(1000)
    };
    Some(ms.min(now))
}

/// Decodes `&amp;`, `&#38;`, `&#x26;` and the few named entities hand-made files use.
fn decode(text: &str) -> String {
    if !text.contains('&') {
        return text.to_owned();
    }
    let named: HashMap<&str, &str> = NAMED.iter().copied().collect();
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(amp) = rest.find('&') {
        out.push_str(&rest[..amp]);
        let after = &rest[amp + 1..];
        let decoded = after.find(';').filter(|semi| *semi <= 9).and_then(|semi| {
            let e = &after[..semi];
            let c = if let Some(hex) = e.strip_prefix("#x").or_else(|| e.strip_prefix("#X")) {
                u32::from_str_radix(hex, 16).ok().and_then(char::from_u32).filter(|c| *c != '\0').map(String::from)
            } else if let Some(dec) = e.strip_prefix('#') {
                dec.parse::<u32>().ok().and_then(char::from_u32).filter(|c| *c != '\0').map(String::from)
            } else {
                named.get(e.to_ascii_lowercase().as_str()).map(|s| (*s).to_owned())
            };
            c.map(|c| (c, semi))
        });
        match decoded {
            Some((c, semi)) => {
                out.push_str(&c);
                rest = &after[semi + 1..];
            }
            None => {
                out.push('&');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

/// Browsers write only the first five; the rest turn up in files made by hand.
const NAMED: &[(&str, &str)] = &[
    ("amp", "&"),
    ("lt", "<"),
    ("gt", ">"),
    ("quot", "\""),
    ("apos", "'"),
    ("nbsp", " "),
    ("ndash", "\u{2013}"),
    ("mdash", "\u{2014}"),
    ("hellip", "\u{2026}"),
    ("lsquo", "\u{2018}"),
    ("rsquo", "\u{2019}"),
    ("ldquo", "\u{201c}"),
    ("rdquo", "\u{201d}"),
    ("laquo", "\u{ab}"),
    ("raquo", "\u{bb}"),
    ("copy", "\u{a9}"),
    ("reg", "\u{ae}"),
    ("trade", "\u{2122}"),
];

fn escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 1_800_000_000_000;

    /// The shape Chromium-based browsers write (folders, the toolbar folder, icons, entities).
    const CHROMIUM: &str = r#"<!DOCTYPE NETSCAPE-Bookmark-file-1>
<!-- This is an automatically generated file.
     It will be read and overwritten.
     DO NOT EDIT! -->
<META HTTP-EQUIV="Content-Type" CONTENT="text/html; charset=UTF-8">
<TITLE>Bookmarks</TITLE>
<H1>Bookmarks</H1>
<DL><p>
    <DT><H3 ADD_DATE="1700000000" LAST_MODIFIED="0" PERSONAL_TOOLBAR_FOLDER="true">Bookmarks bar</H3>
    <DL><p>
        <DT><A HREF="https://example.com/" ADD_DATE="1700000001" ICON="data:image/png;base64,AAAA">Example &amp; Co</A>
        <DT><H3 ADD_DATE="1700000002">News</H3>
        <DL><p>
            <DT><A HREF="https://news.example.org/a?b=1&amp;c=2" ADD_DATE="1700000003">Today &ndash; news</A>
            <DT><A HREF="javascript:void(0)">A bookmarklet</A>
        </DL><p>
    </DL><p>
    <DT><H3>Other bookmarks</H3>
    <DL><p>
        <DT><A HREF='http://192.168.1.1/' ADD_DATE=1700000004>Router</A>
        <DT><A HREF="https://example.com/">A copy</A>
    </DL><p>
</DL><p>
"#;

    #[test]
    fn a_chromium_file_keeps_folders_and_opens_the_toolbar() {
        let c = read(CHROMIUM, NOW);
        assert_eq!(c.skipped, 2);
        let got: Vec<(&str, &str, Vec<String>)> =
            c.entries.iter().map(|e| (e.title.as_str(), e.url.as_str(), e.folder.clone())).collect();
        assert_eq!(
            got,
            [
                ("Example & Co", "https://example.com/", vec![]),
                ("Today \u{2013} news", "https://news.example.org/a?b=1&c=2", vec!["News".to_owned()]),
                ("Router", "http://192.168.1.1/", vec!["Other bookmarks".to_owned()]),
            ]
        );
        assert_eq!(c.entries[0].added, Some(1_700_000_001_000));
        // The toolbar folder is opened up, so it isn't a folder; the others are, with their dates.
        assert_eq!(
            c.folders,
            [
                FolderEntry { path: vec!["News".into()], added: Some(1_700_000_002_000) },
                FolderEntry { path: vec!["Other bookmarks".into()], added: None },
            ]
        );
    }

    #[test]
    fn writing_then_reading_gives_the_same_tree() {
        let entries = vec![
            Entry { title: "A <b>".into(), url: "https://a.example/".into(), added: Some(1_000_000), folder: vec![] },
            Entry {
                title: "B \"q\"".into(),
                url: "https://b.example/?x=1&y=2".into(),
                added: Some(2_000_000),
                folder: vec!["News".into(), "Tech".into()],
            },
        ];
        let folders = [
            FolderEntry { path: vec!["Empty".into()], added: Some(5_000) },
            FolderEntry { path: vec!["News".into()], added: Some(6_000) },
            FolderEntry { path: vec!["News".into(), "Tech".into()], added: Some(7_000) },
        ];
        let file = write(&entries, &folders);
        assert!(file.contains("<H3 ADD_DATE=\"5\">Empty</H3>"));
        let mut back = read(&file, NOW);
        // Folders come first in the file, so only the order differs.
        back.entries.sort_by(|a, b| a.url.cmp(&b.url));
        assert_eq!(back.entries, entries);
        assert_eq!(back.folders, folders);
        assert_eq!(back.skipped, 0);
    }

    #[test]
    fn unclosed_links_and_odd_attributes_still_read() {
        let html =
            "<DL><DT><a href=https://x.example/ add_date=\"x\">One<DT><A\nHREF=\"https://y.example/\">Two</a></DL>";
        let c = read(html, NOW);
        let titles: Vec<&str> = c.entries.iter().map(|e| e.title.as_str()).collect();
        assert_eq!(titles, ["One", "Two"]);
        assert_eq!(c.entries[0].added, None);
    }

    #[test]
    fn dates_in_other_units_and_the_future_are_read_sensibly() {
        assert_eq!(date("1700000000", NOW), Some(1_700_000_000_000));
        assert_eq!(date("1700000000000", NOW), Some(1_700_000_000_000));
        assert_eq!(date("1700000000000000", NOW), Some(1_700_000_000_000));
        assert_eq!(date("9999999999", NOW), Some(NOW));
        assert_eq!(date("-5", NOW), None);
    }

    #[test]
    fn entities_decode_safely() {
        assert_eq!(decode("a &amp; b &#60; &#x3E; &bogus; &#0; &"), "a & b < > &bogus; &#0; &");
        assert_eq!(decode("&HELLIP;"), "\u{2026}");
    }

    #[test]
    fn an_empty_or_foreign_file_gives_nothing() {
        assert_eq!(read("", NOW), Contents::default());
        assert_eq!(read("<html><body><p>Hello</p></body></html>", NOW), Contents::default());
    }

    #[test]
    fn addresses_without_a_host_are_skipped() {
        let html = r#"<DL><DT><A HREF="https://">A</A>
            <DT><A HREF="https://a b/">C</A><DT><A HREF="https://ok.example/">D</A></DL>"#;
        let c = read(html, NOW);
        let urls: Vec<&str> = c.entries.iter().map(|e| e.url.as_str()).collect();
        assert_eq!(urls, ["https://ok.example/"]);
        assert_eq!(c.skipped, 2);
    }

    #[test]
    fn the_entry_limit_holds() {
        let mut html = String::from("<DL>");
        for i in 0..MAX_ENTRIES + 5 {
            let _ = write!(html, "<DT><A HREF=\"https://e.example/{i}\">{i}</A>");
        }
        let c = read(&html, NOW);
        assert_eq!(c.entries.len(), MAX_ENTRIES);
        assert_eq!(c.skipped, 5);
    }

    #[test]
    fn nameless_folders_open_into_the_one_above() {
        let html = "<DL><DT><H3>A</H3><DL><DT><H3></H3><DL><DT><A HREF=\"https://x.example/\">X</A></DL></DL></DL>";
        let c = read(html, NOW);
        assert_eq!(c.entries[0].folder, ["A"]);
        assert_eq!(c.folders, [FolderEntry { path: vec!["A".into()], added: None }]);
    }

    #[test]
    fn a_folder_named_twice_is_listed_once_and_the_folder_limit_holds() {
        let mut html =
            String::from("<DL><DT><H3 ADD_DATE=\"10\">A</H3><DL></DL><DT><H3 ADD_DATE=\"20\">A</H3><DL></DL>");
        for i in 0..MAX_FOLDERS + 5 {
            let _ = write!(html, "<DT><H3>F{i}</H3><DL></DL>");
        }
        html.push_str("</DL>");
        let c = read(&html, NOW);
        assert_eq!(c.folders.len(), MAX_FOLDERS);
        assert_eq!(c.folders[0], FolderEntry { path: vec!["A".into()], added: Some(10_000) });
    }

    #[test]
    fn comments_and_quoted_brackets_dont_confuse_the_scanner() {
        let html = "<!-- <A HREF=\"https://no.example/\">no</A> --><DL><DT><A HREF=\"https://y.example/?a>b\" X='>'>Yes</A></DL>";
        let c = read(html, NOW);
        assert_eq!(c.entries.len(), 1);
        assert_eq!(c.entries[0].url, "https://y.example/?a>b");
    }
}
