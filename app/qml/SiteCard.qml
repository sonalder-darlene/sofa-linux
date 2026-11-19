// SPDX-License-Identifier: GPL-3.0-only
//
// A card on the start page: a site (icon and name), a folder, or an action such as "Add site". The card in focus
// grows and gets the theme's highlight outline, so it can be seen from the sofa.

import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

Pressable {
    id: card

    property string title: ""
    property string url: ""
    property string detail: ""
    property string iconName: ""
    // Being carried with Left and Right (Quick access "Move").
    property bool moving: false

    width: Kirigami.Units.gridUnit * 9
    height: Kirigami.Units.gridUnit * 8
    scale: activeFocus ? 1.08 : 1
    z: activeFocus ? 1 : 0
    Accessible.name: detail !== "" ? title + ", " + detail : title

    Behavior on scale {
        NumberAnimation { duration: Kirigami.Units.shortDuration }
    }

    Rectangle {
        anchors.fill: parent
        radius: Kirigami.Units.cornerRadius * 2
        color: card.activeFocus ? Qt.lighter(Kirigami.Theme.alternateBackgroundColor, 1.15)
                                : Kirigami.Theme.alternateBackgroundColor
        opacity: card.moving ? 0.75 : 1
        border.width: card.activeFocus ? 3 : 0
        border.color: Kirigami.Theme.highlightColor

        ColumnLayout {
            anchors.fill: parent
            anchors.margins: Kirigami.Units.largeSpacing
            spacing: Kirigami.Units.smallSpacing

            SiteIcon {
                Layout.alignment: Qt.AlignHCenter
                Layout.preferredWidth: Kirigami.Units.iconSizes.large
                Layout.preferredHeight: Kirigami.Units.iconSizes.large
                url: card.url
                title: card.title
                iconName: card.iconName
            }

            Text {
                Layout.fillWidth: true
                text: card.title
                color: Kirigami.Theme.textColor
                font.pointSize: Kirigami.Theme.defaultFont.pointSize * 1.05
                horizontalAlignment: Text.AlignHCenter
                elide: Text.ElideRight
                maximumLineCount: 2
                wrapMode: Text.Wrap
            }

            Text {
                Layout.fillWidth: true
                visible: card.detail !== ""
                text: card.detail
                color: Kirigami.Theme.disabledTextColor
                font.pointSize: Kirigami.Theme.smallFont.pointSize
                horizontalAlignment: Text.AlignHCenter
                elide: Text.ElideRight
            }

            Item {
                Layout.fillHeight: true
            }
        }
    }
}
