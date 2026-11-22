// SPDX-License-Identifier: GPL-3.0-only
//
// A question with a text field (a folder name, a web address), on KDE's TV dialog. OK in the field or on Save answers;
// Down reaches the buttons; Back cancels. The system's on-screen keyboard opens for the field. An extra button can
// offer a ready answer ("Use the open page").

import QtQuick
import QtQuick.Layouts
import QtQuick.Templates as T
import org.kde.kirigami as Kirigami
import org.kde.bigscreen.ui as BigscreenUI

BigscreenUI.Dialog {
    id: textDialog

    property string message: ""
    property string error: ""
    property string extraText: ""
    property string extraValue: ""
    property bool address: false
    property var answerHandler: null

    // Asks `options.title`; `answer(text)` runs on Save and returns "" to close or an error message to keep the
    // dialog open.
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

    openFocusItem: field
    onOpened: {
        field.cursorPosition = field.text.length;
        Qt.inputMethod.show();
    }
    onClosed: Qt.inputMethod.hide()

    contentItem: ColumnLayout {
        spacing: Kirigami.Units.largeSpacing

        BigscreenUI.Label {
            Layout.fillWidth: true
            visible: textDialog.message !== ""
            text: textDialog.message
            wrapMode: Text.Wrap
            color: Kirigami.Theme.disabledTextColor
        }

        BigscreenUI.TextField {
            id: field
            Layout.fillWidth: true
            inputMethodHints: textDialog.address ? (Qt.ImhUrlCharactersOnly | Qt.ImhNoPredictiveText | Qt.ImhNoAutoUppercase)
                                             : Qt.ImhNone
            Accessible.name: textDialog.title
            onAccepted: textDialog.answer(text)
            KeyNavigation.down: textDialog.footer
        }

        BigscreenUI.Label {
            Layout.fillWidth: true
            visible: textDialog.error !== ""
            text: textDialog.error
            wrapMode: Text.Wrap
            color: Kirigami.Theme.negativeTextColor
        }
    }

    // Save answers only when the answer is good, so it is not the dialog's own "accept", which always closes.
    footer: BigscreenUI.DialogButtonBox {
        dialog: textDialog
        standardButtons: T.Dialog.Cancel
        Keys.onUpPressed: field.forceActiveFocus()

        BigscreenUI.Button {
            id: saveButton
            text: qsTr("Save")
            icon.name: "dialog-ok"
            T.DialogButtonBox.buttonRole: T.DialogButtonBox.ActionRole
            onClicked: textDialog.answer(field.text)
        }
        BigscreenUI.Button {
            visible: textDialog.extraText !== ""
            text: textDialog.extraText
            T.DialogButtonBox.buttonRole: T.DialogButtonBox.ActionRole
            onClicked: textDialog.answer(textDialog.extraValue)
        }
    }
}
