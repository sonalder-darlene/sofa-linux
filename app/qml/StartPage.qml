// SPDX-License-Identifier: GPL-3.0-only
//
// The start page, on KDE's TV rails: a top bar (the search button and the main buttons) over rows of cards: Quick
// access (the user's own tiles), Most visited (from History, not in private browsing) and Bookmarks (the top level's
// folders, then its newest bookmarks, and "All bookmarks" when they don't all fit). Everything comes from this
// computer. Up and Down between rows keep the focus near the same place across.

import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import org.kde.bigscreen.ui as BigscreenUI
import Sofa
import "words.js" as Words

BigscreenUI.Page {
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
    // Back on the start page (when no tile is being carried).
    signal backPressed()

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

    BigscreenUI.BackHandler.enabled: visible
    BigscreenUI.BackHandler.onActivated: {
        if (movingTile >= 0) {
            drop();
        } else {
            backPressed();
        }
    }

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

    // The rows that have cards, top to bottom.
    readonly property var rows: [quickRow, mostRow, bookmarkRow].filter(r => r.count > 0)

    // Focus when the start page is shown: the card that opened the page, else the first card. It waits for the rails
    // to take their new cards (a rail with none is taken off the page until it has some).
    function focusFirst() {
        Qt.callLater(focusNow);
    }

    function focusNow() {
        if (opened) {
            const r = opened.row;
            if (r.count > 0) {
                focusIndex(r, Math.min(opened.index, r.count - 1));
                opened = null;
                return;
            }
            opened = null;
        }
        if (rows.length > 0) {
            focusIndex(rows[0], 0);
        } else {
            searchButton.forceActiveFocus();
        }
    }

    function focusIndex(row, i) {
        row.currentIndex = Math.max(0, Math.min(i, row.count - 1));
        row.forceActiveFocus();
    }

    // Focus the card of `row` nearest to `sceneX` (the middle of what had the focus above or below).
    function focusNear(row, sceneX) {
        let best = 0;
        let bestDistance = Infinity;
        for (let i = 0; i < row.count; i++) {
            const item = row.view.itemAtIndex(i);
            if (!item) {
                continue;
            }
            const d = Math.abs(item.mapToItem(null, item.width / 2, 0).x - sceneX);
            if (d < bestDistance) {
                bestDistance = d;
                best = i;
            }
        }
        focusIndex(row, best);
    }

    function middleOf(item) {
        return item ? item.mapToItem(null, item.width / 2, 0).x : 0;
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
        if (movingTile >= 0) {
            return;
        }
        const i = rows.indexOf(from) + delta;
        if (i < 0) {
            searchButton.forceActiveFocus();
        } else if (i < rows.length) {
            focusNear(rows[i], middleOf(from.currentItem));
        }
    }

    function startMoving(tileId) {
        const i = tiles.findIndex(t => t.id === tileId);
        if (i >= 0) {
            movingTile = i;
            focusIndex(quickRow, i);
        }
    }

    // Left or Right while a tile is carried: it swaps places with its neighbour ("Add site" stays last).
    function carry(to) {
        if (to < 0 || to >= tiles.length - 1) {
            return;
        }
        const t = tiles.slice();
        const moved = t.splice(movingTile, 1)[0];
        t.splice(to, 0, moved);
        tiles = t;
        movingTile = to;
        focusIndex(quickRow, to);
    }

    function drop() {
        Library.reorderTiles(JSON.stringify(tiles.filter(t => t.kind === "site").map(t => t.id)));
        movingTile = -1;
    }

    // A private session's start page looks apart, so it is never mistaken for the normal one.
    background: BigscreenUI.AmbientBackground {
        color: page.privateMode ? Qt.darker(Kirigami.Theme.backgroundColor, 1.6) : Kirigami.Theme.backgroundColor
        tint: page.privateMode ? "#8e44ad" : Kirigami.Theme.highlightColor
    }

    // In a small window the top buttons show only their icons, so they all fit.
    readonly property bool compact: width < Kirigami.Units.gridUnit * 80

    component TopButton: BigscreenUI.Button {
        property string label
        Layout.alignment: Qt.AlignVCenter
        flat: true
        text: page.compact ? "" : label
        Accessible.name: label
    }

    component CardRail: BigscreenUI.ContentRail {
        id: rail
        Keys.onUpPressed: page.moveRow(rail, -1)
        Keys.onDownPressed: page.moveRow(rail, 1)
    }

    component Card: SiteCard {
        required property var modelData
        required property int index
        title: modelData.title
        url: modelData.url || ""
        detail: modelData.detail || ""
        iconName: modelData.icon || ""
        holdable: modelData.holdable === true
    }

    BigscreenUI.RailView {
        id: rails
        anchors.fill: parent
        anchors.topMargin: page.contentMargins.top - topMargin
        bottomMargin: page.contentMargins.bottom
        focus: true

        FocusScope {
            id: top
            implicitHeight: topColumn.implicitHeight + Kirigami.Units.gridUnit

            ColumnLayout {
                id: topColumn
                x: page.contentMargins.left
                width: parent.width - x - page.contentMargins.right
                spacing: Kirigami.Units.gridUnit

                RowLayout {
                    Layout.fillWidth: true
                    spacing: Kirigami.Units.gridUnit
                    BigscreenUI.KeyNavigationGroup.orientation: Qt.Horizontal
                    Keys.onDownPressed: if (page.rows.length > 0) page.focusNear(page.rows[0], page.middleOf(page.Window.activeFocusItem))

                    Image {
                        Layout.alignment: Qt.AlignVCenter
                        source: "qrc:/sofa/icon.svg"
                        sourceSize.width: Kirigami.Units.iconSizes.huge
                        sourceSize.height: Kirigami.Units.iconSizes.huge
                        Accessible.ignored: true
                    }

                    BigscreenUI.ButtonDelegate {
                        id: searchButton
                        Layout.fillWidth: true
                        Layout.minimumWidth: Kirigami.Units.gridUnit * 14
                        Layout.maximumWidth: Kirigami.Units.gridUnit * 30
                        focus: true
                        icon.name: page.privateMode ? "view-private" : "search"
                        text: qsTr("Search or type a web address")
                        textFont.pixelSize: BigscreenUI.Units.headingFontPixelSize
                        onClicked: page.search()
                    }

                    TopButton {
                        icon.name: "bookmarks"
                        label: qsTr("Bookmarks")
                        onClicked: page.openBookmarks(0)
                    }
                    TopButton {
                        icon.name: "view-history"
                        label: qsTr("History")
                        onClicked: page.openHistory()
                    }
                    TopButton {
                        icon.name: "view-private"
                        label: page.privateMode ? qsTr("Private: on") : qsTr("Private")
                        checked: page.privateMode
                        onClicked: page.togglePrivate()
                    }
                    TopButton {
                        icon.name: "settings-configure"
                        label: qsTr("Settings")
                        onClicked: page.openSettings()
                    }
                    TopButton {
                        icon.name: "window-close-symbolic"
                        label: qsTr("Exit")
                        onClicked: page.exitRequested()
                    }
                }

                BigscreenUI.Label {
                    Layout.fillWidth: true
                    visible: page.privateMode
                    text: qsTr("Private browsing: no history is kept, and cookies and site data are erased when you leave it.")
                    wrapMode: Text.Wrap
                    color: Kirigami.Theme.disabledTextColor
                }
            }
        }

        CardRail {
            id: quickRow
            title: qsTr("Quick access")
            model: page.tiles
            delegate: Card {
                moving: index === page.movingTile
                onClicked: page.movingTile < 0 ? page.open(quickRow, modelData, index) : page.drop()
                onHeld: page.tileHeld(modelData)
                Keys.onLeftPressed: event => {
                    event.accepted = page.movingTile >= 0;
                    if (event.accepted) page.carry(page.movingTile - 1);
                }
                Keys.onRightPressed: event => {
                    event.accepted = page.movingTile >= 0;
                    if (event.accepted) page.carry(page.movingTile + 1);
                }
            }
        }

        CardRail {
            id: mostRow
            title: qsTr("Most visited")
            model: page.mostVisited
            delegate: Card {
                onClicked: page.open(mostRow, modelData, index)
            }
        }

        CardRail {
            id: bookmarkRow
            title: qsTr("Bookmarks")
            model: page.bookmarkCards
            delegate: Card {
                onClicked: page.open(bookmarkRow, modelData, index)
            }
        }
    }
}
