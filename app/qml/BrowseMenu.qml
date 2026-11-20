// SPDX-License-Identifier: GPL-3.0-only
//
// Sofa's menu over a page (Back or Menu on the remote): on top the page's address, the tabs (when there are two or
// more) and "New tab"; under it the buttons. It opens on the button used last (Home the first time); Back closes it.
// OK on another tab switches to it; OK on the tab in sight, or OK held on any tab, asks which tabs to close.

import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

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

    function show() {
        const all = buttonRow.buttons();
        const b = all.find(c => c.objectName === lastFocused && c.available)
               || all.find(c => c.available);
        if (b) {
            b.forceActiveFocus();
        }
    }

    Keys.onBackPressed: menu.closeRequested()
    Keys.onEscapePressed: menu.closeRequested()
    Keys.onMenuPressed: menu.closeRequested()

    // Pressing outside the panel closes the menu (a mouse or a touch screen).
    TapHandler {
        onTapped: menu.closeRequested()
    }

    Rectangle {
        anchors.fill: panel
        anchors.margins: -Kirigami.Units.gridUnit
        color: Kirigami.Theme.backgroundColor
        opacity: 0.96
        radius: Kirigami.Units.cornerRadius * 3

        // Taps on the panel itself don't close the menu.
        TapHandler {}
    }

    ColumnLayout {
        id: panel
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        anchors.margins: Kirigami.Units.gridUnit * 2
        spacing: Kirigami.Units.gridUnit

        RowLayout {
            id: strip
            Layout.fillWidth: true
            spacing: Kirigami.Units.largeSpacing

            Pressable {
                id: addressPill
                Layout.preferredWidth: menu.tabs.length > 1 ? Kirigami.Units.gridUnit * 18 : Kirigami.Units.gridUnit * 30
                Layout.preferredHeight: Kirigami.Units.gridUnit * 2.5
                Accessible.name: qsTr("Address: %1").arg(menu.address)
                Keys.onDownPressed: menu.show()
                onActivated: menu.action("address")
                Keys.onRightPressed: chips.count > 0 ? chips.focusAt(0) : newTabChip.forceActiveFocus()

                Rectangle {
                    anchors.fill: parent
                    radius: height / 2
                    color: addressPill.activeFocus ? Kirigami.Theme.highlightColor : Kirigami.Theme.alternateBackgroundColor
                }
                RowLayout {
                    anchors.fill: parent
                    anchors.leftMargin: Kirigami.Units.gridUnit
                    anchors.rightMargin: Kirigami.Units.gridUnit
                    Kirigami.Icon {
                        source: menu.privateMode ? "view-private" : "search"
                        Layout.preferredWidth: Kirigami.Units.iconSizes.small
                        Layout.preferredHeight: Kirigami.Units.iconSizes.small
                        color: addressPill.activeFocus ? Kirigami.Theme.highlightedTextColor : Kirigami.Theme.disabledTextColor
                        isMask: true
                    }
                    QQC2.Label {
                        Layout.fillWidth: true
                        text: menu.address
                        elide: Text.ElideRight
                        color: addressPill.activeFocus ? Kirigami.Theme.highlightedTextColor : Kirigami.Theme.textColor
                    }
                }
            }

            ListView {
                id: chips
                Layout.fillWidth: true
                Layout.preferredHeight: Kirigami.Units.gridUnit * 2.5
                orientation: ListView.Horizontal
                spacing: Kirigami.Units.smallSpacing
                clip: true
                visible: menu.tabs.length > 1
                model: menu.tabs.length > 1 ? menu.tabs : []
                keyNavigationEnabled: true
                boundsBehavior: Flickable.StopAtBounds
                Accessible.role: Accessible.PageTabList
                Accessible.name: qsTr("Tabs")

                function focusAt(i) {
                    currentIndex = i;
                    positionViewAtIndex(i, ListView.Contain);
                    forceActiveFocus();
                }

                Keys.onLeftPressed: event => currentIndex > 0 ? decrementCurrentIndex() : addressPill.forceActiveFocus()
                Keys.onRightPressed: event => currentIndex < count - 1 ? incrementCurrentIndex() : newTabChip.forceActiveFocus()
                Keys.onDownPressed: buttonRow.focusNear()

                delegate: Pressable {
                    id: chip
                    required property var modelData
                    required property int index
                    width: Math.min(Kirigami.Units.gridUnit * 10, Math.max(Kirigami.Units.gridUnit * 5, label.implicitWidth + Kirigami.Units.gridUnit * 2))
                    height: ListView.view.height
                    focus: ListView.isCurrentItem
                    holdable: true
                    Accessible.role: Accessible.PageTab
                    Accessible.name: modelData.current ? qsTr("%1, in sight").arg(modelData.name) : modelData.name
                    onActivated: modelData.current ? menu.tabHeld(modelData.id) : menu.tabChosen(modelData.id)
                    onHeld: menu.tabHeld(modelData.id)

                    Rectangle {
                        anchors.fill: parent
                        radius: height / 2
                        color: chip.activeFocus ? Kirigami.Theme.highlightColor : Kirigami.Theme.alternateBackgroundColor
                        border.width: chip.modelData.current ? 3 : 0
                        border.color: Kirigami.Theme.highlightColor
                    }
                    QQC2.Label {
                        id: label
                        anchors.centerIn: parent
                        width: parent.width - Kirigami.Units.gridUnit
                        horizontalAlignment: Text.AlignHCenter
                        text: chip.modelData.name
                        // Two pages of one site differ at the end of their address: the middle goes.
                        elide: Text.ElideMiddle
                        color: chip.activeFocus ? Kirigami.Theme.highlightedTextColor : Kirigami.Theme.textColor
                    }
                }
            }

            Item {
                visible: menu.tabs.length <= 1
                Layout.fillWidth: true
            }

            Pressable {
                id: newTabChip
                Layout.preferredWidth: newTabLabel.implicitWidth + Kirigami.Units.gridUnit * 3
                Layout.preferredHeight: Kirigami.Units.gridUnit * 2.5
                Accessible.name: qsTr("New tab")
                Keys.onDownPressed: menu.show()
                Keys.onLeftPressed: chips.count > 0 ? chips.focusAt(chips.count - 1) : addressPill.forceActiveFocus()
                onActivated: menu.action("newTab")

                Rectangle {
                    anchors.fill: parent
                    radius: height / 2
                    color: newTabChip.activeFocus ? Kirigami.Theme.highlightColor : Kirigami.Theme.alternateBackgroundColor
                }
                RowLayout {
                    anchors.centerIn: parent
                    Kirigami.Icon {
                        source: "tab-new"
                        Layout.preferredWidth: Kirigami.Units.iconSizes.small
                        Layout.preferredHeight: Kirigami.Units.iconSizes.small
                        color: newTabChip.activeFocus ? Kirigami.Theme.highlightedTextColor : Kirigami.Theme.textColor
                        isMask: true
                    }
                    QQC2.Label {
                        id: newTabLabel
                        text: qsTr("New tab")
                        color: newTabChip.activeFocus ? Kirigami.Theme.highlightedTextColor : Kirigami.Theme.textColor
                    }
                }
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
            clip: true

            function reveal(item) {
                const left = item.x - Kirigami.Units.gridUnit;
                const right = item.x + item.width + Kirigami.Units.gridUnit;
                if (left < contentX) {
                    contentX = Math.max(0, left);
                } else if (right > contentX + width) {
                    contentX = Math.min(contentWidth - width, right - width);
                }
            }

            Row {
                id: buttonRow
                x: Math.max(0, (buttonScroller.width - width) / 2)
                spacing: Kirigami.Units.smallSpacing

                function buttons() {
                    const out = [];
                    for (let i = 0; i < children.length; i++) {
                        if (children[i].objectName !== "" && children[i].visible) {
                            out.push(children[i]);
                        }
                    }
                    return out;
                }

                // Down from the strip: the last button used.
                function focusNear() {
                    menu.show();
                }

                Keys.onUpPressed: addressPill.forceActiveFocus()
                Keys.onLeftPressed: step(-1)
                Keys.onRightPressed: step(1)
                function step(delta) {
                    const list = buttons();
                    const i = list.findIndex(c => c.activeFocus);
                    if (i + delta >= 0 && i + delta < list.length) {
                        list[i + delta].forceActiveFocus();
                    }
                }

                component MenuButton: IconButton {
                    onActiveFocusChanged: if (activeFocus) {
                        menu.lastFocused = objectName;
                        buttonScroller.reveal(this);
                    }
                    onActivated: if (available) menu.action(objectName)
                }

                MenuButton { objectName: "home"; iconName: "go-home"; text: qsTr("Home") }
                MenuButton { objectName: "back"; iconName: "go-previous"; text: qsTr("Back"); available: menu.canGoBack }
                MenuButton { objectName: "forward"; iconName: "go-next"; text: qsTr("Forward"); available: menu.canGoForward }
                MenuButton { objectName: "reload"; iconName: "view-refresh"; text: qsTr("Reload") }
                MenuButton {
                    objectName: "bookmark"
                    iconName: menu.bookmarked ? "starred-symbolic" : "non-starred-symbolic"
                    text: menu.bookmarked ? qsTr("Bookmarked") : qsTr("Bookmark")
                    checked: menu.bookmarked
                }
                MenuButton { objectName: "pin"; iconName: "list-add"; text: qsTr("Add to start page") }
                MenuButton { objectName: "bookmarks"; iconName: "bookmarks"; text: qsTr("Bookmarks") }
                MenuButton { objectName: "history"; iconName: "view-history"; text: qsTr("History") }
                MenuButton {
                    objectName: "adblock"
                    iconName: menu.adBlockOn ? "security-high" : "security-low"
                    // The state shows in the icon and the tint: "Ad block: off" doesn't fit under the button.
                    text: qsTr("Ad block")
                    Accessible.name: menu.adBlockOn ? qsTr("Ad blocking on for this site") : qsTr("Ad blocking off for this site")
                    checked: menu.adBlockOn
                    available: menu.webPage
                }
                MenuButton {
                    objectName: "allowOnce"
                    iconName: "window-new"
                    text: qsTr("Allow once")
                    Accessible.name: qsTr("Open the pop-up or redirect just blocked")
                    visible: menu.allowPending
                }
                MenuButton {
                    objectName: "pointer"
                    iconName: "input-mouse"
                    text: menu.pointerOn ? qsTr("Pointer: on") : qsTr("Pointer: off")
                    checked: menu.pointerOn
                }
                MenuButton { objectName: "zoomOut"; iconName: "zoom-out"; text: qsTr("Smaller") }
                MenuButton {
                    objectName: "zoomReset"
                    iconName: "zoom-original"
                    text: Math.round(menu.zoom * 100) + " %"
                    Accessible.name: qsTr("Zoom %1 percent, press to reset").arg(Math.round(menu.zoom * 100))
                }
                MenuButton { objectName: "zoomIn"; iconName: "zoom-in"; text: qsTr("Larger") }
                MenuButton { objectName: "settings"; iconName: "settings-configure"; text: qsTr("Settings") }
            }
        }
    }
}
