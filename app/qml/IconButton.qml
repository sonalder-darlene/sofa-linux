// SPDX-License-Identifier: GPL-3.0-only
//
// A round icon with its word under it, as on the start page's top bar and in the menu. The word is always there
// (not only on focus), so nothing moves when focus arrives.

import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

Pressable {
    id: button

    property string iconName: ""
    property string text: ""
    // Greyed out and skipped by OK (Back with no page behind, say); it can still take focus, so focus never jumps.
    property bool available: true
    property bool checked: false

    implicitWidth: Kirigami.Units.gridUnit * 5
    implicitHeight: column.implicitHeight
    Accessible.name: text
    Accessible.checkable: checked
    Accessible.checked: checked
    opacity: available ? 1 : 0.4

    ColumnLayout {
        id: column
        anchors.horizontalCenter: parent.horizontalCenter
        spacing: Kirigami.Units.smallSpacing

        Rectangle {
            Layout.alignment: Qt.AlignHCenter
            implicitWidth: Kirigami.Units.gridUnit * 3
            implicitHeight: implicitWidth
            radius: width / 2
            color: button.activeFocus ? Kirigami.Theme.highlightColor
                 : button.checked ? Qt.rgba(Kirigami.Theme.highlightColor.r, Kirigami.Theme.highlightColor.g,
                                            Kirigami.Theme.highlightColor.b, 0.35)
                 : Kirigami.Theme.alternateBackgroundColor
            scale: button.down ? 0.92 : 1

            Kirigami.Icon {
                anchors.centerIn: parent
                width: Kirigami.Units.iconSizes.medium
                height: width
                source: button.iconName
                color: button.activeFocus ? Kirigami.Theme.highlightedTextColor : Kirigami.Theme.textColor
                isMask: true
            }
        }

        Text {
            Layout.alignment: Qt.AlignHCenter
            Layout.maximumWidth: button.implicitWidth
            text: button.text
            color: button.activeFocus ? Kirigami.Theme.textColor : Kirigami.Theme.disabledTextColor
            font.pointSize: Kirigami.Theme.smallFont.pointSize
            horizontalAlignment: Text.AlignHCenter
            elide: Text.ElideRight
        }
    }
}
