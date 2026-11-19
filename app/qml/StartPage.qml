// SPDX-License-Identifier: GPL-3.0-only
//
// The start page: a top bar (the search pill and the main buttons) over rows of cards: Quick access (the user's own
// tiles), Most visited (from History, not in private browsing) and Bookmarks (the top level's folders, then its
// newest bookmarks, and "All bookmarks" when they don't all fit). Everything comes from this computer.

import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import Sofa
import "words.js" as Words

FocusScope {
    id: page

    property bool privateMode: false

    signal openUrl(string url)
    signal search()
    signal openBookmarks(int folder)
    signal openHistory()
    signal togglePrivate()
    signal openSettings()
    signal exitRequested()
    signal tileHeld(var tile)
    signal addTile()

    // Bookmarks cards on the start page: one line of twelve, as on Android.
    readonly property int bookmarkSlots: 12

    property var tiles: []
    property var mostVisited: []
    property var bookmarkCards: []
    // The Quick access tile being carried with Left and Right, or -1.
    property int movingTile: -1
    // The card that opened something, by row and key, focused again when the start page comes back.
    property var opened: null

    Accessible.role: Accessible.Pane
    Accessible.name: qsTr("Start page")

    function refresh() {
        const t = JSON.parse(Library.tiles());
        tiles = t.map(s => ({ kind: "site", id: s.id, title: s.title, url: s.url, holdable: true }))
                 .concat([{ kind: "add", id: -1, title: qsTr("Add site"), icon: "list-add" }]);
        mostVisited = privateMode ? [] : JSON.parse(Library.mostVisited(10))
            .map(s => ({ kind: "site", id: s.id, title: s.title, url: s.url }));
        const b = JSON.parse(Library.startBookmarks(bookmarkSlots));
        let cards = b.folders.map(f => ({
            kind: "folder", id: f.id, title: f.title, icon: "folder",
            detail: folderDetail(f.folders, f.bookmarks)
        })).concat(b.bookmarks.map(s => ({ kind: "site", id: s.id, title: s.title, url: s.url })));
        if (b.total > bookmarkSlots) {
            cards = cards.slice(0, bookmarkSlots - 1);
            cards.push({ kind: "all", id: 0, title: qsTr("All bookmarks"), icon: "bookmarks" });
        } else {
            cards = cards.slice(0, bookmarkSlots);
        }
        bookmarkCards = cards;
    }

    function folderDetail(folders, bookmarks) {
        if (folders === 0 && bookmarks === 0) {
            return qsTr("Empty");
        }
        const parts = [];
        if (folders > 0) {
            parts.push(Words.count(folders, qsTr("%n folder"), qsTr("%n folders")));
        }
        if (bookmarks > 0) {
            parts.push(Words.count(bookmarks, qsTr("%n bookmark"), qsTr("%n bookmarks")));
        }
        return parts.join(", ");
    }

    // The rows that have cards (not `visible`, which is false whenever the start page itself is hidden).
    readonly property var rows: [quickRow, mostRow, bookmarkRow].filter(r => r.items.length > 0)

    // Focus when the start page is shown: the card that opened the page, else the first card.
    function focusFirst() {
        if (opened) {
            const r = opened.row;
            if (r.items.length > 0) {
                r.focusIndex(Math.min(opened.index, r.items.length - 1));
                opened = null;
                return;
            }
            opened = null;
        }
        if (rows.length > 0) {
            rows[0].focusIndex(0);
        } else {
            searchPill.forceActiveFocus();
        }
    }

    function open(row, item, index) {
        opened = { row: row, index: index };
        if (item.kind === "site") {
            openUrl(item.url);
        } else if (item.kind === "folder") {
            openBookmarks(item.id);
        } else if (item.kind === "all") {
            openBookmarks(0);
        } else if (item.kind === "add") {
            addTile();
        }
    }

    function moveRow(from, delta) {
        const i = rows.indexOf(from) + delta;
        if (i < 0) {
            searchPill.forceActiveFocus();
        } else if (i < rows.length) {
            rows[i].focusNear(from.focusX());
        }
    }

    function startMoving(tileId) {
        const i = tiles.findIndex(t => t.id === tileId);
        if (i >= 0) {
            movingTile = i;
            quickRow.focusIndex(i);
        }
    }

    // A private session's start page looks apart, so it is never mistaken for the normal one.
    Rectangle {
        anchors.fill: parent
        color: page.privateMode ? Qt.darker(Kirigami.Theme.backgroundColor, 1.6) : Kirigami.Theme.backgroundColor
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.leftMargin: Kirigami.Units.gridUnit * 3
        anchors.rightMargin: Kirigami.Units.gridUnit * 3
        anchors.topMargin: Kirigami.Units.gridUnit * 1.5
        spacing: Kirigami.Units.gridUnit

        RowLayout {
            id: topBar
            Layout.fillWidth: true
            spacing: Kirigami.Units.largeSpacing

            Image {
                Layout.alignment: Qt.AlignTop
                source: "qrc:/sofa/icon.svg"
                sourceSize.width: Kirigami.Units.gridUnit * 3
                sourceSize.height: Kirigami.Units.gridUnit * 3
                Accessible.ignored: true
            }

            Pressable {
                id: searchPill
                Layout.alignment: Qt.AlignTop
                Layout.preferredWidth: Kirigami.Units.gridUnit * 30
                Layout.preferredHeight: Kirigami.Units.gridUnit * 3
                focus: true
                Accessible.name: qsTr("Search or type a web address")
                KeyNavigation.right: bookmarksButton
                Keys.onDownPressed: if (page.rows.length > 0) page.rows[0].focusNear(searchPill.mapToItem(null, searchPill.width / 4, 0).x)
                onActivated: page.search()

                Rectangle {
                    anchors.fill: parent
                    radius: height / 2
                    color: searchPill.activeFocus ? Kirigami.Theme.highlightColor : Kirigami.Theme.alternateBackgroundColor

                    RowLayout {
                        anchors.fill: parent
                        anchors.leftMargin: Kirigami.Units.gridUnit
                        anchors.rightMargin: Kirigami.Units.gridUnit
                        spacing: Kirigami.Units.largeSpacing

                        Kirigami.Icon {
                            source: "search"
                            Layout.preferredWidth: Kirigami.Units.iconSizes.smallMedium
                            Layout.preferredHeight: Kirigami.Units.iconSizes.smallMedium
                            color: searchPill.activeFocus ? Kirigami.Theme.highlightedTextColor : Kirigami.Theme.disabledTextColor
                            isMask: true
                        }
                        Text {
                            Layout.fillWidth: true
                            text: qsTr("Search or type a web address")
                            color: searchPill.activeFocus ? Kirigami.Theme.highlightedTextColor : Kirigami.Theme.disabledTextColor
                            font.pointSize: Kirigami.Theme.defaultFont.pointSize * 1.3
                            elide: Text.ElideRight
                        }
                    }
                }
            }

            Item {
                Layout.fillWidth: true
            }

            Row {
                id: buttons
                Layout.alignment: Qt.AlignTop
                spacing: Kirigami.Units.smallSpacing

                Keys.onDownPressed: if (page.rows.length > 0) page.rows[0].focusNear(page.width / 2)

                IconButton {
                    id: bookmarksButton
                    iconName: "bookmarks"
                    text: qsTr("Bookmarks")
                    KeyNavigation.left: searchPill
                    KeyNavigation.right: historyButton
                    onActivated: page.openBookmarks(0)
                }
                IconButton {
                    id: historyButton
                    iconName: "view-history"
                    text: qsTr("History")
                    KeyNavigation.right: privateButton
                    onActivated: page.openHistory()
                }
                IconButton {
                    id: privateButton
                    iconName: "view-private"
                    text: page.privateMode ? qsTr("Private: on") : qsTr("Private")
                    checked: page.privateMode
                    KeyNavigation.right: settingsButton
                    onActivated: page.togglePrivate()
                }
                IconButton {
                    id: settingsButton
                    iconName: "settings-configure"
                    text: qsTr("Settings")
                    KeyNavigation.right: exitButton
                    onActivated: page.openSettings()
                }
                IconButton {
                    id: exitButton
                    iconName: "application-exit"
                    text: qsTr("Exit")
                    onActivated: page.exitRequested()
                }
            }
        }

        Kirigami.InlineMessage {
            Layout.fillWidth: true
            visible: page.privateMode
            type: Kirigami.MessageType.Information
            text: qsTr("Private browsing: no history is kept, and cookies and site data are erased when you leave it.")
        }

        Flickable {
            id: flick
            Layout.fillWidth: true
            Layout.fillHeight: true
            contentHeight: rowsColumn.implicitHeight + Kirigami.Units.gridUnit * 2
            clip: true
            boundsBehavior: Flickable.StopAtBounds
            interactive: contentHeight > height

            // Keeps the row in focus in sight.
            function reveal(item) {
                const top = item.mapToItem(rowsColumn, 0, 0).y;
                const bottom = top + item.height;
                if (top < contentY) {
                    contentY = Math.max(0, top - Kirigami.Units.gridUnit);
                } else if (bottom > contentY + height) {
                    contentY = Math.min(contentHeight - height, bottom - height + Kirigami.Units.gridUnit);
                }
            }

            Behavior on contentY {
                NumberAnimation { duration: Kirigami.Units.shortDuration }
            }

            ColumnLayout {
                id: rowsColumn
                width: flick.width
                spacing: Kirigami.Units.gridUnit * 1.5

                CardRow {
                    id: quickRow
                    Layout.fillWidth: true
                    title: qsTr("Quick access")
                    items: page.tiles
                    movingIndex: page.movingTile
                    onActiveFocusChanged: if (activeFocus) flick.reveal(this)
                    onOpened: (item, index) => page.open(quickRow, item, index)
                    onHeld: item => page.tileHeld(item)
                    onCarry: (from, to) => {
                        // "Add site" stays last.
                        if (to < 0 || to >= page.tiles.length - 1) {
                            return;
                        }
                        const t = page.tiles.slice();
                        const moved = t.splice(from, 1)[0];
                        t.splice(to, 0, moved);
                        page.tiles = t;
                        page.movingTile = to;
                        quickRow.focusIndex(to);
                    }
                    onDropped: {
                        Library.reorderTiles(JSON.stringify(page.tiles.filter(t => t.kind === "site").map(t => t.id)));
                        page.movingTile = -1;
                    }
                    Keys.onUpPressed: page.moveRow(quickRow, -1)
                    Keys.onDownPressed: page.moveRow(quickRow, 1)
                }

                CardRow {
                    id: mostRow
                    Layout.fillWidth: true
                    visible: page.mostVisited.length > 0
                    title: qsTr("Most visited")
                    items: page.mostVisited
                    onActiveFocusChanged: if (activeFocus) flick.reveal(this)
                    onOpened: (item, index) => page.open(mostRow, item, index)
                    Keys.onUpPressed: page.moveRow(mostRow, -1)
                    Keys.onDownPressed: page.moveRow(mostRow, 1)
                }

                CardRow {
                    id: bookmarkRow
                    Layout.fillWidth: true
                    visible: page.bookmarkCards.length > 0
                    title: qsTr("Bookmarks")
                    items: page.bookmarkCards
                    onActiveFocusChanged: if (activeFocus) flick.reveal(this)
                    onOpened: (item, index) => page.open(bookmarkRow, item, index)
                    Keys.onUpPressed: page.moveRow(bookmarkRow, -1)
                    Keys.onDownPressed: page.moveRow(bookmarkRow, 1)
                }
            }
        }
    }
}
