// SPDX-License-Identifier: GPL-3.0-only
//
// Search or type an address: a field, and under it the bookmarks and pages from History that match what is typed
// (looked up on this computer only). OK in the field opens what was typed; Down reaches the suggestions.

import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import Sofa

FocusScope {
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

    Keys.onBackPressed: screen.back()
    Keys.onEscapePressed: screen.back()

    Rectangle {
        anchors.fill: parent
        color: Kirigami.Theme.backgroundColor
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: Kirigami.Units.gridUnit * 3
        spacing: Kirigami.Units.gridUnit

        QQC2.TextField {
            id: field
            Layout.fillWidth: true
            font.pointSize: Kirigami.Theme.defaultFont.pointSize * 1.6
            placeholderText: qsTr("Search or type a web address")
            inputMethodHints: Qt.ImhUrlCharactersOnly | Qt.ImhNoPredictiveText | Qt.ImhNoAutoUppercase
            Accessible.name: qsTr("Search or type a web address")
            onTextEdited: screen.update()
            onAccepted: if (text.trim() !== "") screen.openText(text)
            Keys.onDownPressed: if (screen.suggestions.length > 0) {
                list.currentIndex = 0;
                list.forceActiveFocus();
            }
            Keys.onBackPressed: screen.back()
        }

        ListView {
            id: list
            Layout.fillWidth: true
            Layout.fillHeight: true
            model: screen.suggestions
            keyNavigationEnabled: true
            clip: true
            spacing: Kirigami.Units.smallSpacing
            Accessible.role: Accessible.List
            Accessible.name: qsTr("Suggestions")
            Keys.onUpPressed: event => {
                if (currentIndex <= 0) {
                    field.forceActiveFocus();
                } else {
                    decrementCurrentIndex();
                }
            }

            delegate: ListRow {
                required property var modelData
                required property int index
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
