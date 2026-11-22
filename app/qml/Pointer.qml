// SPDX-License-Identifier: GPL-3.0-only
//
// The pointer over a page, driven by the remote, as on Sofa for Android. Holding a direction speeds the pointer up
// (scaled to the page's width, capped at a 25th of it per 60 Hz frame); letting go stops it at once. Near an edge
// the page scrolls at the pointer's speed. OK clicks where the pointer is; OK held while moving drags; OK held still
// for a second asks the page what is under the pointer: a link or picture gets Sofa's link menu, anywhere else the
// arrows scroll the page until OK or Back. The pointer hides after five seconds unused; the first OK then only
// shows it. Page Up and Page Down (Channel up and down) scroll by a screen.
//
// Every click, move and scroll reaches the page as real mouse and wheel input, so pages treat it as the user's own.
//
// A real pointer (a mouse, a touchpad, a pad's stick that the system turns into a pointer) takes over as soon as it
// is used: this one hides and sends nothing, and the next arrow press brings it back where the real one was. While
// this one is in use, the window hides the system's pointer (Main.qml), so only one shows.

import QtQuick

Item {
    id: pointer

    // The page in sight (a WebEngineView), and the helper that sends it input.
    property Item target: null
    property QtObject host: null

    property bool scrollMode: false
    property real px: width / 2
    property real py: height / 2
    property bool shown: false
    // Whether the remote is the pointer in use (since an arrow or OK), rather than a real pointer.
    property bool driving: false

    // Asks the window for a link menu at the pointer: it replies with answered().
    signal hold(real x, real y)

    readonly property real unit: Math.max(width, 1) / 1920
    readonly property real maxSpeed: width / 25
    readonly property real edge: width / 15
    readonly property int hideAfter: 5000
    readonly property int holdFor: 1000

    property int dirX: 0
    property int dirY: 0
    property real velX: 0
    property real velY: 0
    property bool okHeld: false
    property bool okRevealOnly: false
    property bool dragging: false
    property real pendingScroll: 0
    property bool waitingForAnswer: false

    onWidthChanged: px = Math.min(px, Math.max(width - 1, 0))
    onHeightChanged: py = Math.min(py, Math.max(height - 1, 0))

    function poke() {
        shown = true;
        hideTimer.restart();
    }

    // Hides the pointer until the next key (an overlay opened, say) and stops what it was doing.
    function reset() {
        if (dragging && target && host) {
            host.mouse(target, 2, px, py);
        }
        dragging = false;
        okHeld = false;
        dirX = 0;
        dirY = 0;
        velX = 0;
        velY = 0;
        pendingScroll = 0;
        holdTimer.stop();
        shown = false;
        setScrollMode(false);
    }

    function setScrollMode(on) {
        scrollMode = on;
    }

    // A real pointer was used at (x, y) in the window: step aside, and come back there on the next arrow.
    function realUsed(x, y) {
        if (driving || shown || okHeld || scrollMode) {
            reset();
        }
        driving = false;
        const p = mapFromItem(null, x, y);
        px = Math.max(0, Math.min(width - 1, p.x));
        py = Math.max(0, Math.min(height - 1, p.y));
    }

    // A press landed on the page (this pointer's, a real mouse's or a touch): whether it went into a text field is
    // known a moment later.
    function checkTyping() {
        typingCheck.restart();
    }

    // The window's answer to hold(): whether there was a link or a picture under the pointer.
    function answered(found) {
        if (!waitingForAnswer) {
            return;
        }
        waitingForAnswer = false;
        answerTimer.stop();
        if (!found) {
            setScrollMode(true);
        }
    }

    // Scrolls by `pages` screen heights (negative: up).
    function scrollPages(pages) {
        pendingScroll += pages * height * 0.85;
        frames.running = true;
    }

    // A remote key from the window: returns whether the pointer used it.
    function key(k, pressed) {
        switch (k) {
        case Qt.Key_Left: return direction(-1, 0, pressed);
        case Qt.Key_Right: return direction(1, 0, pressed);
        case Qt.Key_Up: return direction(0, -1, pressed);
        case Qt.Key_Down: return direction(0, 1, pressed);
        case Qt.Key_Return:
        case Qt.Key_Enter:
        case Qt.Key_Select:
            pressed ? okDown() : okUp();
            return true;
        case Qt.Key_PageUp:
        case Qt.Key_ChannelUp:
            if (pressed) scrollPages(-1);
            return true;
        case Qt.Key_PageDown:
        case Qt.Key_ChannelDown:
            if (pressed) scrollPages(1);
            return true;
        }
        return false;
    }

    function direction(dx, dy, pressed) {
        if (host) {
            host.typing = false;
        }
        if (pressed) {
            driving = true;
            if (dx !== 0) dirX = dx;
            if (dy !== 0) dirY = dy;
            poke();
            if (!scrollMode) {
                // A quick tap still moves a few pixels, for fine aiming.
                const ox = px, oy = py;
                px = Math.max(0, Math.min(width - 1, px + dx * 6 * unit));
                py = Math.max(0, Math.min(height - 1, py + dy * 6 * unit));
                moved(ox, oy);
            }
            frames.running = true;
        } else {
            if (dx !== 0) dirX = 0;
            if (dy !== 0) dirY = 0;
            // Stop dead on release; no glide.
            velX = 0;
            velY = 0;
        }
        return true;
    }

    function okDown() {
        driving = true;
        if (scrollMode) {
            setScrollMode(false);
            okRevealOnly = true;
            return;
        }
        if (!shown) {
            okRevealOnly = true;
            poke();
            return;
        }
        okHeld = true;
        poke();
        holdTimer.restart();
    }

    function okUp() {
        holdTimer.stop();
        poke();
        if (okRevealOnly) {
            okRevealOnly = false;
            return;
        }
        if (!okHeld) {
            return;
        }
        okHeld = false;
        if (!target || !host) {
            return;
        }
        if (!dragging) {
            host.mouse(target, 1, px, py);
        }
        host.mouse(target, 2, px, py);
        dragging = false;
        // A click into a field: Return and Enter from a keyboard go to the page while typing there.
        typingCheck.restart();
    }

    // The pointer moved from (ox, oy): hover, or drag while OK is held.
    function moved(ox, oy) {
        if (!target || !host || (px === ox && py === oy)) {
            return;
        }
        if (okHeld) {
            if (!dragging) {
                holdTimer.stop();
                host.mouse(target, 1, ox, oy);
                dragging = true;
            }
        }
        host.mouse(target, 0, px, py);
    }

    function scrollBy(dx, dy) {
        if (target && host && (dx !== 0 || dy !== 0)) {
            host.wheel(target, px, py, dx, dy);
        }
    }

    // One frame of movement, `dt` milliseconds after the last.
    function step(dt) {
        dt = Math.max(1, Math.min(dt, 50));
        const f = dt / 16.67;
        const accel = 0.05 * unit * dt;
        velX = dirX === 0 ? 0 : Math.max(-maxSpeed, Math.min(maxSpeed, velX + dirX * accel));
        velY = dirY === 0 ? 0 : Math.max(-maxSpeed, Math.min(maxSpeed, velY + dirY * accel));
        const sx = velX * f;
        const sy = velY * f;
        if (scrollMode) {
            scrollBy(sx * 2, sy * 2);
        } else if (sx !== 0 || sy !== 0) {
            poke();
            const ox = px, oy = py;
            px = Math.max(0, Math.min(width - 1, px + sx));
            py = Math.max(0, Math.min(height - 1, py + sy));
            moved(ox, oy);
            // At an edge and still pushing: the page scrolls instead.
            const ex = (px > width - edge && sx > 0) || (px < edge && sx < 0) ? sx : 0;
            const ey = (py > height - edge && sy > 0) || (py < edge && sy < 0) ? sy : 0;
            if (!okHeld) {
                scrollBy(ex, ey);
            }
        }
        if (pendingScroll !== 0) {
            const s = Math.abs(pendingScroll) < 8 ? pendingScroll : pendingScroll * 0.22;
            pendingScroll -= s;
            scrollBy(0, s);
        }
        frames.running = dirX !== 0 || dirY !== 0 || pendingScroll !== 0;
    }

    FrameAnimation {
        id: frames
        running: false
        onTriggered: pointer.step(frameTime * 1000)
    }

    Timer {
        id: hideTimer
        interval: pointer.hideAfter
        onTriggered: if (!pointer.okHeld) pointer.shown = false
    }

    // OK held still for a second: what is under the pointer?
    Timer {
        id: holdTimer
        interval: pointer.holdFor
        onTriggered: {
            if (!pointer.okHeld || pointer.dragging) {
                return;
            }
            pointer.okHeld = false;
            pointer.okRevealOnly = true; // the release that follows does nothing
            pointer.waitingForAnswer = true;
            answerTimer.restart();
            pointer.hold(pointer.px, pointer.py);
        }
    }

    // A page that keeps its context menu to itself never answers: then it is scroll mode.
    Timer {
        id: answerTimer
        interval: 400
        onTriggered: pointer.answered(false)
    }

    Timer {
        id: typingCheck
        interval: 200
        onTriggered: if (pointer.host) pointer.host.typing = pointer.host.editing()
    }

    // The pointer itself: a filled dot with a white ring and a faint dark outline, readable on any page.
    Rectangle {
        readonly property real r: Math.max(pointer.width / 132, 6) + (pointer.okHeld || pointer.dragging ? 4 : 0)
        visible: pointer.shown && !pointer.scrollMode
        x: pointer.px - r
        y: pointer.py - r
        width: r * 2
        height: r * 2
        radius: r
        color: "#577fff"
        border.color: "white"
        border.width: 3
        antialiasing: true

        Rectangle {
            anchors.centerIn: parent
            width: parent.width + 6
            height: width
            radius: width / 2
            color: "transparent"
            border.color: "#99000000"
            border.width: 2
        }
    }
}
