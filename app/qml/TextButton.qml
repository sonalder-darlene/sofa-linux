// SPDX-License-Identifier: GPL-3.0-only
//
// A labelled button for a screen's header ("New folder", "Delete all"): on a TV a word is easier to find than an
// icon alone.

import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

Pressable {
    id: button

    property string text: ""
    property string iconName: ""

    implicitWidth: row.implicitWidth + Kirigami.Units.gridUnit * 2
    implicitHeight: Kirigami.Units.gridUnit * 2.5
    Accessible.name: text

    Rectangle {
        anchors.fill: parent
        radius: height / 2
        color: button.activeFocus ? Kirigami.Theme.highlightColor : Kirigami.Theme.alternateBackgroundColor
        scale: button.down ? 0.95 : 1
    }

    RowLayout {
        id: row
        anchors.centerIn: parent
        spacing: Kirigami.Units.smallSpacing

        Kirigami.Icon {
            visible: button.iconName !== ""
            source: button.iconName
            Layout.preferredWidth: Kirigami.Units.iconSizes.small
            Layout.preferredHeight: Kirigami.Units.iconSizes.small
            color: button.activeFocus ? Kirigami.Theme.highlightedTextColor : Kirigami.Theme.textColor
            isMask: true
        }
        QQC2.Label {
            text: button.text
            color: button.activeFocus ? Kirigami.Theme.highlightedTextColor : Kirigami.Theme.textColor
        }
    }
}
