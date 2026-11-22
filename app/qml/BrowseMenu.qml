// SPDX-License-Identifier: GPL-3.0-only
//
// Sofa's menu over a page (Back or Menu on the remote), laid out as KDE's TV apps lay out their controls over what
// plays: at the bottom, over a dark veil. On top the page's address, the tabs (when there are two or more) and "New
// tab"; under it a row of round buttons, the name of the one in focus under the row. It opens on the button used last
// (Home the first time); Back closes it. OK on another tab switches to it; OK on the tab in sight, or OK held on any
// tab, asks which tabs to close. A button that can't be used now (Back with no page behind) stays in its place,
// greyed, so the focus never jumps.

import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import org.kde.bigscreen.ui as BigscreenUI

FocusScope {
    id: menu

    // [{id, name, current}]
    property var tabs: []
    property string address: ""
    property bool canGoBack: false
    property bool canGoForward: false
    property bool bookmarked: false
    property bool pointerOn: true
    property bool privateMode: false
    // The page in sight is a web page, and ad blocking is on for its site.
    property bool webPage: false
    property bool adBlockOn: true
    // A pop-up or redirect was just blocked: "Allow once" shows.
    property bool allowPending: false
    property real zoom: 1

    signal action(string name)
    signal tabChosen(int id)
    signal tabHeld(int id)
    signal closeRequested()

    property string lastFocused: "home"
    // How far up the panel reaches, for notices shown over it.
    readonly property real panelHeight: panel.height + panel.anchors.bottomMargin

    function show() {
        const all = buttonRow.buttons();
        const b = all.find(c => c.objectName === lastFocused && c.available)
               || all.find(c => c.available);
        if (b) {
            b.forceActiveFocus();
        }
    }

    BigscreenUI.BackHandler.enabled: visible
    BigscreenUI.BackHandler.onActivated: menu.closeRequested()
    Keys.onMenuPressed: menu.closeRequested()

    // Pressing outside the panel closes the menu (a mouse or a touch screen).
    TapHandler {
        onTapped: menu.closeRequested()
    }

    Rectangle {
        anchors.fill: parent
        gradient: Gradient {
            GradientStop { position: 0.0; color: Qt.rgba(0, 0, 0, 0.35) }
            GradientStop { position: 0.45; color: Qt.rgba(0, 0, 0, 0.75) }
            GradientStop { position: 1.0; color: Qt.rgba(0, 0, 0, 0.95) }
        }
    }

    ColumnLayout {
        id: panel
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        anchors.leftMargin: BigscreenUI.Units.horizontalScreenMarginFor(menu.width)
        anchors.rightMargin: BigscreenUI.Units.horizontalScreenMarginFor(menu.width)
        anchors.bottomMargin: BigscreenUI.Units.verticalScreenMarginFor(menu.height)
        spacing: Kirigami.Units.gridUnit
        // Light text and controls over the dark veil, whatever the colour scheme.
        Kirigami.Theme.colorSet: Kirigami.Theme.Complementary
        Kirigami.Theme.inherit: false

        // Taps on the panel itself don't close the menu.
        TapHandler {}

        RowLayout {
            id: strip
            Layout.fillWidth: true
            spacing: Kirigami.Units.largeSpacing

            BigscreenUI.Button {
                id: addressButton
                Layout.preferredWidth: menu.tabs.length > 1 ? Kirigami.Units.gridUnit * 18 : Kirigami.Units.gridUnit * 34
                Layout.maximumWidth: strip.width - newTabChip.width - strip.spacing
                icon.name: menu.privateMode ? "view-private" : "search"
                text: menu.address
                Accessible.name: qsTr("Address: %1").arg(menu.address)
                Keys.onDownPressed: menu.show()
                Keys.onRightPressed: chips.count > 0 ? chips.focusAt(0) : newTabChip.forceActiveFocus()
                onClicked: menu.action("address")
            }

            ListView {
                id: chips
                Layout.fillWidth: true
                Layout.preferredHeight: newTabChip.implicitHeight
                orientation: ListView.Horizontal
                spacing: Kirigami.Units.largeSpacing
                clip: true
                visible: menu.tabs.length > 1
                model: menu.tabs.length > 1 ? menu.tabs : []
                keyNavigationEnabled: true
                boundsBehavior: Flickable.StopAtBounds
                highlightRangeMode: ListView.ApplyRange
                preferredHighlightBegin: Kirigami.Units.gridUnit
                preferredHighlightEnd: width - Kirigami.Units.gridUnit
                Accessible.role: Accessible.PageTabList
                Accessible.name: qsTr("Tabs")

                function focusAt(i) {
                    currentIndex = i;
                    positionViewAtIndex(i, ListView.Contain);
                    forceActiveFocus();
                }

                Keys.onLeftPressed: currentIndex > 0 ? decrementCurrentIndex() : addressButton.forceActiveFocus()
                Keys.onRightPressed: currentIndex < count - 1 ? incrementCurrentIndex() : newTabChip.forceActiveFocus()
                Keys.onDownPressed: menu.show()

                delegate: BigscreenUI.Chip {
                    id: chip
                    required property var modelData
                    required property int index
                    width: Math.min(Kirigami.Units.gridUnit * 12, implicitWidth)
                    focus: ListView.isCurrentItem
                    // The tab in sight is marked.
                    checked: modelData.current
                    text: modelData.name
                    Accessible.role: Accessible.PageTab
                    Accessible.name: modelData.current ? qsTr("%1, in sight").arg(modelData.name) : modelData.name
                    onClicked: modelData.current ? menu.tabHeld(modelData.id) : menu.tabChosen(modelData.id)
                    onPressAndHold: menu.tabHeld(modelData.id)
                    Keys.onMenuPressed: menu.tabHeld(modelData.id)

                    contentItem: BigscreenUI.Label {
                        text: chip.text
                        horizontalAlignment: Text.AlignHCenter
                        // Two pages of one site differ at the end of their address: the middle goes.
                        elide: Text.ElideMiddle
                        Accessible.ignored: true
                    }
                }
            }

            Item {
                visible: menu.tabs.length <= 1
                Layout.fillWidth: true
            }

            BigscreenUI.Button {
                id: newTabChip
                icon.name: "tab-new"
                text: qsTr("New tab")
                Keys.onDownPressed: menu.show()
                Keys.onLeftPressed: chips.count > 0 ? chips.focusAt(chips.count - 1) : addressButton.forceActiveFocus()
                onClicked: menu.action("newTab")
            }
        }

        // The buttons scroll sideways when they don't all fit, keeping the one in focus in sight.
        Flickable {
            id: buttonScroller
            Layout.fillWidth: true
            Layout.preferredHeight: buttonRow.height
            contentWidth: buttonRow.width
            contentHeight: buttonRow.height
            interactive: contentWidth > width
            boundsBehavior: Flickable.StopAtBounds
            // Room for the focused button's outline.
            leftMargin: Kirigami.Units.smallSpacing
            rightMargin: Kirigami.Units.smallSpacing

            function reveal(item) {
                const left = item.x - Kirigami.Units.gridUnit;
                const right = item.x + item.width + Kirigami.Units.gridUnit;
                if (left < contentX) {
                    contentX = Math.max(-leftMargin, left);
                } else if (right > contentX + width) {
                    contentX = Math.min(contentWidth + rightMargin - width, right - width);
                }
            }

            Row {
                id: buttonRow
                x: Math.max(0, (buttonScroller.width - width) / 2)
                spacing: Kirigami.Units.largeSpacing * 2

                function buttons() {
                    const out = [];
                    for (let i = 0; i < children.length; i++) {
                        if (children[i].objectName !== "" && children[i].visible) {
                            out.push(children[i]);
                        }
                    }
                    return out;
                }

                Keys.onUpPressed: addressButton.forceActiveFocus()
                Keys.onLeftPressed: step(-1)
                Keys.onRightPressed: step(1)
                function step(delta) {
                    const list = buttons();
                    const i = list.findIndex(c => c.activeFocus);
                    if (i + delta >= 0 && i + delta < list.length) {
                        list[i + delta].forceActiveFocus();
                    }
                }

                component MenuButton: BigscreenUI.RoundButton {
                    // Greyed out and skipped by OK, but it can still take focus, so focus never jumps.
                    property bool available: true
                    opacity: available ? 1 : 0.4
                    Accessible.name: text
                    onActiveFocusChanged: if (activeFocus) {
                        menu.lastFocused = objectName;
                        buttonScroller.reveal(this);
                    }
                    onClicked: if (available) menu.action(objectName)
                }

                MenuButton { objectName: "home"; icon.name: "go-home"; text: qsTr("Home") }
                MenuButton { objectName: "back"; icon.name: "go-previous"; text: qsTr("Back"); available: menu.canGoBack }
                MenuButton { objectName: "forward"; icon.name: "go-next"; text: qsTr("Forward"); available: menu.canGoForward }
                MenuButton { objectName: "reload"; icon.name: "view-refresh"; text: qsTr("Reload") }
                MenuButton {
                    objectName: "bookmark"
                    icon.name: menu.bookmarked ? "starred-symbolic" : "non-starred-symbolic"
                    text: menu.bookmarked ? qsTr("Bookmarked") : qsTr("Bookmark")
                    checked: menu.bookmarked
                }
                MenuButton { objectName: "pin"; icon.name: "list-add"; text: qsTr("Add to start page") }
                MenuButton { objectName: "bookmarks"; icon.name: "bookmarks"; text: qsTr("Bookmarks") }
                MenuButton { objectName: "history"; icon.name: "view-history"; text: qsTr("History") }
                MenuButton {
                    objectName: "adblock"
                    icon.name: menu.adBlockOn ? "security-high" : "security-low"
                    text: menu.adBlockOn ? qsTr("Ad blocking: on for this site") : qsTr("Ad blocking: off for this site")
                    checked: menu.adBlockOn
                    available: menu.webPage
                }
                MenuButton {
                    objectName: "allowOnce"
                    icon.name: "window-new"
                    text: qsTr("Allow once: open the pop-up or redirect just blocked")
                    visible: menu.allowPending
                }
                MenuButton {
                    objectName: "pointer"
                    icon.name: "input-mouse"
                    text: menu.pointerOn ? qsTr("Pointer: on") : qsTr("Pointer: off")
                    checked: menu.pointerOn
                }
                MenuButton { objectName: "zoomOut"; icon.name: "zoom-out"; text: qsTr("Smaller") }
                MenuButton {
                    objectName: "zoomReset"
                    icon.name: "zoom-original"
                    text: qsTr("Zoom %1 %: press to reset").arg(Math.round(menu.zoom * 100))
                }
                MenuButton { objectName: "zoomIn"; icon.name: "zoom-in"; text: qsTr("Larger") }
                MenuButton { objectName: "settings"; icon.name: "settings-configure"; text: qsTr("Settings") }
            }
        }

        // The name of the round button in focus, as KDE's TV apps show it.
        BigscreenUI.Label {
            Layout.fillWidth: true
            horizontalAlignment: Text.AlignHCenter
            text: {
                const item = menu.Window.activeFocusItem;
                return item instanceof BigscreenUI.RoundButton && item.parent === buttonRow ? item.text : "";
            }
            // Keeps its height when empty, so nothing moves.
            opacity: text.length > 0 ? 1 : 0
            color: Kirigami.Theme.disabledTextColor
            Accessible.ignored: true
        }
    }
}
