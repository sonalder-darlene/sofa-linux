# Sofa for Linux

A web browser for the TV, made for [KDE Plasma Bigscreen](https://plasma-bigscreen.org/) and driven from the couch
with a TV remote, a game pad or a keyboard. It is the Linux sibling of
[Sofa TV Browser](https://github.com/sonalder-darlene/sofa-tv-browser) for Android TV: the same ideas and choices,
in a new code base made for Linux.

**This is an early version.** It opens a web address or a search, goes full screen, and goes back with the remote.
Pages get no camera, microphone, location or notifications. The start page, tabs, bookmarks, ad blocking and Sofa's
own video player come next.

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

**Needs:** Qt 6.8 or newer with Qt WebEngine, KDE's Kirigami, Rust, CMake and lld. The web engine is the one your
distribution (or the Flatpak runtime) ships and keeps patched; Sofa never bundles its own.

**Video formats** depend on how your distribution built Qt WebEngine: `./sofa.sh check` tells you what plays. Paid
streaming services that need Widevine DRM may not play, or play only in low resolution.

## Using it

| Key | Remote / game pad | What it does |
|---|---|---|
| Arrows, OK | D-pad, A | Move and choose; on a page, the arrows scroll |
| Back, Esc | Back / B | Leave full-screen video, hide the address bar, or go back a page |
| Menu, Home, Ctrl+L | Menu / X, Home | Show the address bar |
| F5, Ctrl+R | | Reload |
| F11 | | Window or full screen |

`sofa-tv-browser --windowed` starts in a window; `sofa-tv-browser example.com` opens a page at start.

## Flatpak

The manifest is in [`packaging/flatpak/`](packaging/flatpak/manifest.yml). It builds on the KDE runtime and the
shared Qt WebEngine base, keeps the engine's sandbox on, and asks only for the display, sound, network, the GPU and
your Downloads folder; any other file goes through the file chooser. `./sofa.sh flatpak` builds it with
`flatpak-builder` and Flatpak's
[`flatpak-cargo-generator.py`](https://github.com/flatpak/flatpak-builder-tools/tree/master/cargo), which lists the
Rust crates so the build needs no network. It isn't on Flathub yet.

## How it is made

- **QML and Kirigami** for the screens, so Sofa looks and behaves like a KDE app.
- **A Rust core** (`core/`) for everything that reads untrusted input: what you type, the addresses pages ask for,
  and later filter lists, bookmark and backup files. It has no Qt in it and is tested on its own.
- **[CXX-Qt](https://github.com/KDAB/cxx-qt)** (`app/src/`) joins the two, and a **thin C++ layer**
  (`app/cpp/`) does only what Qt WebEngine needs done in C++: it asks the Rust core about every request and decides
  nothing itself.
- **CMake** runs Cargo and installs the files; `./sofa.sh` wraps it.

The app ID in [`APP_ID`](APP_ID) is a placeholder until Sofa has its own domain; it is used for the `.desktop` file,
the Wayland app ID and the data folder (`~/.local/share/<app ID>`).

For development, `SOFA_BLOCK_HOSTS=example.com,example.org sofa-tv-browser` blocks those hosts and everything
under them, to see the request filter at work.

Sofa is made with the help of an AI assistant (Claude), as the Android app is. Every change is built and tested
before it goes in.

## Licence

Sofa for Linux is free software under the [GNU General Public License v3.0 only](LICENSE) (`GPL-3.0-only`). The
third-party parts are listed in [NOTICE.md](NOTICE.md).
