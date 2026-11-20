#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-only
#
# Build, run and install Sofa for Linux from this source tree. Run `./sofa.sh help` for the commands.
# Nothing here uses root unless you pass --system (or `deps --install`), and it says so before it does.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BUILD_DIR="$ROOT/build"
APP_ID="$(tr -d '[:space:]' < "$ROOT/APP_ID")"
BINARY=sofa-tv-browser
FLATPAK_MANIFEST="$ROOT/packaging/flatpak/manifest.yml"

say() { printf '\033[1m==>\033[0m %s\n' "$*"; }
warn() { printf '\033[1;33mwarning:\033[0m %s\n' "$*" >&2; }
die() { printf '\033[1;31merror:\033[0m %s\n' "$*" >&2; exit 1; }

usage() {
    cat <<EOF
Usage: ./sofa.sh COMMAND [OPTIONS]

  deps [--install] [--yes]   Print (or run, with sudo; --yes asks nothing) the command that installs
                             what the build needs
  build [--debug]            Build into build/ (Release unless --debug)
  run [--fresh] [ARGS]       Run the build without installing; --fresh uses a throwaway profile
  install [--system]         Build if needed, then install to ~/.local (or /usr/local with --system, using sudo)
  uninstall [--purge]        Remove exactly what install put there; --purge also deletes your Sofa data
  update                     git pull, rebuild and reinstall where it was installed
  flatpak [--lint]           Build the Flatpak from packaging/flatpak and install it for this user
  check                      Print the Qt and engine versions and what this build can play
  help                       Show this help
EOF
}

# --- Distribution detection and dependency lists ------------------------------------------------------------------

os_id() {
    local id="" like=""
    if [[ -r /etc/os-release ]]; then
        # shellcheck source=/dev/null
        id="$(. /etc/os-release && echo "${ID:-}")"
        # shellcheck source=/dev/null
        like="$(. /etc/os-release && echo "${ID_LIKE:-}")"
    fi
    case " $id $like " in
        *" steamos "*) echo steamos ;;
        *" nixos "*) echo nixos ;;
        *" arch "*) echo arch ;;
        *" fedora "*) echo fedora ;;
        *" opensuse"*|*" suse "*) echo opensuse ;;
        *" debian "*|*" ubuntu "*) echo debian ;;
        *) echo unknown ;;
    esac
}

# The packages each distribution needs to build and run Sofa. Keep in step with README.md and the CI, which builds
# on each of these in a container.
deps_packages() {
    case "$1" in
        arch) echo "base-devel cmake ninja rust lld sqlite qt6-base qt6-declarative qt6-webengine qt6-svg qt6-wayland kirigami qqc2-desktop-style" ;;
        fedora) echo "cmake ninja-build gcc-c++ cargo rust lld pkgconf-pkg-config sqlite-devel qt6-qtbase-devel qt6-qtdeclarative-devel qt6-qtwebengine-devel qt6-qtsvg-devel qt6-qtwayland kf6-kirigami kf6-qqc2-desktop-style" ;;
        debian)
            # Debian 13's own Rust (1.85) is older than Brave's ad-block engine needs; Debian keeps a newer one for
            # its browsers, rustc-web, which installs as the usual rustc and cargo.
            local rust="cargo rustc"
            apt-cache show rustc-web > /dev/null 2>&1 && rust="cargo-web rustc-web"
            echo "build-essential ca-certificates cmake ninja-build $rust lld pkgconf libsqlite3-dev qt6-base-dev qt6-base-dev-tools qt6-declarative-dev qt6-webengine-dev qt6-svg-dev qt6-svg-plugins qt6-wayland qml6-module-org-kde-kirigami qml6-module-qtwebengine qml6-module-qtquick-controls qml6-module-qtquick-layouts qml6-module-qtquick-window qml6-module-qtquick-templates qml6-module-qtqml-workerscript qml6-module-qtquick-dialogs qml6-module-org-kde-desktop"
            ;;
        opensuse) echo "cmake ninja gcc-c++ cargo lld pkgconf sqlite3-devel qt6-base-devel qt6-declarative-devel qt6-webenginequick-devel qt6-webengine-imports qt6-svg-devel qt6-wayland kf6-kirigami-imports kf6-qqc2-desktop-style" ;;
        *) return 1 ;;
    esac
}

