# Third-party notices

Sofa for Linux itself is licensed under the GNU General Public License, version 3 only (see LICENSE).

## Material Symbols (icon)

The sofa glyph in `data/icon.svg` comes from [Material Symbols](https://github.com/google/material-design-icons) by
Google, licensed under the Apache License, Version 2.0 (see LICENSE-APACHE-2.0.txt).

## Rust crates

Sofa links these crates into its program, each under its own licence (listed in `Cargo.lock`; all of them are
compatible with the GPL-3.0):

- [CXX-Qt](https://github.com/KDAB/cxx-qt) (`cxx-qt`, `cxx-qt-lib`, `cxx-qt-build`), MIT or Apache-2.0
- [CXX](https://github.com/dtolnay/cxx), MIT or Apache-2.0
- [url](https://github.com/servo/rust-url) and its dependencies (`idna`, `percent-encoding`, ICU4X crates), MIT or
  Apache-2.0, ICU4X under Unicode-3.0

## Used, not included

Qt (LGPL-3.0), Qt WebEngine (LGPL-3.0, with Chromium's licences) and KDE's Kirigami (LGPL-2.0-or-later) come from
your system or the Flatpak runtime and are not part of this source tree.
