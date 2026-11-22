// SPDX-License-Identifier: GPL-3.0-only
//
// Search or type an address: a field, and under it the bookmarks and pages from History that match what is typed
// (looked up on this computer only). OK in the field opens what was typed; Down reaches the suggestions.

import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import org.kde.bigscreen.ui as BigscreenUI
import Sofa

BigscreenUI.Page {
    id: screen

    property var suggestions: []

    signal openText(string text)
    signal openUrl(string url)
    signal back()

    // Shows the screen with `text` in the field (the page's address, all selected, or nothing).
    function start(text) {
        field.text = text;
        field.selectAll();
        field.forceActiveFocus();
        Qt.inputMethod.show();
        update();
    }

    function update() {
        suggestions = field.text.trim() === "" ? [] : JSON.parse(Library.suggest(field.text));
    }

    background: BigscreenUI.AmbientBackground {
        intensity: 0.3
    }

    BigscreenUI.BackHandler.enabled: visible
    BigscreenUI.BackHandler.onActivated: screen.back()

    ColumnLayout {
        anchors.fill: parent
        anchors.leftMargin: Math.max(screen.contentMargins.left, (parent.width - Kirigami.Units.gridUnit * 60) / 2)
        anchors.rightMargin: anchors.leftMargin
        anchors.topMargin: screen.contentMargins.top + Kirigami.Units.gridUnit
        anchors.bottomMargin: screen.contentMargins.bottom
        spacing: Kirigami.Units.gridUnit

        BigscreenUI.Label {
            Layout.fillWidth: true
            text: qsTr("Search or type a web address")
            type: BigscreenUI.Label.Title
            Accessible.ignored: true
        }

        BigscreenUI.TextField {
            id: field
            Layout.fillWidth: true
            font.pixelSize: BigscreenUI.Units.sectionFontPixelSize
            leftPadding: Kirigami.Units.gridUnit * 3
            inputMethodHints: Qt.ImhUrlCharactersOnly | Qt.ImhNoPredictiveText | Qt.ImhNoAutoUppercase
            EnterKey.type: Qt.EnterKeyGo
            Accessible.name: qsTr("Search or type a web address")
            onTextEdited: screen.update()
            onAccepted: if (text.trim() !== "") screen.openText(text)
            Keys.onDownPressed: if (screen.suggestions.length > 0) {
                list.currentIndex = 0;
                list.forceActiveFocus();
            }

            Kirigami.Icon {
                anchors.left: parent.left
                anchors.leftMargin: Kirigami.Units.gridUnit
                anchors.verticalCenter: parent.verticalCenter
                width: Kirigami.Units.iconSizes.smallMedium
                height: width
                source: "search"
                color: Kirigami.Theme.disabledTextColor
            }
        }

        ListView {
            id: list
            Layout.fillWidth: true
            Layout.fillHeight: true
            model: screen.suggestions
            keyNavigationEnabled: true
            clip: true
            spacing: Kirigami.Units.smallSpacing
            boundsBehavior: Flickable.StopAtBounds
            Accessible.role: Accessible.List
            Accessible.name: qsTr("Suggestions")

            Keys.onUpPressed: currentIndex <= 0 ? field.forceActiveFocus() : decrementCurrentIndex()

            delegate: ListRow {
                required property var modelData
                width: ListView.view.width
                focus: ListView.isCurrentItem
                title: modelData.title
                detail: modelData.url
                url: modelData.url
                onActivated: screen.openUrl(modelData.url)
            }
        }
    }
}
