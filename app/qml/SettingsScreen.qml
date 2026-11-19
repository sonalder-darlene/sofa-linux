// SPDX-License-Identifier: GPL-3.0-only
//
// Settings: the search engine, what Sofa opens at start, and what this copy is. More settings come with the
// features they belong to.

import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import Sofa

FocusScope {
    id: screen

    // The page in sight, offered as "the open page" for Open at start (never a private one); "" for none.
    property string openPage: ""
    property var rows: []

    signal chooseEngine()
    signal back()

    function start(focusKey) {
        refresh();
        const i = Math.max(0, rows.findIndex(r => r.key === focusKey));
        list.currentIndex = i;
        list.forceActiveFocus();
    }

    function refresh() {
        const engines = JSON.parse(Session.engines());
        const engine = engines.find(e => e.id === Session.value("search_engine"));
        const open = Session.value("open_at_start");
        const page = Session.value("start_page");
        // A new list puts the focus back on its first row: keep it where it was.
        const keep = Math.max(0, list.currentIndex);
        Qt.callLater(() => list.currentIndex = keep);
        rows = [
            { key: "engine", title: qsTr("Search engine"), value: engine ? engine.name : qsTr("Not chosen yet (DuckDuckGo for now)"), icon: "search" },
            { key: "open", title: qsTr("Open at start"), icon: "go-home",
              value: open === "page" ? (page !== "" ? Session.label(page) : qsTr("A web page"))
                   : open === "last" ? qsTr("Where I left off") : qsTr("Start page") },
            { key: "about", title: qsTr("About Sofa"), value: qsTr("Version %1").arg(Core.version), icon: "help-about" }
        ];
    }

    function activate(row) {
        if (row.key === "engine") {
            screen.chooseEngine();
        } else if (row.key === "open") {
            askOpenAtStart();
        } else if (row.key === "about") {
            choice.tell(qsTr("Sofa %1").arg(Core.version),
                        qsTr("A TV web browser for KDE Plasma Bigscreen. Free software under the GNU GPL version 3. Sofa has no servers and no accounts of its own, and sends nothing about you anywhere. Bookmarks, history and settings stay on this computer."));
        }
    }

    function askOpenAtStart() {
        const current = Session.value("open_at_start");
        const options = [
            { text: qsTr("Start page"), value: "home" },
            { text: qsTr("A web page"), detail: qsTr("The page you choose"), value: "page" },
            { text: qsTr("Where I left off"), detail: qsTr("The tab that was in sight, if a page was. Never a private one."), value: "last" }
        ];
        choice.ask(qsTr("Open at start"), qsTr("What Sofa shows when it starts. Home in the menu always opens the start page."),
                   options, value => {
            if (value === "page") {
                askStartPage();
            } else {
                Session.setValue("open_at_start", value);
                refresh();
            }
        }, Math.max(0, options.findIndex(o => o.value === current)));
    }

    function askStartPage() {
        const page = Session.value("start_page");
        const offer = screen.openPage !== "" && screen.openPage !== page;
        text.ask({
            title: qsTr("A web page"),
            message: qsTr("The page Sofa opens at start."),
            text: page !== "" ? page : "https://",
            address: true,
            extraText: offer ? qsTr("Use the open page") : "",
            extraValue: screen.openPage
        }, typed => {
            const address = Session.webAddress(typed);
            if (address === "") {
                return qsTr("Type a web address, such as example.com.");
            }
            Session.setValue("start_page", address);
            Session.setValue("open_at_start", "page");
            refresh();
            return "";
        });
    }

    Keys.onBackPressed: screen.back()
    Keys.onEscapePressed: screen.back()

    Rectangle {
        anchors.fill: parent
        color: Kirigami.Theme.backgroundColor
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: Kirigami.Units.gridUnit * 2
        anchors.leftMargin: Kirigami.Units.gridUnit * 3
        anchors.rightMargin: Kirigami.Units.gridUnit * 3
        spacing: Kirigami.Units.gridUnit

        Kirigami.Heading {
            level: 1
            text: qsTr("Settings")
            Accessible.role: Accessible.Heading
        }

        ListView {
            id: list
            Layout.fillWidth: true
            Layout.fillHeight: true
            model: screen.rows
            keyNavigationEnabled: true
            spacing: Kirigami.Units.smallSpacing
            Accessible.role: Accessible.List
            Accessible.name: qsTr("Settings")

            delegate: ListRow {
                required property var modelData
                width: ListView.view.width
                focus: ListView.isCurrentItem
                iconName: modelData.icon
                title: modelData.title
                detail: modelData.value
                onActivated: screen.activate(modelData)
            }
        }
    }

    ChoiceDialog {
        id: choice
        onClosed: if (screen.visible && !text.visible) list.forceActiveFocus()
    }

    TextDialog {
        id: text
        onClosed: if (screen.visible && !choice.visible) list.forceActiveFocus()
    }
}
