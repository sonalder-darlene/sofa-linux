// SPDX-License-Identifier: GPL-3.0-only
//
// History, newest first, under a heading per day. OK opens a page; OK held or Menu offers Delete; the header has
// Delete all. Private browsing adds nothing here.

import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import Sofa

FocusScope {
    id: screen

    property var visits: []

    signal openUrl(string url)
    signal back()

    function start() {
        load(0);
    }

    function load(index) {
        const now = new Date();
        const today = new Date(now.getFullYear(), now.getMonth(), now.getDate()).getTime();
        visits = JSON.parse(Library.history(500)).map(v => {
            const d = new Date(v.visited);
            const day = new Date(d.getFullYear(), d.getMonth(), d.getDate()).getTime();
            v.day = day === today ? qsTr("Today")
                  : day === today - 86400000 ? qsTr("Yesterday")
                  : d.toLocaleDateString(Qt.locale(), Locale.LongFormat);
            v.time = d.toLocaleTimeString(Qt.locale(), Locale.ShortFormat);
            return v;
        });
        if (visits.length > 0) {
            list.currentIndex = Math.max(0, Math.min(index, visits.length - 1));
            list.positionViewAtIndex(list.currentIndex, ListView.Contain);
            list.forceActiveFocus();
        } else {
            screen.forceActiveFocus();
        }
    }

    Keys.onBackPressed: screen.back()
    Keys.onEscapePressed: screen.back()

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

            Kirigami.Heading {
                Layout.fillWidth: true
                level: 1
                text: qsTr("History")
                Accessible.role: Accessible.Heading
            }

            TextButton {
                id: deleteAllButton
                visible: screen.visits.length > 0
                text: qsTr("Delete all")
                iconName: "edit-delete"
                Keys.onDownPressed: list.forceActiveFocus()
                onActivated: choice.ask(qsTr("Delete all of History?"), "",
                                        [{ text: qsTr("Cancel"), value: false }, { text: qsTr("Delete all"), value: true }],
                                        yes => {
                                            if (yes) {
                                                Library.clearHistory();
                                                screen.load(0);
                                            }
                                        }, 0)
            }
        }

        QQC2.Label {
            visible: screen.visits.length === 0
            Layout.fillWidth: true
            wrapMode: Text.Wrap
            color: Kirigami.Theme.disabledTextColor
            text: qsTr("No history. Pages you open show here, kept on this computer only.")
        }

        ListView {
            id: list
            Layout.fillWidth: true
            Layout.fillHeight: true
            model: screen.visits
            keyNavigationEnabled: true
            clip: true
            spacing: Kirigami.Units.smallSpacing
            reuseItems: true
            highlightRangeMode: ListView.ApplyRange
            preferredHighlightBegin: Kirigami.Units.gridUnit * 3
            preferredHighlightEnd: height - Kirigami.Units.gridUnit * 3
            Accessible.role: Accessible.List
            Accessible.name: qsTr("History")
            Keys.onUpPressed: event => currentIndex <= 0 ? deleteAllButton.forceActiveFocus() : decrementCurrentIndex()

            section.property: "day"
            section.delegate: Kirigami.Heading {
                required property string section
                width: ListView.view.width
                level: 3
                text: section
                color: Kirigami.Theme.disabledTextColor
                topPadding: Kirigami.Units.largeSpacing
                bottomPadding: Kirigami.Units.smallSpacing
                Accessible.role: Accessible.Heading
            }

            delegate: ListRow {
                required property var modelData
                required property int index
                width: ListView.view.width
                focus: ListView.isCurrentItem
                holdable: true
                title: modelData.title !== "" ? modelData.title : modelData.url
                detail: modelData.url
                url: modelData.url
                trailing: modelData.time
                onActivated: screen.openUrl(modelData.url)
                onHeld: choice.ask(title, "", [{ text: qsTr("Delete"), value: true }], () => {
                    Library.removeVisit(modelData.id);
                    screen.load(index);
                })
            }
        }
    }

    ChoiceDialog {
        id: choice
        onClosed: if (screen.visible) (screen.visits.length > 0 ? list : screen).forceActiveFocus()
    }
}
