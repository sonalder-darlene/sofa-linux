// SPDX-License-Identifier: GPL-3.0-only
//
// A site's icon: the one the engine remembers for the page (only for pages opened in Sofa, nothing is fetched for
// it), else the site's first letter on a colour picked from its name, else a theme icon.

import QtQuick
import org.kde.kirigami as Kirigami

Item {
    id: icon

    property string url: ""
    property string title: ""
    // A theme icon name instead of a site (folders, "Add site", "All bookmarks").
    property string iconName: ""

    readonly property string letter: {
        const t = title.trim();
        return t.length > 0 ? t.charAt(0).toUpperCase() : "?";
    }

    // The engine answers with an empty picture for a page it has no icon for.
    readonly property bool hasFavicon: favicon.status === Image.Ready && favicon.implicitWidth > 1

    implicitWidth: Kirigami.Units.iconSizes.large
    implicitHeight: implicitWidth

    Kirigami.Icon {
        anchors.fill: parent
        visible: icon.iconName !== ""
        source: icon.iconName
    }

    Rectangle {
        anchors.fill: parent
        visible: icon.iconName === "" && !icon.hasFavicon
        radius: width / 2
        color: Qt.hsla(Math.abs(hash(icon.title + icon.url)) % 360 / 360, 0.45, 0.42, 1)

        function hash(s) {
            let h = 0;
            for (let i = 0; i < s.length; i++) {
                h = (h * 31 + s.charCodeAt(i)) | 0;
            }
            return h;
        }

        Text {
            anchors.centerIn: parent
            text: icon.letter
            color: "white"
            font.pixelSize: parent.height * 0.5
            font.bold: true
        }
    }

    Image {
        id: favicon
        anchors.fill: parent
        visible: icon.iconName === "" && icon.hasFavicon
        source: icon.iconName === "" && icon.url !== "" ? "image://favicon/" + icon.url : ""
        sourceSize.width: width
        sourceSize.height: height
        asynchronous: true
        cache: true
        fillMode: Image.PreserveAspectFit
    }
}
