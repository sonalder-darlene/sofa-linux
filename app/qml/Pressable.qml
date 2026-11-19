// SPDX-License-Identifier: GPL-3.0-only
//
// Something the remote can press: OK (Return, Enter, Select, Space) activates it on release; when `holdable`,
// holding OK for a moment or pressing Menu asks for its actions instead (held). A mouse or touch works too.
// Repeats of a held key are ignored, and a release whose press happened elsewhere (the press that opened this
// screen) does nothing.

import QtQuick

FocusScope {
    id: control

    signal activated()
    signal held()

    property bool holdable: false
    readonly property bool down: pressedDown || tap.pressed
    property bool pressedDown: false

    activeFocusOnTab: true
    Accessible.role: Accessible.Button
    Accessible.focusable: true
    Accessible.onPressAction: control.activated()

    function isOk(key) {
        return key === Qt.Key_Return || key === Qt.Key_Enter || key === Qt.Key_Select || key === Qt.Key_Space;
    }

    onActiveFocusChanged: if (!activeFocus) {
        pressedDown = false;
        holdTimer.stop();
    }

    Keys.onPressed: event => {
        if (isOk(event.key)) {
            event.accepted = true;
            if (event.isAutoRepeat) {
                return;
            }
            pressedDown = true;
            if (holdable) {
                holdTimer.restart();
            }
        } else if (event.key === Qt.Key_Menu && holdable) {
            event.accepted = true;
            if (!event.isAutoRepeat) {
                held();
            }
        }
    }

    Keys.onReleased: event => {
        if (!isOk(event.key)) {
            return;
        }
        event.accepted = true;
        if (event.isAutoRepeat || !pressedDown) {
            return;
        }
        pressedDown = false;
        holdTimer.stop();
        activated();
    }

    Timer {
        id: holdTimer
        interval: 700
        onTriggered: {
            control.pressedDown = false;
            control.held();
        }
    }

    TapHandler {
        id: tap
        onTapped: {
            control.forceActiveFocus();
            control.activated();
        }
        onLongPressed: if (control.holdable) {
            control.forceActiveFocus();
            control.held();
        }
    }
}
