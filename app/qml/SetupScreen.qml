// SPDX-License-Identifier: GPL-3.0-only
//
// First start: the user picks a search engine; Sofa doesn't pick one for them. Back leaves without choosing (Sofa
// then searches with DuckDuckGo until one is chosen), and the question comes back at the next start. The same list
// opens from Settings.

import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import Sofa

FocusScope {
    id: screen

    property var engines: []

    signal done()

    // Where each engine is from and where its results come from (as of October 2026, as on Android).
    readonly property var about: ({
        brave: qsTr("USA · own independent index"),
        duckduckgo: qsTr("USA · mostly Bing results, plus its own crawler"),
        ecosia: qsTr("Germany · Bing, Google and EUSP results"),
        google: qsTr("USA · own index"),
        kagi: qsTr("USA · own index plus other sources"),
        metager: qsTr("Germany · non-profit meta-search"),
        mojeek: qsTr("UK · own independent index"),
        murena: qsTr("France · Qwant results, from Murena"),
        qwant: qsTr("France · own index plus Bing results"),
        startpage: qsTr("Netherlands · Google and Bing results, proxied"),
        swisscows: qsTr("Switzerland · family-safe; own index + Bing"),
        uruky: qsTr("Portugal · meta-search of European indexes")
    })

    function start() {
        engines = JSON.parse(Session.engines());
        const chosen = Session.value("search_engine");
        const i = engines.findIndex(e => e.id === chosen);
        list.currentIndex = Math.max(0, i);
        list.positionViewAtIndex(list.currentIndex, ListView.Center);
        list.forceActiveFocus();
    }

    function choose(engine) {
        if (engine.paid) {
            choice.ask(qsTr("Use %1?").arg(engine.name),
                       qsTr("%1 only works with a paid account or key. You'll be asked to sign in on its site the first time you search.").arg(engine.name),
                       [{ text: qsTr("Use %1").arg(engine.name), value: true }, { text: qsTr("Choose another"), value: false }],
                       yes => {
                           if (yes) {
                               Session.setValue("search_engine", engine.id);
                               screen.done();
                           }
                       }, 0);
            return;
        }
        Session.setValue("search_engine", engine.id);
        screen.done();
    }

    Keys.onBackPressed: screen.done()
    Keys.onEscapePressed: screen.done()

    Rectangle {
        anchors.fill: parent
        color: Kirigami.Theme.backgroundColor
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: Kirigami.Units.gridUnit * 3
        spacing: Kirigami.Units.gridUnit

        Image {
            Layout.alignment: Qt.AlignHCenter
            source: "qrc:/sofa/icon.svg"
            sourceSize.width: Kirigami.Units.gridUnit * 4
            sourceSize.height: Kirigami.Units.gridUnit * 4
            Accessible.ignored: true
        }
        Kirigami.Heading {
            Layout.alignment: Qt.AlignHCenter
            text: qsTr("Choose your search engine")
            level: 1
        }
        QQC2.Label {
            Layout.alignment: Qt.AlignHCenter
            text: qsTr("Sofa doesn't pick one for you. You can change it any time in Settings.")
            color: Kirigami.Theme.disabledTextColor
        }

        ListView {
            id: list
            Layout.alignment: Qt.AlignHCenter
            Layout.preferredWidth: Math.min(parent.width, Kirigami.Units.gridUnit * 36)
            Layout.fillHeight: true
            model: screen.engines
            keyNavigationEnabled: true
            clip: true
            spacing: Kirigami.Units.smallSpacing
            highlightRangeMode: ListView.ApplyRange
            preferredHighlightBegin: Kirigami.Units.gridUnit * 3
            preferredHighlightEnd: height - Kirigami.Units.gridUnit * 3
            Accessible.role: Accessible.List
            Accessible.name: qsTr("Search engines")

            delegate: ListRow {
                required property var modelData
                width: ListView.view.width
                focus: ListView.isCurrentItem
                iconName: "search"
                title: modelData.name + (modelData.paid ? "  · " + qsTr("paid") : "")
                detail: screen.about[modelData.id] || ""
                onActivated: screen.choose(modelData)
            }
        }
    }

    ChoiceDialog {
        id: choice
        onClosed: if (screen.visible) list.forceActiveFocus()
    }
}
