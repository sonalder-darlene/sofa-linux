// SPDX-License-Identifier: GPL-3.0-only
//
// A question with a text field (a folder name, a web address). OK in the field or on Save answers; Down reaches the
// buttons; Back cancels. The system's on-screen keyboard opens for the field. An extra button can offer a ready
// answer ("Use the open page").

import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

QQC2.Popup {
    id: dialog

    property string title: ""
    property string message: ""
    property string error: ""
    property string extraText: ""
    property string extraValue: ""
    property bool address: false
    property var answerHandler: null

    // Asks `title`; `answer(text)` runs on Save and returns "" to close or an error message to keep the dialog open.
    function ask(options, answer) {
        title = options.title || "";
        message = options.message || "";
        field.placeholderText = options.placeholder || "";
        field.text = options.text || "";
        address = options.address === true;
        extraText = options.extraText || "";
        extraValue = options.extraValue || "";
        saveButton.text = options.saveText || qsTr("Save");
        error = "";
        answerHandler = answer;
        open();
    }

    function answer(value) {
        const problem = answerHandler ? answerHandler(value) : "";
        if (problem) {
            error = problem;
            field.forceActiveFocus();
            return;
        }
        close();
    }

    parent: QQC2.Overlay.overlay
    anchors.centerIn: parent
    width: Math.min(parent ? parent.width - Kirigami.Units.gridUnit * 4 : 600, Kirigami.Units.gridUnit * 36)
    modal: true
    focus: true
    closePolicy: QQC2.Popup.CloseOnEscape | QQC2.Popup.CloseOnPressOutside
    padding: Kirigami.Units.gridUnit

    onOpened: {
        field.forceActiveFocus();
        field.cursorPosition = field.text.length;
        Qt.inputMethod.show();
    }
    onClosed: Qt.inputMethod.hide()

    background: Rectangle {
        color: Kirigami.Theme.backgroundColor
        radius: Kirigami.Units.cornerRadius * 2
        border.color: Kirigami.Theme.disabledTextColor
        border.width: 1
    }

    contentItem: ColumnLayout {
        spacing: Kirigami.Units.largeSpacing

        Kirigami.Heading {
            Layout.fillWidth: true
            text: dialog.title
            level: 2
            wrapMode: Text.Wrap
        }

        QQC2.Label {
            Layout.fillWidth: true
            visible: dialog.message !== ""
            text: dialog.message
            wrapMode: Text.Wrap
            color: Kirigami.Theme.disabledTextColor
        }

        QQC2.TextField {
            id: field
            Layout.fillWidth: true
            font.pointSize: Kirigami.Theme.defaultFont.pointSize * 1.3
            inputMethodHints: dialog.address ? (Qt.ImhUrlCharactersOnly | Qt.ImhNoPredictiveText | Qt.ImhNoAutoUppercase)
                                             : Qt.ImhNone
            Accessible.name: dialog.title
            onAccepted: dialog.answer(text)
            KeyNavigation.down: dialog.extraText !== "" ? extraButton : saveButton
            Keys.onBackPressed: dialog.close()
        }

        QQC2.Label {
            Layout.fillWidth: true
            visible: dialog.error !== ""
            text: dialog.error
            wrapMode: Text.Wrap
            color: Kirigami.Theme.negativeTextColor
        }

        RowLayout {
            Layout.alignment: Qt.AlignRight
            spacing: Kirigami.Units.largeSpacing

            QQC2.Button {
                id: extraButton
                visible: dialog.extraText !== ""
                text: dialog.extraText
                KeyNavigation.up: field
                KeyNavigation.right: saveButton
                onClicked: dialog.answer(dialog.extraValue)
                Keys.onReturnPressed: clicked()
                Keys.onEnterPressed: clicked()
                Keys.onBackPressed: dialog.close()
            }
            QQC2.Button {
                id: saveButton
                text: qsTr("Save")
                highlighted: true
                KeyNavigation.up: field
                KeyNavigation.right: cancelButton
                onClicked: dialog.answer(field.text)
                Keys.onReturnPressed: clicked()
                Keys.onEnterPressed: clicked()
                Keys.onBackPressed: dialog.close()
            }
            QQC2.Button {
                id: cancelButton
                text: qsTr("Cancel")
                KeyNavigation.up: field
                onClicked: dialog.close()
                Keys.onReturnPressed: clicked()
                Keys.onEnterPressed: clicked()
                Keys.onBackPressed: dialog.close()
            }
        }
    }
}
