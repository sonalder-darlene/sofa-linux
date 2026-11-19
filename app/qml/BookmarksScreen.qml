// SPDX-License-Identifier: GPL-3.0-only
//
// Bookmarks, in folders: a folder's own folders (A to Z) and then its bookmarks (newest first), with an Up row inside
// a folder. OK opens; OK held or Menu lists what can be done (folder: Move, Rename, Delete; bookmark: Move, Delete).
// Back retraces every step taken in the screen, then leaves it. The header has New folder, Import or export and
// Delete all. Rows are made only for what is on screen, so thousands of bookmarks open at once.

import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Dialogs
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import Sofa
import "words.js" as Words

FocusScope {
    id: screen

    property int folderId: 0
    property var folder: ({ id: 0, title: "", path: [], folders: [], bookmarks: [] })
    property var rows: []
    // Every step taken here, for Back: {folder, index}.
    property var trail: []

    signal openUrl(string url)
    signal back()

    function start(id) {
        trail = [];
        load(id, -1);
    }

    // Shows folder `id` with row `index` in focus (-1: the first row that isn't Up).
    function load(id, index) {
        folder = JSON.parse(Library.folder(id));
        folderId = folder.id;
        const r = [];
        if (folderId !== 0) {
            const parent = folder.path.length > 1 ? folder.path[folder.path.length - 2] : null;
            r.push({ kind: "up", id: parent ? parent.id : 0,
                     title: qsTr("Up to %1").arg(parent ? parent.title : qsTr("Bookmarks")), icon: "go-up" });
        }
        for (const f of folder.folders) {
            r.push({ kind: "folder", id: f.id, title: f.title, icon: "folder", detail: contents(f.folders, f.bookmarks) });
        }
        for (const b of folder.bookmarks) {
            r.push({ kind: "bookmark", id: b.id, title: b.title, url: b.url, detail: b.url });
        }
        rows = r;
        const first = folderId !== 0 && r.length > 1 ? 1 : 0;
        const i = index < 0 ? first : Math.min(index, r.length - 1);
        if (r.length > 0) {
            list.currentIndex = i;
            list.positionViewAtIndex(i, ListView.Contain);
            list.forceActiveFocus();
        } else {
            headerButtons.children[0].forceActiveFocus();
        }
    }

    function contents(folders, bookmarks) {
        if (folders === 0 && bookmarks === 0) {
            return qsTr("Empty");
        }
        const parts = [];
        if (folders > 0) parts.push(Words.count(folders, qsTr("%n folder"), qsTr("%n folders")));
        if (bookmarks > 0) parts.push(Words.count(bookmarks, qsTr("%n bookmark"), qsTr("%n bookmarks")));
        return parts.join(", ");
    }

    function go(id, focusRowOf) {
        trail.push({ folder: folderId, index: list.currentIndex });
        load(id, -1);
        if (focusRowOf !== undefined) {
            const i = rows.findIndex(r => r.kind === "folder" && r.id === focusRowOf);
            if (i >= 0) {
                list.currentIndex = i;
                list.positionViewAtIndex(i, ListView.Contain);
            }
        }
    }

    function goBack() {
        if (trail.length === 0) {
            screen.back();
            return;
        }
        const step = trail.pop();
        load(step.folder, step.index);
    }

    function reload(focusIndex) {
        load(folderId, Math.max(0, Math.min(focusIndex, rows.length)));
    }

    function activate(row) {
        if (row.kind === "up") {
            go(row.id, folderId);
        } else if (row.kind === "folder") {
            go(row.id);
        } else {
            screen.openUrl(row.url);
        }
    }

    function actions(row, index) {
        if (row.kind === "up") {
            return;
        }
        const options = [{ text: qsTr("Move"), value: "move" }];
        if (row.kind === "folder") {
            options.push({ text: qsTr("Rename"), value: "rename" });
        }
        options.push({ text: qsTr("Delete"), value: "delete" });
        choice.ask(row.title, "", options, value => {
            if (value === "move") {
                askMove(row, index);
            } else if (value === "rename") {
                askRename(row, index);
            } else {
                askDelete(row, index);
            }
        });
    }

    function askMove(row, index) {
        const isFolder = row.kind === "folder";
        const targets = JSON.parse(Library.moveTargets(isFolder, row.id));
        const options = targets.map(t => ({
            text: (t.id === 0 ? qsTr("Bookmarks (top level)") : t.title) + (t.here ? "   " + qsTr("here now") : ""),
            value: t.id,
            depth: t.depth,
            enabled: t.allowed && !t.here && !(isFolder && t.id === row.id)
        }));
        options.push({ text: qsTr("New folder…"), value: -1, depth: 0 });
        const here = targets.findIndex(t => t.here);
        choice.ask(qsTr("Move %1 to").arg(row.title), "", options, to => {
            if (to === -1) {
                askNewFolder(folderId, made => moveInto(row, made, index));
            } else {
                moveInto(row, to, index);
            }
        }, here >= 0 ? here : 0);
    }

    function moveInto(row, to, index) {
        if (row.kind === "folder") {
            if (!Library.moveFolder(row.id, to)) {
                choice.tell(qsTr("This folder can't go there"), "");
                return;
            }
        } else {
            Library.moveBookmark(row.id, to);
        }
        reload(index);
    }

    function askRename(row, index) {
        text.ask({ title: qsTr("Rename folder"), text: row.title }, typed => {
            const problem = Library.renameFolder(row.id, typed);
            if (problem === "exists") {
                return qsTr("There is already a folder called %1 here.").arg(typed.trim());
            }
            load(folderId, -1);
            const i = rows.findIndex(r => r.kind === "folder" && r.id === row.id);
            if (i >= 0) {
                list.currentIndex = i;
                list.positionViewAtIndex(i, ListView.Contain);
            }
            return "";
        });
    }

    function askDelete(row, index) {
        let question = qsTr("Delete %1?").arg(row.title);
        if (row.kind === "folder") {
            const c = JSON.parse(Library.folderContents(row.id));
            if (c.folders > 0 || c.bookmarks > 0) {
                question = qsTr("Delete %1 and what is in it (%2)?").arg(row.title).arg(contents(c.folders, c.bookmarks));
            }
        }
        choice.ask(question, "", [{ text: qsTr("Cancel"), value: false }, { text: qsTr("Delete"), value: true }], yes => {
            if (!yes) {
                return;
            }
            if (row.kind === "folder") {
                Library.deleteFolder(row.id);
            } else {
                Library.deleteBookmark(row.id);
            }
            reload(index);
        }, 0);
    }

    // Asks a name and makes a folder in `parent`; `made(id)` runs with the new folder.
    function askNewFolder(parent, made) {
        text.ask({ title: qsTr("New folder"), placeholder: qsTr("New folder") }, typed => {
            const name = typed.trim() === "" ? qsTr("New folder") : typed;
            const result = JSON.parse(Library.addFolder(parent, name));
            if (result.error === "exists") {
                return qsTr("There is already a folder called %1 here.").arg(name.trim());
            }
            if (result.error === "deep") {
                return qsTr("Folders can't go deeper than this.");
            }
            if (result.error) {
                return qsTr("The folder could not be made.");
            }
            made(result.id);
            return "";
        });
    }

    function importOrExport() {
        choice.ask(qsTr("Import or export"), "", [
            { text: qsTr("Import bookmarks"), detail: qsTr("From a bookmarks file another browser saved"), value: "import" },
            { text: qsTr("Export bookmarks"), detail: qsTr("Into your Downloads folder, for another browser or Sofa"), value: "export" }
        ], value => value === "import" ? askImportSource() : exportNow());
    }

    function askImportSource() {
        const files = JSON.parse(Library.downloadedFiles());
        const options = files.map(f => ({ text: f.name, detail: qsTr("In your Downloads folder"), value: f.path }));
        options.push({ text: qsTr("Choose a file…"), value: "" });
        choice.ask(qsTr("Import bookmarks"),
                   qsTr("Export your bookmarks from the other browser as an HTML file, then pick it here. Bookmarks already in Sofa are kept as they are."),
                   options, path => path === "" ? fileDialog.open() : importNow(path));
    }

    function importNow(path) {
        const r = JSON.parse(Library.importFile(path));
        if (r.error) {
            const why = {
                unreadable: qsTr("The file could not be read."),
                large: qsTr("This file is too large to be a bookmarks file."),
                empty: qsTr("No bookmarks were found in this file."),
                failed: qsTr("Sofa could not save the bookmarks.")
            };
            choice.tell(qsTr("Nothing imported"), why[r.error] || why.failed);
            return;
        }
        const lines = [r.added > 0 ? (r.folders > 0 ? Words.count(r.added, qsTr("Added %n bookmark"), qsTr("Added %n bookmarks")) + " " + Words.count(r.folders, qsTr("in %n folder."), qsTr("in %n folders."))
                                                    : Words.count(r.added, qsTr("Added %n bookmark."), qsTr("Added %n bookmarks.")))
                                   : qsTr("No new bookmarks.")];
        if (r.already > 0) lines.push(Words.count(r.already, qsTr("%n was already there."), qsTr("%n were already there.")));
        if (r.skipped > 0) lines.push(Words.count(r.skipped, qsTr("%n link was left out: Sofa keeps web addresses only."), qsTr("%n links were left out: Sofa keeps web addresses only.")));
        choice.tell(qsTr("Import done"), lines.join("\n"));
        load(folderId, -1);
    }

    function exportNow() {
        const r = JSON.parse(Library.exportFile());
        if (r.error) {
            choice.tell(qsTr("Nothing exported"), qsTr("The bookmarks file could not be written."));
            return;
        }
        choice.tell(qsTr("Bookmarks exported"),
                    Words.count(r.count, qsTr("%n bookmark saved as %1 in %2."), qsTr("%n bookmarks saved as %1 in %2.")).arg(r.name).arg(r.folder));
    }

    function askDeleteAll() {
        choice.ask(qsTr("Delete all bookmarks and folders?"), "",
                   [{ text: qsTr("Cancel"), value: false }, { text: qsTr("Delete all"), value: true }], yes => {
            if (yes) {
                Library.clearBookmarks();
                trail = [];
                load(0, -1);
            }
        }, 0);
    }

    Keys.onBackPressed: goBack()
    Keys.onEscapePressed: goBack()

    Rectangle {
        anchors.fill: parent
        color: Kirigami.Theme.backgroundColor
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: Kirigami.Units.gridUnit * 2
        anchors.leftMargin: Kirigami.Units.gridUnit * 3
        anchors.rightMargin: Kirigami.Units.gridUnit * 3
        spacing: Kirigami.Units.gridUnit

        RowLayout {
            Layout.fillWidth: true
            spacing: Kirigami.Units.largeSpacing

            Kirigami.Heading {
                Layout.fillWidth: true
                level: 1
                elide: Text.ElideLeft
                text: [qsTr("Bookmarks")].concat(screen.folder.path.map(p => p.title)).join("  ›  ")
                Accessible.role: Accessible.Heading
            }

            Row {
                id: headerButtons
                spacing: Kirigami.Units.largeSpacing
                Keys.onDownPressed: if (screen.rows.length > 0) list.forceActiveFocus()

                TextButton {
                    id: newFolderButton
                    text: qsTr("New folder")
                    iconName: "folder-new"
                    KeyNavigation.right: transferButton
                    onActivated: screen.askNewFolder(screen.folderId, id => {
                        screen.load(screen.folderId, -1);
                        const i = screen.rows.findIndex(r => r.kind === "folder" && r.id === id);
                        if (i >= 0) {
                            list.currentIndex = i;
                            list.forceActiveFocus();
                        }
                    })
                }
                TextButton {
                    id: transferButton
                    text: qsTr("Import or export")
                    iconName: "document-import"
                    KeyNavigation.right: deleteAllButton
                    onActivated: screen.importOrExport()
                }
                TextButton {
                    id: deleteAllButton
                    visible: screen.folderId === 0 && screen.rows.length > 0
                    text: qsTr("Delete all")
                    iconName: "edit-delete"
                    onActivated: screen.askDeleteAll()
                }
            }
        }

        QQC2.Label {
            visible: screen.rows.length === 0
            Layout.fillWidth: true
            wrapMode: Text.Wrap
            color: Kirigami.Theme.disabledTextColor
            text: qsTr("No bookmarks yet. Bookmark a page from the menu, or import them from another browser.")
        }

        ListView {
            id: list
            Layout.fillWidth: true
            Layout.fillHeight: true
            model: screen.rows
            keyNavigationEnabled: true
            clip: true
            spacing: Kirigami.Units.smallSpacing
            highlightMoveDuration: Kirigami.Units.shortDuration
            preferredHighlightBegin: Kirigami.Units.gridUnit * 3
            preferredHighlightEnd: height - Kirigami.Units.gridUnit * 3
            highlightRangeMode: ListView.ApplyRange
            reuseItems: true
            Accessible.role: Accessible.List
            Accessible.name: qsTr("Bookmarks")
            Keys.onUpPressed: event => currentIndex <= 0 ? newFolderButton.forceActiveFocus() : decrementCurrentIndex()
            // Channel and page keys move a screen at a time.
            Keys.onPressed: event => {
                const page = Math.max(1, Math.floor(height / (Kirigami.Units.gridUnit * 3.5)) - 1);
                if (event.key === Qt.Key_PageDown || event.key === Qt.Key_ChannelDown) {
                    currentIndex = Math.min(count - 1, currentIndex + page);
                    event.accepted = true;
                } else if (event.key === Qt.Key_PageUp || event.key === Qt.Key_ChannelUp) {
                    currentIndex = Math.max(0, currentIndex - page);
                    event.accepted = true;
                }
            }

            delegate: ListRow {
                required property var modelData
                required property int index
                width: ListView.view.width
                focus: ListView.isCurrentItem
                slim: modelData.kind === "up"
                holdable: modelData.kind !== "up"
                title: modelData.title
                detail: modelData.detail || ""
                url: modelData.url || ""
                iconName: modelData.icon || ""
                Accessible.name: modelData.kind === "folder" ? qsTr("%1, folder, %2").arg(modelData.title).arg(modelData.detail)
                                                              : (modelData.detail ? modelData.title + ", " + modelData.detail : modelData.title)
                onActivated: screen.activate(modelData)
                onHeld: screen.actions(modelData, index)
            }
        }
    }

    ChoiceDialog {
        id: choice
        onClosed: if (screen.visible && !text.visible) (screen.rows.length > 0 ? list : newFolderButton).forceActiveFocus()
    }

    TextDialog {
        id: text
        onClosed: if (screen.visible && !choice.visible) (screen.rows.length > 0 ? list : newFolderButton).forceActiveFocus()
    }

    FileDialog {
        id: fileDialog
        title: qsTr("Import bookmarks")
        nameFilters: [qsTr("Bookmarks files (*.html *.htm)")]
        onAccepted: screen.importNow(selectedFile.toString())
    }
}
