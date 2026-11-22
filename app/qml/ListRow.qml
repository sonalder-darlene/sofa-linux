// SPDX-License-Identifier: GPL-3.0-only
//
// A row in a list (Bookmarks, History, suggestions, choices), made of KDE's TV list row: an icon, a title and a line
// under it. OK opens it (`activated`); OK held or Menu asks for its actions (`held`) when it has some.

import QtQuick
import org.kde.kirigami as Kirigami
import org.kde.bigscreen.ui as BigscreenUI

BigscreenUI.ButtonDelegate {
    id: row

    property string title: ""
    property string detail: ""
    property string url: ""
    property string iconName: ""
    // A slimmer row (the Up row in a folder), so it reads as a way out rather than an item.
    property bool slim: false
    property string trailingText: ""
    property bool holdable: false
    // An icon at the start: a site's (from `url` and `title`) or a theme icon. Off for plain choices.
    property bool showIcon: true

    signal activated()
    signal held()

    text: title
    description: detail
    verticalPadding: slim ? Kirigami.Units.largeSpacing : BigscreenUI.Units.verticalPadding
    Accessible.name: detail !== "" ? title + ", " + detail : title
    Accessible.role: Accessible.ListItem

    // An address is one line, its middle left out when it doesn't fit.
    descriptionItem.wrapMode: Text.NoWrap
    descriptionItem.elide: Text.ElideMiddle
    descriptionItem.maximumLineCount: 1

    leadingPadding: showIcon ? Kirigami.Units.gridUnit : 0
    trailingPadding: trailingText !== "" ? Kirigami.Units.gridUnit : 0
    leading: SiteIcon {
        visible: row.showIcon
        implicitWidth: !row.showIcon ? 0 : row.slim ? Kirigami.Units.iconSizes.smallMedium : Kirigami.Units.iconSizes.medium
        implicitHeight: implicitWidth
        url: row.url
        title: row.title
        iconName: row.iconName
    }
    trailing: BigscreenUI.Label {
        text: row.trailingText
        color: Kirigami.Theme.disabledTextColor
    }

    onClicked: activated()
    // A long press of OK on a row without actions opens it, as a short one does.
    onPressAndHold: holdable ? held() : activated()
    Keys.onMenuPressed: event => {
        event.accepted = holdable;
        if (holdable && !event.isAutoRepeat) {
            held();
        }
    }
}
