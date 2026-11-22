// SPDX-License-Identifier: GPL-3.0-only
//
// A card on the start page, made of KDE's media card: a site (its icon on a tile coloured from its name), a folder,
// or an action such as "Add site". OK opens it; OK held or Menu asks for its actions (`held`) when it has some.

import QtQuick
import org.kde.kirigami as Kirigami
import org.kde.bigscreen.ui as BigscreenUI

BigscreenUI.MediaCard {
    id: card

    property string title: ""
    property string url: ""
    property string detail: ""
    // A theme icon instead of a site (folders, "Add site", "All bookmarks").
    property string iconName: ""
    property bool holdable: false
    // Being carried with Left and Right (Quick access "Move").
    property bool moving: false

    signal held()

    artworkWidth: Kirigami.Units.gridUnit * 12
    artworkHeight: Math.round(artworkWidth * 9 / 16)
    text: title
    subtitle: detail
    opacity: moving ? 0.75 : 1

    // A long press of OK on a card without actions opens it, as a short one does.
    onPressAndHold: holdable ? held() : clicked()
    Keys.onMenuPressed: event => {
        event.accepted = holdable;
        if (holdable && !event.isAutoRepeat) {
            held();
        }
    }

    // A site's tile: a colour picked from its name, light at the top, with the site's icon in the middle.
    Rectangle {
        id: tile
        parent: card.artworkItem
        anchors.fill: parent
        radius: BigscreenUI.Units.cardCornerRadius
        readonly property color tone: card.iconName !== "" ? Kirigami.Theme.alternateBackgroundColor
                                                           : Qt.hsva(Math.abs(hash(card.title + card.url)) % 360 / 360, 0.45, 0.6, 1)
        gradient: Gradient {
            GradientStop { position: 0; color: Qt.lighter(tile.tone, 1.2) }
            GradientStop { position: 1; color: Qt.darker(tile.tone, 1.3) }
        }
        border.width: card.activeFocus ? BigscreenUI.Units.focusBorderWidth : 0
        border.color: Kirigami.Theme.highlightColor

        function hash(s) {
            let h = 0;
            for (let i = 0; i < s.length; i++) {
                h = (h * 31 + s.charCodeAt(i)) | 0;
            }
            return h;
        }

        Rectangle {
            anchors.centerIn: parent
            width: Math.round(parent.height * 0.5)
            height: width
            radius: width * 0.2
            visible: card.iconName === ""
            color: Qt.rgba(1, 1, 1, 0.95)

            SiteIcon {
                anchors.centerIn: parent
                width: Math.round(parent.width * 0.65)
                height: width
                url: card.url
                title: card.title
            }
        }

        Kirigami.Icon {
            anchors.centerIn: parent
            width: Math.round(parent.height * 0.45)
            height: width
            visible: card.iconName !== ""
            source: card.iconName
        }
    }
}
