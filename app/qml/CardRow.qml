// SPDX-License-Identifier: GPL-3.0-only
//
// One row of the start page: a title over cards that scroll sideways. Left and Right move along the row; the start
// page moves between rows and keeps the focus near the same place across.

import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

FocusScope {
    id: row

    property string title: ""
    // [{kind: "site" | "folder" | "add" | "all", id, title, url, detail, icon}]
    property var items: []
    // The card being carried with Left and Right, or -1.
    property int movingIndex: -1
    readonly property alias currentIndex: list.currentIndex

    signal opened(var item, int index)
    signal held(var item, int index)
    // Left or Right while a card is carried: the start page swaps it with its neighbour.
    signal carry(int from, int to)
    signal dropped()

    implicitHeight: column.implicitHeight
    Accessible.role: Accessible.List
    Accessible.name: title

    // Focus the card nearest to `x` (in the start page's coordinates) when the row is entered from above or below.
    function focusNear(sceneX) {
        let best = 0;
        let bestDistance = Infinity;
        for (let i = 0; i < list.count; i++) {
            const item = list.itemAtIndex(i);
            if (!item) {
                continue;
            }
            const centre = item.mapToItem(null, item.width / 2, 0).x;
            const d = Math.abs(centre - sceneX);
            if (d < bestDistance) {
                bestDistance = d;
                best = i;
            }
        }
        list.currentIndex = best;
        list.forceActiveFocus();
    }

    function focusIndex(i) {
        list.currentIndex = Math.max(0, Math.min(i, list.count - 1));
        list.positionViewAtIndex(list.currentIndex, ListView.Contain);
        list.forceActiveFocus();
    }

    // The middle of the focused card, for the row below or above.
    function focusX() {
        const item = list.currentItem;
        return item ? item.mapToItem(null, item.width / 2, 0).x : 0;
    }

    ColumnLayout {
        id: column
        anchors.left: parent.left
        anchors.right: parent.right
        spacing: Kirigami.Units.largeSpacing

        Kirigami.Heading {
            visible: row.title !== ""
            text: row.title
            level: 3
            color: Kirigami.Theme.disabledTextColor
            Accessible.role: Accessible.Heading
        }

        ListView {
            id: list
            Layout.fillWidth: true
            Layout.preferredHeight: Kirigami.Units.gridUnit * 9
            orientation: ListView.Horizontal
            spacing: Kirigami.Units.largeSpacing * 1.5
            model: row.items
            focus: true
            clip: false
            keyNavigationEnabled: row.movingIndex < 0
            highlightMoveDuration: Kirigami.Units.shortDuration
            preferredHighlightBegin: Kirigami.Units.gridUnit * 2
            preferredHighlightEnd: width - Kirigami.Units.gridUnit * 11
            highlightRangeMode: ListView.ApplyRange
            boundsBehavior: Flickable.StopAtBounds
            leftMargin: Kirigami.Units.smallSpacing
            rightMargin: Kirigami.Units.gridUnit * 2

            Keys.onPressed: event => {
                if (row.movingIndex < 0) {
                    return;
                }
                event.accepted = true;
                if (event.key === Qt.Key_Left || event.key === Qt.Key_Right) {
                    row.carry(row.movingIndex, row.movingIndex + (event.key === Qt.Key_Left ? -1 : 1));
                } else if (event.key === Qt.Key_Escape || event.key === Qt.Key_Back) {
                    if (!event.isAutoRepeat) {
                        row.dropped();
                    }
                }
            }

            delegate: SiteCard {
                required property var modelData
                required property int index
                y: Kirigami.Units.smallSpacing * 2
                title: modelData.title
                url: modelData.url || ""
                detail: modelData.detail || ""
                iconName: modelData.icon || ""
                holdable: modelData.kind === "site" && modelData.holdable === true
                moving: index === row.movingIndex
                focus: ListView.isCurrentItem
                onActivated: row.movingIndex < 0 ? row.opened(modelData, index) : row.dropped()
                onHeld: row.held(modelData, index)
            }
        }
    }
}
