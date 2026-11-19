// SPDX-License-Identifier: GPL-3.0-only
//
// A row in a list (Bookmarks, History, suggestions): an icon, a title and a line under it. The row in focus takes
// the theme's highlight.

import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

Pressable {
    id: row

    property string title: ""
    property string detail: ""
    property string url: ""
    property string iconName: ""
    // A slimmer row (the Up row in a folder), so it reads as a way out rather than an item.
    property bool slim: false
    property string trailing: ""

    height: slim ? Kirigami.Units.gridUnit * 2.5 : Kirigami.Units.gridUnit * 3.5
    Accessible.name: detail !== "" ? title + ", " + detail : title
    Accessible.role: Accessible.ListItem

    Rectangle {
        anchors.fill: parent
        radius: Kirigami.Units.cornerRadius
        color: row.activeFocus ? Kirigami.Theme.highlightColor : "transparent"
    }

    RowLayout {
        anchors.fill: parent
        anchors.leftMargin: Kirigami.Units.largeSpacing
        anchors.rightMargin: Kirigami.Units.largeSpacing
        spacing: Kirigami.Units.largeSpacing

        SiteIcon {
            Layout.preferredWidth: row.slim ? Kirigami.Units.iconSizes.smallMedium : Kirigami.Units.iconSizes.medium
            Layout.preferredHeight: Layout.preferredWidth
            url: row.url
            title: row.title
            iconName: row.iconName
        }

        ColumnLayout {
            Layout.fillWidth: true
            spacing: 0

            QQC2.Label {
                Layout.fillWidth: true
                text: row.title
                elide: Text.ElideRight
                font.pointSize: Kirigami.Theme.defaultFont.pointSize * 1.15
                color: row.activeFocus ? Kirigami.Theme.highlightedTextColor : Kirigami.Theme.textColor
            }
            QQC2.Label {
                Layout.fillWidth: true
                visible: row.detail !== ""
                text: row.detail
                elide: Text.ElideMiddle
                font.pointSize: Kirigami.Theme.smallFont.pointSize
                color: row.activeFocus ? Kirigami.Theme.highlightedTextColor : Kirigami.Theme.disabledTextColor
            }
        }

        QQC2.Label {
            visible: row.trailing !== ""
            text: row.trailing
            color: row.activeFocus ? Kirigami.Theme.highlightedTextColor : Kirigami.Theme.disabledTextColor
        }
    }
}
