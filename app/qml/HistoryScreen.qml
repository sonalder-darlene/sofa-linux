// SPDX-License-Identifier: GPL-3.0-only
//
// History, newest first, under a heading per day. OK opens a page; OK held or Menu offers Delete; the header has
// Delete all. Private browsing adds nothing here.

import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import org.kde.bigscreen.ui as BigscreenUI
import Sofa

BigscreenUI.Page {
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

    background: BigscreenUI.AmbientBackground {}

    BigscreenUI.BackHandler.enabled: visible
    BigscreenUI.BackHandler.onActivated: screen.back()

    ColumnLayout {
        anchors.fill: parent
        anchors.leftMargin: screen.contentMargins.left
        anchors.rightMargin: screen.contentMargins.right
        anchors.topMargin: screen.contentMargins.top
        spacing: Kirigami.Units.gridUnit

        RowLayout {
            Layout.fillWidth: true
            spacing: Kirigami.Units.gridUnit

            BigscreenUI.Label {
                Layout.fillWidth: true
                text: qsTr("History")
                type: BigscreenUI.Label.Title
                Accessible.role: Accessible.Heading
            }
            BigscreenUI.Button {
                id: deleteAllButton
                visible: screen.visits.length > 0
                icon.name: "edit-delete"
                text: qsTr("Delete all")
                Keys.onDownPressed: list.forceActiveFocus()
                onClicked: choice.ask(qsTr("Delete all of History?"), "",
                                      [{ text: qsTr("Cancel"), value: false }, { text: qsTr("Delete all"), value: true }],
                                      yes => {
                                          if (yes) {
                                              Library.clearHistory();
                                              screen.load(0);
                                          }
                                      }, 0)
            }
        }

        BigscreenUI.PlaceholderMessage {
            Layout.alignment: Qt.AlignHCenter
            Layout.topMargin: Kirigami.Units.gridUnit * 4
            Layout.preferredWidth: implicitWidth
            visible: screen.visits.length === 0
            iconName: "view-history"
            text: qsTr("No history")
            explanation: qsTr("Pages you open show here, kept on this computer only.")
        }

        ListView {
            id: list
            Layout.fillWidth: true
            Layout.fillHeight: true
            // The focused row's outline needs a little room at the sides.
            Layout.leftMargin: -Kirigami.Units.smallSpacing
            Layout.rightMargin: -Kirigami.Units.smallSpacing
            visible: screen.visits.length > 0
            model: screen.visits
            keyNavigationEnabled: true
            clip: true
            spacing: Kirigami.Units.smallSpacing
            reuseItems: true
            boundsBehavior: Flickable.StopAtBounds
            highlightRangeMode: ListView.ApplyRange
            preferredHighlightBegin: Kirigami.Units.gridUnit * 3
            preferredHighlightEnd: height - Kirigami.Units.gridUnit * 3
            bottomMargin: screen.contentMargins.bottom
            Accessible.role: Accessible.List
            Accessible.name: qsTr("History")

            Keys.onUpPressed: currentIndex <= 0 ? deleteAllButton.forceActiveFocus() : decrementCurrentIndex()

            section.property: "day"
            section.delegate: BigscreenUI.Label {
                required property string section
                width: ListView.view.width
                text: section
                type: BigscreenUI.Label.Heading
                color: Kirigami.Theme.disabledTextColor
                topPadding: Kirigami.Units.gridUnit
                bottomPadding: Kirigami.Units.smallSpacing
                leftPadding: Kirigami.Units.smallSpacing
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
                trailingText: modelData.time
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
