// SPDX-License-Identifier: GPL-3.0-only
//
// The browser window: an address bar over one page. Everything works with the arrows, OK and Back.
// Keys, as Plasma Bigscreen's remote support sends them (a TV remote through CEC, a game pad):
//   Back (remote Back, game pad B / Escape, Alt+Left)  leave full screen video, hide the bar, or go back
//   Menu (game pad X), Home, Ctrl+L, F6                  show the address bar
//   F5, Ctrl+R                                           reload
//   F11                                                  window or full screen

import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import QtWebEngine
import org.kde.kirigami as Kirigami
import Sofa

Kirigami.AbstractApplicationWindow {
    id: root

    required property WebEngineProfile profile
    required property string startUrl
    required property bool startFullScreen

    // A page has been opened (the start screen is gone).
    readonly property bool hasPage: view.url.toString() !== ""
    // The address bar is in front of the page.
    property bool barShown: true

    title: view.title !== "" ? view.title : "Sofa"
    width: Kirigami.Units.gridUnit * 64
    height: Kirigami.Units.gridUnit * 36
    color: Kirigami.Theme.backgroundColor

    function open(text) {
        const url = Core.resolveAddress(text);
        if (url === "") {
            return;
        }
        view.url = url;
        showPage();
    }

    function showBar() {
        barShown = true;
        address.forceActiveFocus();
        address.selectAll();
    }

    function showPage() {
        if (!hasPage) {
            return;
        }
        barShown = false;
        view.forceActiveFocus();
    }

    function back() {
        if (view.isFullScreen) {
            view.fullScreenCancelled();
        } else if (barShown && hasPage) {
            showPage();
        } else if (view.canGoBack) {
            view.goBack();
        } else if (hasPage) {
            showBar();
        }
    }

    function toggleWindowFullScreen() {
        if (root.visibility === Window.FullScreen) {
            root.showNormal();
        } else {
            root.showFullScreen();
        }
    }

    Component.onCompleted: {
        if (startFullScreen) {
            root.showFullScreen();
        } else {
            root.show();
        }
        if (startUrl !== "") {
            view.url = startUrl;
            showPage();
        } else {
            showBar();
        }
    }

    Shortcut {
        sequences: ["Back", "Esc", "Alt+Left", StandardKey.Back]
        onActivated: root.back()
    }
    Shortcut {
        sequences: ["Menu", "HomePage", "Ctrl+L", "F6"]
        onActivated: root.showBar()
    }
    Shortcut {
        sequences: [StandardKey.Refresh, "Ctrl+R"]
        enabled: root.hasPage
        onActivated: view.reload()
    }
    Shortcut {
        sequence: "F11"
        onActivated: root.toggleWindowFullScreen()
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        QQC2.ToolBar {
            id: bar
            Layout.fillWidth: true
            visible: root.barShown && !view.isFullScreen

            contentItem: RowLayout {
                spacing: Kirigami.Units.largeSpacing

                Kirigami.Icon {
                    source: "go-home"
                    implicitWidth: Kirigami.Units.iconSizes.medium
                    implicitHeight: implicitWidth
                    visible: !root.hasPage
                }

                QQC2.TextField {
                    id: address
                    Layout.fillWidth: true
                    font.pointSize: Kirigami.Theme.defaultFont.pointSize * 1.4
                    placeholderText: qsTr("Type a web address or search")
                    inputMethodHints: Qt.ImhUrlCharactersOnly | Qt.ImhNoPredictiveText | Qt.ImhNoAutoUppercase
                    selectByMouse: true
                    Accessible.name: qsTr("Address")
                    // Shows the short form while it isn't being edited.
                    text: activeFocus ? view.url.toString() : Core.displayAddress(view.url.toString())
                    onAccepted: root.open(text)
                    Keys.onDownPressed: root.showPage()
                }
            }
        }

        // How far the page has loaded.
        QQC2.ProgressBar {
            Layout.fillWidth: true
            Layout.preferredHeight: Kirigami.Units.smallSpacing
            visible: view.loading && !view.isFullScreen
            from: 0
            to: 100
            value: view.loadProgress
        }

        Item {
            Layout.fillWidth: true
            Layout.fillHeight: true

            WebEngineView {
                id: view
                anchors.fill: parent
                visible: root.hasPage
                profile: root.profile
                activeFocusOnPress: true

                // Private by default: pages get no camera, microphone, location, screen or notifications.
                onPermissionRequested: permission => permission.deny()

                // A new window only for a link the user chose, and in this view; pop-ups pages open on their own
                // are dropped. (The full pop-up rules come with ad blocking.)
                onNewWindowRequested: request => {
                    if (request.userInitiated) {
                        view.url = request.requestedUrl;
                    }
                }

                // Videos and games may go full screen; Back brings the page back.
                onFullScreenRequested: request => {
                    request.accept();
                    if (request.toggleOn) {
                        root.showFullScreen();
                    } else if (!root.startFullScreen) {
                        root.showNormal();
                    }
                }
            }

            // The start screen until a page is opened.
            ColumnLayout {
                anchors.centerIn: parent
                visible: !root.hasPage
                spacing: Kirigami.Units.largeSpacing * 2

                Image {
                    Layout.alignment: Qt.AlignHCenter
                    source: "qrc:/sofa/icon.svg"
                    sourceSize.width: Kirigami.Units.gridUnit * 8
                    sourceSize.height: Kirigami.Units.gridUnit * 8
                    Accessible.ignored: true
                }
                Kirigami.Heading {
                    Layout.alignment: Qt.AlignHCenter
                    text: "Sofa"
                    level: 1
                }
                QQC2.Label {
                    Layout.alignment: Qt.AlignHCenter
                    text: qsTr("Type a web address or something to search for, then press OK.")
                    color: Kirigami.Theme.disabledTextColor
                }
            }
        }
    }
}
