# Sofa for Linux

A web browser for the TV, made for [KDE Plasma Bigscreen](https://plasma-bigscreen.org/) and driven from the couch
with a TV remote, a game pad or a keyboard. It is the Linux sibling of
[Sofa TV Browser](https://github.com/sonalder-darlene/sofa-tv-browser) for Android TV: the same ideas and choices,
in a new code base made for Linux.

**This is an early version.** What it does today:

- **A remote is all you need.** On Sofa's own screens the arrows move the focus; on a page they move a pointer, and
  the page scrolls when the pointer reaches an edge. OK clicks; OK held on a link opens Sofa's link menu (open in a
  new tab, open, bookmark); OK held anywhere else scrolls with the arrows.
- **A start page** with your Quick access tiles (add, edit, move, remove), the sites you visit most and your
  bookmarks, and a search field that suggests from your bookmarks and history.
- **Tabs**, up to ten, in Sofa's menu. A tab out of sight is frozen, so it can't play or open anything; past the few
  your computer's memory keeps ready, and after ten minutes out of sight, it is unloaded and comes back with its
  history when you choose it. Your tabs are there again the next time Sofa starts.
- **Bookmarks in folders**, with import and export as the bookmarks file every desktop browser reads and writes.
- **History**, by day, kept on this computer only.
- **Private browsing** in a session of its own: nothing goes to History, and its cookies and site data are gone when
  you leave it.
- **Open at start:** the start page, a page you choose, or where you left off.
- **Ads and trackers blocked** (Settings → Block ads and trackers, on from the start): the pictures, scripts and
  frames of about 150,000 ad, tracker and pop-up domains are never loaded, in private browsing too. Sofa carries the
  lists itself ([StevenBlack's hosts](https://github.com/StevenBlack/hosts) and
  [HaGeZi's pop-up ad domains](https://github.com/hagezi/dns-blocklists)), so blocking needs no download, and checks
  each request with [Brave's ad-block engine](https://github.com/brave/adblock-rust). A site that doesn't work
  with it can be let through from Sofa's menu (Ad block); in private browsing that lasts until you leave it.
- Pages get no camera, microphone, location or notifications.

Pop-up rules, more filter lists, site filters and Sofa's own video player come next.

## Install from the source

One script does everything; nothing uses root unless you ask for it.

```sh
git clone https://github.com/sonalder-darlene/sofa-linux.git
cd sofa-linux
./sofa.sh deps            # prints the command that installs what the build needs (--install runs it)
./sofa.sh install         # builds, then installs into ~/.local with an entry in the app list
```

| Command | What it does |
|---|---|
| `./sofa.sh deps [--install] [--yes]` | The build packages for Arch, Fedora, Debian/Ubuntu or openSUSE (NixOS: use the flake) |
| `./sofa.sh build [--debug]` | Builds into `build/` |
| `./sofa.sh run [--fresh] [ADDRESS]` | Runs the build without installing; `--fresh` uses a throwaway profile |
| `./sofa.sh install [--system]` | Installs into `~/.local`, or `/usr/local` with `--system` (uses sudo) |
| `./sofa.sh uninstall [--purge]` | Removes what `install` put there; `--purge` also deletes your Sofa data |
| `./sofa.sh update` | `git pull`, rebuild, reinstall |
| `./sofa.sh flatpak` | Builds the Flatpak and installs it for you |
| `./sofa.sh check` | Prints the Qt and engine versions and which video formats this build plays |

**With Nix:** `nix run github:sonalder-darlene/sofa-linux`, or `nix develop` for a shell with everything the build
needs.

**Needs:** Qt 6.8 or newer with Qt WebEngine, KDE's Kirigami, SQLite, Rust, CMake, pkg-config and lld. The web engine is the one your
distribution (or the Flatpak runtime) ships and keeps patched; Sofa never bundles its own.

**Video formats** depend on how your distribution built Qt WebEngine: `./sofa.sh check` tells you what plays. Paid
streaming services that need Widevine DRM may not play, or play only in low resolution.

## Using it

| Key | Remote / game pad | What it does |
|---|---|---|
| Arrows, OK | D-pad, A | On Sofa's screens: move and choose. On a page: move the pointer and click |
| OK held, Menu | A held, X | On Sofa's screens: what can be done with the item (move, rename, delete…). On a page: Menu opens Sofa's menu |
| Back, Esc | Back / B | Back a step; on a page, leave full-screen video or scroll mode, else open Sofa's menu |
| Home | Home | The start page |
| Page Up / Page Down | Channel up / down | Scroll a page by a screen |
| F5 | | Reload |
| F11 | | Window or full screen |

Sofa's menu over a page has the page's address, your tabs and "New tab", then Home, Back, Forward, Reload, Bookmark,
Add to start page, Bookmarks, History, the pointer switch (off for sites made for arrow keys), zoom and Settings.
OK on another tab switches to it; OK on the tab you are on, or OK held on any tab, closes tabs.

`sofa-tv-browser --windowed` starts in a window; `sofa-tv-browser example.com` opens a page. While Sofa runs, a
second launch hands its address to the running Sofa, which opens it in a new tab.

Your data stays on this computer: bookmarks and history in `~/.local/share/<app ID>/library.db`, settings in
`~/.config/<app ID>/settings.conf`, the open tabs in `~/.local/state/<app ID>/session.conf`.

## Flatpak

The manifest is in [`packaging/flatpak/`](packaging/flatpak/manifest.yml). It builds on the KDE runtime and the
shared Qt WebEngine base, keeps the engine's sandbox on, and asks only for the display, sound, network, the GPU and
your Downloads folder; any other file goes through the file chooser. `./sofa.sh flatpak` builds it with
`flatpak-builder` and Flatpak's
[`flatpak-cargo-generator.py`](https://github.com/flatpak/flatpak-builder-tools/tree/master/cargo), which lists the
Rust crates so the build needs no network. It isn't on Flathub yet.

## How it is made

- **QML and Kirigami** for the screens, so Sofa looks and behaves like a KDE app.
- **A Rust core** (`core/`) for everything that reads untrusted input (what you type, the addresses pages ask for,
  bookmarks files, filter lists, and later backups) and everything Sofa keeps (bookmarks, history, tabs,
  settings). It has no Qt in it and is tested on its own.
- **[CXX-Qt](https://github.com/KDAB/cxx-qt)** (`app/src/`) joins the two, and a **thin C++ layer**
  (`app/cpp/`) does only what Qt WebEngine needs done in C++: it asks the Rust core about every request and decides
  nothing itself.
- **CMake** runs Cargo and installs the files; `./sofa.sh` wraps it.

The app ID in [`APP_ID`](APP_ID) is a placeholder until Sofa has its own domain; it is used for the `.desktop` file,
the Wayland app ID and the data folder (`~/.local/share/<app ID>`).

For development, `SOFA_BLOCK_HOSTS=example.com,example.org sofa-tv-browser` blocks those hosts and everything
under them, to see the request filter at work. The built-in filter lists are in [`data/filters/`](data/filters/);
`data/filters/update.sh` refreshes them before a release.

Sofa is made with the help of an AI assistant (Claude), as the Android app is. Every change is built and tested
before it goes in.

## Licence

Sofa for Linux is free software under the [GNU General Public License v3.0 only](LICENSE) (`GPL-3.0-only`). The
third-party parts are listed in [NOTICE.md](NOTICE.md).
