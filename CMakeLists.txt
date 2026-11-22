# SPDX-License-Identifier: GPL-3.0-only
#
# CMake runs Cargo (which builds the Rust core, the CXX-Qt bridge, the thin C++ layer and the QML module into one
# binary) and installs the binary with its .desktop file, icon and AppStream metadata. `./sofa.sh` wraps it.

cmake_minimum_required(VERSION 3.22)

# The version lives in Cargo.toml, the app ID in APP_ID: one place each.
file(READ "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml" _cargo_toml)
string(REGEX MATCH "\nversion = \"([0-9]+\\.[0-9]+\\.[0-9]+)\"" _ "${_cargo_toml}")
set(_sofa_version "${CMAKE_MATCH_1}")
file(STRINGS "${CMAKE_CURRENT_SOURCE_DIR}/APP_ID" _sofa_app_id LIMIT_COUNT 1)
string(STRIP "${_sofa_app_id}" _sofa_app_id)

project(sofa-tv-browser VERSION ${_sofa_version} LANGUAGES CXX) # Cargo compiles the C++ parts; Qt's CMake files need the language enabled

set(SOFA_APP_ID "${_sofa_app_id}" CACHE STRING "Reverse-DNS app ID (Wayland app ID, .desktop and data folder name)")
set(SOFA_BINARY sofa-tv-browser)
set(SOFA_HOMEPAGE "https://github.com/sonalder-darlene/sofa-linux" CACHE STRING "Project home page")
option(SOFA_CARGO_OFFLINE "Build only from vendored or already downloaded crates (no network)" OFF)

include(GNUInstallDirs)

# Fail here, with package names, rather than deep inside Cargo.
find_package(Qt6 REQUIRED COMPONENTS Core Gui DBus Qml Quick WebEngineQuick Svg)
# Qt 6.11 is what Plasma Bigscreen and KDE's TV components build on; older web engines also miss a year of fixes.
set(SOFA_QT_MIN 6.11)
if(Qt6_VERSION VERSION_LESS SOFA_QT_MIN)
    message(FATAL_ERROR "Sofa needs Qt ${SOFA_QT_MIN} or newer; this system has Qt ${Qt6_VERSION}. "
        "Use the Flatpak instead (./sofa.sh flatpak), which brings a current Qt and web engine, or Nix (nix run .).")
endif()
# KDE's components for TV apps (org.kde.bigscreen.ui), which Sofa's screens are made of. When the system has none,
# Sofa builds them from their release tag and installs them in a folder of its own, which only Sofa looks in.
set(SOFA_BIGSCREEN_UI_TAG v0.1.0)
set(SOFA_BIGSCREEN_UI AUTO CACHE STRING "Build KDE's bigscreen-ui with Sofa: AUTO (when the system has none), ON or OFF")
set(SOFA_BIGSCREEN_UI_SOURCE "" CACHE PATH "A bigscreen-ui source tree to build instead of downloading it")
set(_bigscreen_ui_search "${QT6_INSTALL_PREFIX}/${QT6_INSTALL_QML}")
foreach(_var QML_IMPORT_PATH QML2_IMPORT_PATH)
    if(DEFINED ENV{${_var}})
        string(REPLACE ":" ";" _paths "$ENV{${_var}}")
        list(APPEND _bigscreen_ui_search ${_paths})
    endif()
endforeach()
find_file(SOFA_SYSTEM_BIGSCREEN_UI qmldir PATHS ${_bigscreen_ui_search} PATH_SUFFIXES org/kde/bigscreen/ui NO_DEFAULT_PATH)
if(SOFA_BIGSCREEN_UI STREQUAL "ON" OR (SOFA_BIGSCREEN_UI STREQUAL "AUTO" AND NOT SOFA_SYSTEM_BIGSCREEN_UI))
    message(STATUS "Building KDE's bigscreen-ui ${SOFA_BIGSCREEN_UI_TAG} with Sofa")
    include(FetchContent)
    if(SOFA_BIGSCREEN_UI_SOURCE)
        FetchContent_Declare(bigscreen_ui SOURCE_DIR "${SOFA_BIGSCREEN_UI_SOURCE}")
    else()
        FetchContent_Declare(bigscreen_ui
            GIT_REPOSITORY https://invent.kde.org/libraries/bigscreen-ui.git
            GIT_TAG 2ab509c16a38ab87a066a35aeb3e7aa4c4cafe5a # ${SOFA_BIGSCREEN_UI_TAG}
            GIT_SHALLOW FALSE)
    endif()
    # Next to Sofa, not where the system's QML modules go: a distribution package of it later doesn't collide.
    set(KDE_INSTALL_QMLDIR "lib/${SOFA_BINARY}/qml" CACHE PATH "" FORCE)
    set(BUILD_EXAMPLES OFF CACHE BOOL "" FORCE)
    FetchContent_MakeAvailable(bigscreen_ui)