# The install command for distribution $1; with $2 = yes, one that asks nothing (for CI and containers).
deps_command() {
    local os=$1 yes=${2:-no} packages sudo="sudo "
    packages="$(deps_packages "$os")" || return 1
    [[ $EUID -eq 0 ]] && sudo=""
    case "$os" in
        arch) [[ $yes == yes ]] && echo "${sudo}pacman -Syu --needed --noconfirm $packages" || echo "${sudo}pacman -Syu --needed $packages" ;;
        fedora) [[ $yes == yes ]] && echo "${sudo}dnf install -y $packages" || echo "${sudo}dnf install $packages" ;;
        debian) [[ $yes == yes ]] && echo "${sudo}apt-get update && ${sudo}apt-get install -y --no-install-recommends $packages" || echo "${sudo}apt update && ${sudo}apt install $packages" ;;
        opensuse) [[ $yes == yes ]] && echo "${sudo}zypper --non-interactive install $packages" || echo "${sudo}zypper install $packages" ;;
    esac
}

cmd_deps() {
    local install=false yes=no
    for arg in "$@"; do
        case "$arg" in
            --install) install=true ;;
            --yes) yes=yes ;;
            *) die "deps: unknown option '$arg'" ;;
        esac
    done
    local os; os="$(os_id)"
    case "$os" in
        nixos)
            say "On NixOS everything comes from the flake: run 'nix develop' here, then ./sofa.sh build."
            say "Or build and run it straight away: nix run ."
            return 0
            ;;
        steamos)
            say "SteamOS keeps its system read-only, so build packages can't be installed on it. Either:"
            echo "  1. ./sofa.sh flatpak            build and install the Flatpak for this user, or"
            echo "  2. distrobox create --image archlinux --name sofa-build && distrobox enter sofa-build"
            echo "     then ./sofa.sh deps --install && ./sofa.sh install   (for testing only)"
            return 0
            ;;
    esac
    local cmd
    cmd="$(deps_command "$os" "$yes")" || die "I don't know this distribution's package names. You need: CMake, Ninja, a C++ compiler, Rust (cargo), lld, pkg-config, SQLite, Qt 6.8 or newer (Base, Declarative, WebEngine, Svg, Wayland) and KDE's Kirigami. Or use the Flatpak: ./sofa.sh flatpak"
    if $install; then
        say "Installing the build dependencies:"
        echo "  $cmd"
        eval "$cmd"
    else
        say "Install what the build needs with:"
        echo "  $cmd"
        echo "(or run ./sofa.sh deps --install to do it now; it uses sudo)"
    fi
}

# --- Build, run, install ------------------------------------------------------------------------------------------

