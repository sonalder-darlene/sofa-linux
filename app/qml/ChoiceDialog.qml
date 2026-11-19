// SPDX-License-Identifier: GPL-3.0-only
//
// A question with a list of answers, for the remote: Up and Down between answers, OK picks one, Back cancels.
// Answers that can't be picked show greyed and are skipped. Also used for a plain message (one "OK" answer).

import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

QQC2.Popup {
    id: dialog

    property string title: ""
    property string message: ""
    // [{text, detail, value, enabled (default true), depth (indent, default 0)}]
    property var options: []
    // The answer focused first.
    property int initialIndex: 0
    // Called with the chosen answer's value; replaced each time the dialog is asked.
    property var choiceHandler: null

    signal cancelled()

    // Asks `title` with `options`; `choice(value)` runs when one is picked.
    function ask(title, message, options, choice, initial) {
        dialog.title = title;
        dialog.message = message || "";
        dialog.options = options;
        dialog.choiceHandler = choice || null;
        let first = initial !== undefined ? initial : 0;
        while (first < options.length && options[first].enabled === false) {
            first++;
        }
        dialog.initialIndex = first < options.length ? first : 0;
        open();
    }

    function tell(title, message) {
        ask(title, message, [{ text: qsTr("OK"), value: true }], null, 0);
    }

    parent: QQC2.Overlay.overlay
    anchors.centerIn: parent
    width: Math.min(parent ? parent.width - Kirigami.Units.gridUnit * 4 : 600, Kirigami.Units.gridUnit * 34)
    height: Math.min(column.implicitHeight + topPadding + bottomPadding, parent ? parent.height - Kirigami.Units.gridUnit * 4 : 600)
    modal: true
    focus: true
    closePolicy: QQC2.Popup.CloseOnEscape | QQC2.Popup.CloseOnPressOutside
    padding: Kirigami.Units.gridUnit

    onOpened: {
        list.currentIndex = initialIndex;
        list.positionViewAtIndex(initialIndex, ListView.Center);
        list.forceActiveFocus();
    }
    onClosed: if (!picked) cancelled()

    property bool picked: false
    onAboutToShow: picked = false

    function pick(index) {
        const option = options[index];
        if (!option || option.enabled === false) {
            return;
        }
        picked = true;
        const handler = choiceHandler;
        close();
        if (handler) {
            handler(option.value);
        }
    }

    background: Rectangle {
        color: Kirigami.Theme.backgroundColor
        radius: Kirigami.Units.cornerRadius * 2
        border.color: Kirigami.Theme.disabledTextColor
        border.width: 1
    }

    contentItem: ColumnLayout {
        id: column
        spacing: Kirigami.Units.largeSpacing

        Kirigami.Heading {
            Layout.fillWidth: true
            text: dialog.title
            level: 2
            wrapMode: Text.Wrap
            Accessible.role: Accessible.Heading
        }

        QQC2.Label {
            Layout.fillWidth: true
            visible: dialog.message !== ""
            text: dialog.message
            wrapMode: Text.Wrap
            color: Kirigami.Theme.disabledTextColor
        }

        ListView {
            id: list
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.preferredHeight: contentHeight
            Layout.minimumHeight: Math.min(contentHeight, Kirigami.Units.gridUnit * 3)
            clip: true
            model: dialog.options
            keyNavigationEnabled: true
            spacing: Kirigami.Units.smallSpacing
            boundsBehavior: Flickable.StopAtBounds
            Accessible.role: Accessible.List

            // Skips answers that can't be picked.
            Keys.onUpPressed: event => move(-1)
            Keys.onDownPressed: event => move(1)
            function move(delta) {
                let i = currentIndex + delta;
                while (i >= 0 && i < count && dialog.options[i].enabled === false) {
                    i += delta;
                }
                if (i >= 0 && i < count) {
                    currentIndex = i;
                }
            }
            Keys.onBackPressed: dialog.close()

            delegate: Pressable {
                id: option
                required property var modelData
                required property int index
                width: ListView.view.width
                height: Math.max(Kirigami.Units.gridUnit * 2.5, texts.implicitHeight + Kirigami.Units.largeSpacing * 2)
                focus: ListView.isCurrentItem
                Accessible.name: modelData.detail ? modelData.text + ", " + modelData.detail : modelData.text
                Accessible.role: Accessible.ListItem
                onActivated: dialog.pick(index)

                Rectangle {
                    anchors.fill: parent
                    radius: Kirigami.Units.cornerRadius
                    color: option.activeFocus ? Kirigami.Theme.highlightColor : "transparent"
                }

                ColumnLayout {
                    id: texts
                    anchors.verticalCenter: parent.verticalCenter
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.leftMargin: Kirigami.Units.largeSpacing + (option.modelData.depth || 0) * Kirigami.Units.gridUnit
                    anchors.rightMargin: Kirigami.Units.largeSpacing
                    spacing: 0
                    opacity: option.modelData.enabled === false ? 0.4 : 1

                    QQC2.Label {
                        Layout.fillWidth: true
                        text: option.modelData.text
                        elide: Text.ElideRight
                        color: option.activeFocus ? Kirigami.Theme.highlightedTextColor : Kirigami.Theme.textColor
                        font.pointSize: Kirigami.Theme.defaultFont.pointSize * 1.15
                    }
                    QQC2.Label {
                        Layout.fillWidth: true
                        visible: !!option.modelData.detail
                        text: option.modelData.detail || ""
                        wrapMode: Text.Wrap
                        color: option.activeFocus ? Kirigami.Theme.highlightedTextColor : Kirigami.Theme.disabledTextColor
                        font.pointSize: Kirigami.Theme.smallFont.pointSize
                    }
                }
            }
        }
    }
}