elseif(NOT SOFA_SYSTEM_BIGSCREEN_UI)
    message(WARNING "KDE's bigscreen-ui (QML module org.kde.bigscreen.ui) was not found; Sofa won't start without it. "
        "Install it, or configure with -DSOFA_BIGSCREEN_UI=AUTO.")
endif()

find_program(CARGO cargo REQUIRED)
get_target_property(_qmake Qt6::qmake IMPORTED_LOCATION)
if(DEFINED ENV{QMAKE})
    set(_qmake "$ENV{QMAKE}")
endif()

if(CMAKE_BUILD_TYPE STREQUAL "Debug")
    set(_cargo_profile_flag "")
    set(_cargo_out_dir debug)
else()
    set(_cargo_profile_flag "--release")
    set(_cargo_out_dir release)
endif()

set(_cargo_target_dir "${CMAKE_CURRENT_BINARY_DIR}/cargo")
set(_sofa_binary_path "${_cargo_target_dir}/${_cargo_out_dir}/${SOFA_BINARY}")
set(_cargo_flags --locked)
if(SOFA_CARGO_OFFLINE)
    list(APPEND _cargo_flags --offline)
endif()

# Cargo decides itself what needs rebuilding, so this runs on every build and is quick when nothing changed.
add_custom_target(sofa ALL
    COMMAND ${CMAKE_COMMAND} -E env
        "QMAKE=${_qmake}"
        "SOFA_APP_ID=${SOFA_APP_ID}"
        "CARGO_TARGET_DIR=${_cargo_target_dir}"
        ${CARGO} build ${_cargo_profile_flag} ${_cargo_flags} --package sofa-tv-browser
    WORKING_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}"
    BYPRODUCTS "${_sofa_binary_path}"
    COMMENT "Building Sofa with Cargo"
    USES_TERMINAL)

configure_file(data/sofa.desktop.in "${CMAKE_CURRENT_BINARY_DIR}/${SOFA_APP_ID}.desktop" @ONLY)
configure_file(data/sofa.metainfo.xml.in "${CMAKE_CURRENT_BINARY_DIR}/${SOFA_APP_ID}.metainfo.xml" @ONLY)

install(PROGRAMS "${_sofa_binary_path}" DESTINATION ${CMAKE_INSTALL_BINDIR})
install(FILES "${CMAKE_CURRENT_BINARY_DIR}/${SOFA_APP_ID}.desktop" DESTINATION ${CMAKE_INSTALL_DATADIR}/applications)
install(FILES "${CMAKE_CURRENT_BINARY_DIR}/${SOFA_APP_ID}.metainfo.xml" DESTINATION ${CMAKE_INSTALL_DATADIR}/metainfo)
install(FILES data/icon.svg DESTINATION ${CMAKE_INSTALL_DATADIR}/icons/hicolor/scalable/apps RENAME ${SOFA_APP_ID}.svg)

include(CTest)
if(BUILD_TESTING)
    add_test(NAME cargo-test
        COMMAND ${CMAKE_COMMAND} -E env "QMAKE=${_qmake}" "SOFA_APP_ID=${SOFA_APP_ID}"
            "CARGO_TARGET_DIR=${_cargo_target_dir}" ${CARGO} test ${_cargo_profile_flag} ${_cargo_flags} --workspace
        WORKING_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}")
    find_program(DESKTOP_FILE_VALIDATE desktop-file-validate)
    if(DESKTOP_FILE_VALIDATE)
        add_test(NAME desktop-file COMMAND ${DESKTOP_FILE_VALIDATE} "${CMAKE_CURRENT_BINARY_DIR}/${SOFA_APP_ID}.desktop")
    endif()
    find_program(APPSTREAMCLI appstreamcli)
    if(APPSTREAMCLI)
        add_test(NAME metainfo
            COMMAND ${APPSTREAMCLI} validate --no-net --explain "${CMAKE_CURRENT_BINARY_DIR}/${SOFA_APP_ID}.metainfo.xml")
    endif()
endif()
