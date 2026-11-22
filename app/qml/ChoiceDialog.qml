// SPDX-License-Identifier: GPL-3.0-only
//
// A question with a list of answers, on KDE's TV dialog: Up and Down between answers, OK picks one, Back cancels.
// Answers that can't be picked show greyed and are skipped. Also used for a plain message (one "OK" answer).

import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import org.kde.bigscreen.ui as BigscreenUI

BigscreenUI.Dialog {
    id: dialog

    property string message: ""
    // [{text, detail, value, enabled (default true), depth (indent, default 0)}]
    property var options: []
    // The answer focused first.
    property int initialIndex: 0
    // Called with the chosen answer's value; replaced each time the dialog is asked.
    property var choiceHandler: null
    property bool picked: false

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

    // The answers are the buttons: no row of buttons under them.
    footer: null
    openFocusItem: list
    onAboutToShow: picked = false
    onOpened: {
        list.currentIndex = initialIndex;
        list.positionViewAtIndex(initialIndex, ListView.Center);
    }
    onClosed: if (!picked) cancelled()

    contentItem: ColumnLayout {
        spacing: Kirigami.Units.largeSpacing

        BigscreenUI.Label {
            Layout.fillWidth: true
            visible: dialog.message !== ""
            text: dialog.message
            wrapMode: Text.Wrap
            color: Kirigami.Theme.disabledTextColor
        }

        ListView {
            id: list
            Layout.fillWidth: true
            Layout.preferredHeight: Math.min(contentHeight, dialog.parent ? dialog.parent.height * 0.6 : contentHeight)
            clip: true
            model: dialog.options
            keyNavigationEnabled: true
            spacing: Kirigami.Units.smallSpacing
            boundsBehavior: Flickable.StopAtBounds
            highlightRangeMode: ListView.ApplyRange
            preferredHighlightBegin: Kirigami.Units.gridUnit * 2
            preferredHighlightEnd: height - Kirigami.Units.gridUnit * 2
            Accessible.role: Accessible.List

            // Skips answers that can't be picked.
            Keys.onUpPressed: move(-1)
            Keys.onDownPressed: move(1)
            function move(delta) {
                let i = currentIndex + delta;
                while (i >= 0 && i < count && dialog.options[i].enabled === false) {
                    i += delta;
                }
                if (i >= 0 && i < count) {
                    currentIndex = i;
                }
            }

            delegate: ListRow {
                required property var modelData
                required property int index
                width: ListView.view.width
                focus: ListView.isCurrentItem
                showIcon: false
                title: modelData.text
                detail: modelData.detail || ""
                enabled: modelData.enabled !== false
                leftPadding: horizontalPadding + (modelData.depth || 0) * Kirigami.Units.gridUnit
                // A choice's detail is a sentence: it may take two lines.
                descriptionItem.wrapMode: Text.Wrap
                descriptionItem.maximumLineCount: 3
                descriptionItem.elide: Text.ElideRight
                onActivated: dialog.pick(index)
            }
        }
    }
}