check_build_tools() {
    local missing=()
    for tool in cmake cargo c++; do
        command -v "$tool" >/dev/null || missing+=("$tool")
    done
    command -v ninja >/dev/null || command -v ninja-build >/dev/null || missing+=("ninja")
    if ((${#missing[@]})); then
        warn "missing: ${missing[*]}"
        cmd_deps
        exit 1
    fi
}

cmd_build() {
    local type=Release
    [[ "${1:-}" == "--debug" ]] && type=Debug
    check_build_tools
    say "Configuring ($type) in build/"
    cmake -S "$ROOT" -B "$BUILD_DIR" -G Ninja -DCMAKE_BUILD_TYPE="$type" -DCMAKE_INSTALL_PREFIX="$HOME/.local" \
        || die "configuring failed; if Qt or a package is missing, ./sofa.sh deps says what to install"
    say "Building"
    cmake --build "$BUILD_DIR"
    say "Built: $(built_binary)"
}

built_binary() {
    local type
    type="$(sed -n 's/^CMAKE_BUILD_TYPE:STRING=//p' "$BUILD_DIR/CMakeCache.txt" 2>/dev/null || true)"
    if [[ "$type" == "Debug" ]]; then
        echo "$BUILD_DIR/cargo/debug/$BINARY"
    else
        echo "$BUILD_DIR/cargo/release/$BINARY"
    fi
}

cmd_run() {
    local fresh=false
    if [[ "${1:-}" == "--fresh" ]]; then fresh=true; shift; fi
    [[ -x "$(built_binary)" ]] || cmd_build
    if $fresh; then
        local tmp; tmp="$(mktemp -d -t sofa-fresh.XXXXXX)"
        trap 'rm -rf "$tmp"' EXIT
        say "Running with a throwaway profile in $tmp"
        XDG_DATA_HOME="$tmp/data" XDG_CONFIG_HOME="$tmp/config" XDG_CACHE_HOME="$tmp/cache" "$(built_binary)" "$@"
    else
        "$(built_binary)" "$@"
    fi
}

refresh_desktop() {
    local prefix=$1
    command -v update-desktop-database >/dev/null && update-desktop-database -q "$prefix/share/applications" 2>/dev/null || true
    # Plasma and Bigscreen read applications through KService; rebuild its cache so Sofa shows without a log-out.
    command -v kbuildsycoca6 >/dev/null && kbuildsycoca6 >/dev/null 2>&1 || true
}

cmd_install() {
    local prefix="$HOME/.local" sudo=()
    if [[ "${1:-}" == "--system" ]]; then
        prefix=/usr/local
        sudo=(sudo)
        say "Installing into $prefix needs root: this will use sudo."
    fi
    if [[ ! -f "$BUILD_DIR/CMakeCache.txt" ]]; then
        cmd_build
    else
        cmake --build "$BUILD_DIR"
    fi
    say "Installing into $prefix"
    "${sudo[@]}" cmake --install "$BUILD_DIR" --prefix "$prefix"
    # Remember where, for update and uninstall.
    echo "$prefix" > "$BUILD_DIR/installed-prefix"
    refresh_desktop "$prefix"
    if [[ "$prefix" == "$HOME/.local" && ":$PATH:" != *":$HOME/.local/bin:"* ]]; then
        warn "$HOME/.local/bin is not on your PATH; Sofa is in your app menu, or run $prefix/bin/$BINARY"
    fi
    say "Installed. Start Sofa from the app list (Plasma Bigscreen shows it on its home screen) or run $BINARY."
}

cmd_uninstall() {
    local purge=false
    [[ "${1:-}" == "--purge" ]] && purge=true
    local manifest="$BUILD_DIR/install_manifest.txt"
    [[ -f "$manifest" ]] || die "nothing to uninstall: build/install_manifest.txt is missing (was Sofa installed from this tree?)"
    local prefix sudo=()
    prefix="$(cat "$BUILD_DIR/installed-prefix" 2>/dev/null || echo "$HOME/.local")"
    [[ "$prefix" == "$HOME"/* ]] || { sudo=(sudo); say "Removing files from $prefix needs root: this will use sudo."; }
    say "Removing the installed files:"
    # The manifest has no newline after its last line.
    while IFS= read -r file || [[ -n "$file" ]]; do
        [[ -n "$file" ]] || continue
        echo "  $file"
        "${sudo[@]}" rm -f -- "$file"
    done < "$manifest"
    rm -f "$manifest" "$BUILD_DIR/installed-prefix"
    refresh_desktop "$prefix"
    if $purge; then
        local data="${XDG_DATA_HOME:-$HOME/.local/share}/$APP_ID" cache="${XDG_CACHE_HOME:-$HOME/.cache}/$APP_ID"
        local config="${XDG_CONFIG_HOME:-$HOME/.config}/$APP_ID"
        say "--purge deletes your Sofa data too (bookmarks, history, cookies, settings):"
        echo "  $data"; echo "  $cache"; echo "  $config"
        read -r -p "Type 'delete' to confirm: " answer
        if [[ "$answer" == "delete" ]]; then
            read -r -p "Sure? This can't be undone [y/N]: " again
            [[ "$again" == [yY] ]] && rm -rf -- "$data" "$cache" "$config" && say "Your Sofa data is deleted."
        else
            say "Kept your data."
        fi
    else
        say "Your Sofa data stays (./sofa.sh uninstall --purge removes it too)."
    fi
}

cmd_update() {
    command -v git >/dev/null || die "git is needed to update"
    say "Fetching the latest code (fast-forward only)"
    git -C "$ROOT" pull --ff-only
    if [[ -f "$BUILD_DIR/installed-prefix" ]]; then
        local prefix; prefix="$(cat "$BUILD_DIR/installed-prefix")"
        if [[ "$prefix" == /usr/local ]]; then cmd_install --system; else cmd_install; fi
    else
        cmd_build
    fi
}

cmd_flatpak() {
    command -v flatpak-builder >/dev/null || die "flatpak-builder is missing: install it with your package manager, or 'flatpak install flathub org.flatpak.Builder' and run this again"
    [[ -f "$FLATPAK_MANIFEST" ]] || die "missing $FLATPAK_MANIFEST"
    local manifest="$BUILD_DIR/flatpak/$APP_ID.yml"
    mkdir -p "$BUILD_DIR/flatpak"
    # The manifest's file name and id follow the app ID.
    sed "s/@APP_ID@/$APP_ID/g" "$FLATPAK_MANIFEST" > "$manifest"
    # The crates, listed from Cargo.lock so that the build needs no network.
    local sources="$BUILD_DIR/flatpak/cargo-sources.json"
    if [[ ! -f "$sources" || "$ROOT/Cargo.lock" -nt "$sources" ]]; then
        # Flatpak's own tool, a single Python script: github.com/flatpak/flatpak-builder-tools, folder cargo/.
        local generator="${SOFA_CARGO_GENERATOR:-$(command -v flatpak-cargo-generator.py || true)}"
        [[ -n "$generator" && -f "$generator" ]] \
            || die "flatpak-cargo-generator.py is missing: put it (from github.com/flatpak/flatpak-builder-tools, folder cargo/) on your PATH or point SOFA_CARGO_GENERATOR at it, then run this again"
        say "Listing the crates from Cargo.lock for the offline build"
        if command -v uv >/dev/null; then
            uv run --script "$generator" "$ROOT/Cargo.lock" -o "$sources"
        else
            python3 "$generator" "$ROOT/Cargo.lock" -o "$sources" \
                || die "the generator needs Python with aiohttp, PyYAML and tomlkit (or install uv, which fetches them)"
        fi
    fi
    if [[ "${1:-}" == "--lint" ]]; then
        command -v flatpak >/dev/null && flatpak run --command=flatpak-builder-lint org.flatpak.Builder manifest "$manifest"
        return
    fi
    say "Building the Flatpak with the same manifest Flathub uses (this downloads the KDE runtime the first time)"
    flatpak-builder --user --install-deps-from=flathub --force-clean --repo="$BUILD_DIR/flatpak/repo" \
        --state-dir="$BUILD_DIR/flatpak/state" "$BUILD_DIR/flatpak/app" "$manifest"
    say "Installing it for this user from the local repository"
    flatpak --user remote-add --if-not-exists --no-gpg-verify sofa-local "$BUILD_DIR/flatpak/repo"
    flatpak --user install -y --reinstall sofa-local "$APP_ID"
    say "Done: flatpak run $APP_ID"
}

cmd_check() {
    [[ -x "$(built_binary)" ]] || cmd_build
    "$(built_binary)" --check
}

main() {
    local cmd="${1:-help}"
    shift || true
    case "$cmd" in
        deps) cmd_deps "$@" ;;
        build) cmd_build "$@" ;;
        run) cmd_run "$@" ;;
        install) cmd_install "$@" ;;
        uninstall) cmd_uninstall "$@" ;;
        update) cmd_update "$@" ;;
        flatpak) cmd_flatpak "$@" ;;
        check) cmd_check "$@" ;;
        help|-h|--help) usage ;;
        *) usage; exit 2 ;;
    esac
}

main "$@"
