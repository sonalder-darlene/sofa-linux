# Third-party notices

Sofa for Linux itself is licensed under the GNU General Public License, version 3 only (see LICENSE).

## Material Symbols (icon)

The sofa glyph in `data/icon.svg` comes from [Material Symbols](https://github.com/google/material-design-icons) by
Google, licensed under the Apache License, Version 2.0 (see LICENSE-APACHE-2.0.txt).

## Filter lists

Sofa carries two domain lists in `data/filters/`, unchanged apart from keeping only the domains, each with its
source and licence on its first lines:

- [StevenBlack's hosts](https://github.com/StevenBlack/hosts), MIT, Copyright (c) Steven Black
- [HaGeZi's Pop-Up Ads list](https://github.com/hagezi/dns-blocklists), GPL-3.0, HaGeZi

## Rust crates

Sofa links these crates into its program, each under its own licence (listed in `Cargo.lock`; all of them are
compatible with the GPL-3.0):

- [CXX-Qt](https://github.com/KDAB/cxx-qt) (`cxx-qt`, `cxx-qt-lib`, `cxx-qt-build`), MIT or Apache-2.0
- [CXX](https://github.com/dtolnay/cxx), MIT or Apache-2.0
- [url](https://github.com/servo/rust-url) and its dependencies (`idna`, `percent-encoding`, ICU4X crates), MIT or
  Apache-2.0, ICU4X under Unicode-3.0
- [serde_json](https://github.com/serde-rs/json) and `serde`, MIT or Apache-2.0
- [adblock-rust](https://github.com/brave/adblock-rust) (`adblock`) by Brave, MPL-2.0, with its dependencies
  (`addr`, `psl`, `psl-types`, `arrayvec`, `base64`, `either`, `itertools`, `regex`, `regex-automata`,
  `regex-syntax`, `rustc-hash`, `rustc_version`), MIT or Apache-2.0; `flatbuffers`, Apache-2.0; `aho-corasick`,
  Unlicense or MIT; `seahash`, MIT

## Used, not included

Qt (LGPL-3.0), Qt WebEngine (LGPL-3.0, with Chromium's licences), KDE's Kirigami (LGPL-2.0-or-later) and SQLite
(public domain) come from your system or the Flatpak runtime and are not part of this source tree. KDE's
[bigscreen-ui](https://invent.kde.org/libraries/bigscreen-ui) (LGPL-2.0-or-later) comes from your system when it has
it; otherwise the build, the Flatpak and the Nix flake fetch its release tag and build it next to Sofa, unchanged. Sofa reaches
SQLite through a short binding of its own (`core/src/sqlite.rs`).
