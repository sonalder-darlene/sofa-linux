// SPDX-License-Identifier: GPL-3.0-only
//
// The thin C++ layer: only what QtWebEngine needs done in C++. It forwards decisions to the Rust core and makes
// none itself.

#pragma once

#include <QtCore/QString>
#include <QtQml/QQmlApplicationEngine>

#include "rust/cxx.h"

namespace sofa {

struct RequestPolicy; // Rust type, see src/shell.rs

// Sets up QtWebEngine; must run before the application object exists.
void initWebEngine();

// Creates the persistent web profile named `storageName`, routes every request through `policy` (in that profile and
// in private ones), runs `pageScript` at the start of every frame of their pages in a world of its own, and hands
// the profile, the Host helper and the start options to the QML root object as its initial properties. Returns false when another Sofa already runs and took the start address (this one then
// quits).
bool prepareEngine(QQmlApplicationEngine &engine, const QString &storageName, rust::Box<RequestPolicy> policy,
                   const QString &startUrl, bool fullScreen, const QString &pageScript);

// Whether loading the QML gave a root object (a window).
bool hasWindow(const QQmlApplicationEngine &engine);

// "Qt 6.x.y, QtWebEngine 6.x.y, Chromium a.b.c.d (security patches up to e.f.g.h)".
QString engineVersions();

// The Qt platform plugin in use, e.g. "wayland" or "xcb".
QString platformName();

} // namespace sofa
