// SPDX-License-Identifier: GPL-3.0-only
//
// Settings, laid out as KDE's TV settings pages: the search engine, what Sofa opens at start, ad blocking, how long
// history is kept, and what this copy is. More settings come with the features they belong to.

import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import org.kde.bigscreen.ui as BigscreenUI
import Sofa

BigscreenUI.ScrollablePage {
    id: screen

    // The page in sight, offered as "the open page" for Open at start (never a private one); "" for none.
    property string openPage: ""
    // The row whose question is open, focused again when it closes.
    property Item lastRow: engineRow
    // Bumped to read the settings again.
    property int revision: 0

    signal chooseEngine()
    signal back()

    title: qsTr("Settings")
    background: BigscreenUI.AmbientBackground {}

    BigscreenUI.BackHandler.enabled: visible
    BigscreenUI.BackHandler.onActivated: screen.back()

    // Shows the screen with the row `focusKey` ("engine", "open", "adblock", "history", "about"; "" for the first) in
    // focus.
    function start(focusKey) {
        revision++;
        const rows = { engine: engineRow, open: openRow, adblock: adBlockRow, history: historyRow, about: aboutRow };
        (rows[focusKey] || engineRow).forceActiveFocus();
        scrollToTop();
    }

    readonly property string engineName: {
        revision;
        const engine = JSON.parse(Session.engines()).find(e => e.id === Session.value("search_engine"));
        return engine ? engine.name : qsTr("Not chosen yet (DuckDuckGo for now)");
    }
    readonly property string openAtStart: {
        revision;
        const open = Session.value("open_at_start");
        const page = Session.value("start_page");
        return open === "page" ? (page !== "" ? Session.label(page) : qsTr("A web page"))
             : open === "last" ? qsTr("Where I left off") : qsTr("Start page");
    }
    readonly property bool adBlock: (revision, Session.value("adblock") === "true")

    // Settings → Keep history for: the stored value (days, -1 always, 0 never) and its name.
    readonly property var historyChoices: [
        { text: qsTr("7 days"), value: "7" },
        { text: qsTr("30 days"), value: "30" },
        { text: qsTr("90 days"), value: "90" },
        { text: qsTr("Always"), value: "-1" },
        { text: qsTr("Never"), detail: qsTr("Nothing is saved"), value: "0" }
    ]
    readonly property string historyDays: (revision, Session.value("history_days"))

    function askHistory() {
        choice.ask(qsTr("Keep history for"),
                   qsTr("Pages you visit stay in History, on this computer, for this long. Older ones are removed. Never: nothing is saved."),
                   historyChoices, value => {
            Session.setValue("history_days", value);
            Library.keepHistoryFor(Number(value));
            revision++;
        }, Math.max(0, historyChoices.findIndex(c => c.value === historyDays)));
    }

    function about() {
        const engine = JSON.parse(Session.engineInfo());
        choice.tell(qsTr("Sofa %1").arg(Core.version),
                    qsTr("A TV web browser for KDE Plasma Bigscreen. Free software under the GNU GPL version 3. Sofa has no servers and no accounts of its own, and sends nothing about you anywhere. Bookmarks, history and settings stay on this computer.")
                    + "\n\n"
                    + qsTr("Web engine: Qt WebEngine on Qt %1, built from Chromium %2, with Chromium's security fixes up to version %3.")
                          .arg(engine.qt).arg(engine.chromium).arg(engine.fixes));
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
                revision++;
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
            revision++;
            return "";
        });
    }

    ColumnLayout {
        spacing: Kirigami.Units.smallSpacing
        BigscreenUI.KeyNavigationGroup.orientation: Qt.Vertical

        BigscreenUI.ButtonDelegate {
            id: engineRow
            Layout.fillWidth: true
            icon.name: "search"
            text: qsTr("Search engine")
            description: screen.engineName
            onClicked: screen.chooseEngine()
        }
        BigscreenUI.ButtonDelegate {
            id: openRow
            Layout.fillWidth: true
            icon.name: "go-home"
            text: qsTr("Open at start")
            description: screen.openAtStart
            onClicked: {
                screen.lastRow = openRow;
                screen.askOpenAtStart();
            }
        }
        BigscreenUI.SwitchDelegate {
            id: adBlockRow
            Layout.fillWidth: true
            icon.name: screen.adBlock ? "security-high" : "security-low"
            text: qsTr("Block ads and trackers")
            checked: screen.adBlock
            onToggled: {
                Session.setValue("adblock", checked ? "true" : "false");
                screen.revision++;
            }
        }
        BigscreenUI.ButtonDelegate {
            id: historyRow
            Layout.fillWidth: true
            icon.name: "view-history"
            text: qsTr("Keep history for")
            description: (screen.historyChoices.find(c => c.value === screen.historyDays) || screen.historyChoices[2]).text
            onClicked: {
                screen.lastRow = historyRow;
                screen.askHistory();
            }
        }
        BigscreenUI.ButtonDelegate {
            id: aboutRow
            Layout.fillWidth: true
            icon.name: "help-about"
            text: qsTr("About Sofa")
            description: qsTr("Version %1").arg(Core.version)
            onClicked: {
                screen.lastRow = aboutRow;
                screen.about();
            }
        }
    }

    ChoiceDialog {
        id: choice
        onClosed: if (screen.visible && !text.visible) screen.lastRow.forceActiveFocus()
    }

    TextDialog {
        id: text
        onClosed: if (screen.visible && !choice.visible) screen.lastRow.forceActiveFocus()
    }
}
