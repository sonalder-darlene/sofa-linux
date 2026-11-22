// SPDX-License-Identifier: GPL-3.0-only
//
// The browser window. It holds the tabs (each a page of its own, loaded only while it is in sight or seen lately),
// the start page, Sofa's screens (search, bookmarks, history, settings) and the menu over a page.
//
// Keys, as Plasma Bigscreen's remote support sends them (a TV remote through CEC, a game pad):
//   on a page: arrows and OK drive the pointer; Back or Menu opens Sofa's menu; Home opens the start page;
//              Page Up / Page Down (Channel up / down) scroll a screen
//   on Sofa's screens: arrows move the focus, OK picks, OK held or Menu lists actions, Back goes back
//   F11 switches between a window and full screen.
//
// A page out of sight (another tab, or the start page over it) is frozen: its scripts and media stop, so it can
// neither be heard nor open anything behind Sofa's screens. Tabs out of sight past the few kept loaded (by the
// computer's memory) and any out of sight for ten minutes are unloaded; their address and back list stay.

import QtQuick
import QtQuick.Templates as T
import QtWebEngine
import org.kde.kirigami as Kirigami
import org.kde.bigscreen.ui as BigscreenUI
import Sofa

T.ApplicationWindow {
    id: root

    // KDE's TV look: the window's colours, and Bigscreen's colour scheme outside Plasma (Plasma's own inside it).
    Kirigami.Theme.colorSet: Kirigami.Theme.Window
    Kirigami.Theme.inherit: false
    readonly property QtObject colorSchemes: BigscreenUI.ColorSchemeManager

    required property WebEngineProfile profile
    required property QtObject host
    required property string startUrl
    required property bool startFullScreen
    // Whether the window was a window (not full screen) when a page last asked for full screen.
    property bool windowedBeforePage: false

    // "start", "page", "search", "bookmarks", "history", "settings" or "setup".
    property string screen: "start"
    // Where Back goes from a screen: the screens it was opened from.
    property var screenTrail: []
    property bool menuShown: false
    property bool pointerOn: true

    // Tabs: one side for normal browsing, one for private browsing (as on Android, never side by side).
    property bool privateMode: false
    property QtObject privateProfile: null
    // id → {id, url, title, view (a WebEngineView or null while unloaded), private}
    property var tabs: ({})
    property int nextTabId: 1
    property int currentId: -1
    // Bumped whenever a tab changes, so bindings that read `tabs` follow.
    property int revision: 0
    readonly property TabOrder order: privateMode ? privateOrder : normalOrder
    readonly property var currentTab: (revision, tabs[currentId] || null)
    // Each reads `revision`: `currentTab` stays the same object while its fields change.
    readonly property string currentUrl: (revision, currentTab ? currentTab.url : "")
    readonly property var currentView: (revision, currentTab ? currentTab.view : null)
    // How many pages out of sight stay loaded on this computer.
    readonly property int liveKeep: Session.liveBackground()
    property double lastBackOnStart: 0

    readonly property bool dialogOpen: linkMenu.visible || tabMenu.visible || tileDialog.visible || tileMenu.visible

    title: (revision, currentTab && currentTab.title !== "" && screen === "page" ? currentTab.title + " — Sofa" : "Sofa")
    width: Kirigami.Units.gridUnit * 64
    height: Kirigami.Units.gridUnit * 36
    color: Kirigami.Theme.backgroundColor

    TabOrder {
        id: normalOrder
    }
    TabOrder {
        id: privateOrder
    }

    // The remote's keys go to the pointer while a page is in front with nothing over it.
    Binding {
        target: root.host
        property: "remoteKeys"
        value: root.screen === "page" && !root.menuShown && !root.dialogOpen && root.pointerOn
               && root.currentView !== null && !root.currentView.isFullScreen
    }

    // No second pointer on screen: the system's hides while the remote drives Sofa's over a page, until a real
    // pointer is used again.
    Binding {
        target: root.host
        property: "hideCursor"
        value: pointer.driving && pageLayer.visible && root.pointerOn
    }

    Connections {
        target: root.host
        // A press that may reach the page in front: its tab's pop-up rules count it.
        function onPressed() {
            if (root.screen === "page" && root.currentView) {
                root.currentView.guard.press();
                // Into a text field or out of one: a keyboard's Return then goes to the page, or not.
                pointer.checkTyping();
            }
        }
        // Back from Bigscreen's home (or wherever the window was hidden): the page in sight takes the keys again.
        function onWindowShownChanged() {
            if (root.host.windowShown && root.screen === "page" && !root.menuShown && !root.dialogOpen && root.currentView) {
                root.currentView.forceActiveFocus();
            }
        }
        // Another launch (a link from another app, the launcher) handed over an address, or asked for the window.
        function onOpenRequested(text) {
            root.raise();
            root.requestActivate();
            if (text === "") {
                return;
            }
            const url = text.toLowerCase().startsWith("file:") ? text : Session.resolveAddress(text);
            if (url === "") {
                return;
            }
            // In a tab of its own, unless the tab in sight is empty or no tab fits, as on Android.
            if (root.currentUrl !== "" && !root.order.isFull()) {
                root.openInNewTab(url);
            } else {
                root.openInCurrent(url);
            }
        }
        function onRemoteKey(key, pressed) {
            if (key === Qt.Key_Back || key === Qt.Key_Escape) {
                if (pressed) root.pageBack();
            } else if (key === Qt.Key_Menu) {
                if (pressed) root.openMenu();
            } else if (key === Qt.Key_HomePage) {
                if (pressed) root.goHome();
            } else if (key === Qt.Key_Forward) {
                if (pressed) root.pageForward();
            } else {
                pointer.key(key, pressed);
            }
        }
        // A mouse, touchpad or a pad's stick: Sofa's pointer steps aside until the next arrow.
        function onRealPointer(x, y) {
            pointer.realUsed(x, y);
        }
    }

    // --- Tabs ------------------------------------------------------------------------------------------------------

    // Adds a tab for `url` ("" for an empty one) after `after` (-1: at the end); its id, or -1 when the side is full.
    function addTab(url, after) {
        const id = nextTabId;
        if (!order.add(id, after)) {
            return -1;
        }
        nextTabId++;
        // `quiet`: the tab's first load is Sofa's own (a tab brought back at start), not a visit the user made.
        tabs[id] = { id: id, url: url, title: "", view: null, private: privateMode, quiet: false };
        revision++;
        return id;
    }

    function tabAddresses() {
        return JSON.parse(order.order()).map(id => tabs[id]);
    }

    // The page of `tab`, made the first time it is needed.
    function viewOf(tab) {
        if (!tab.view) {
            tab.view = pageComponent.createObject(pageLayer, {
                tabId: tab.id,
                profile: tab.private ? root.privateProfile : root.profile
            });
            if (tab.url !== "") {
                tab.view.quietLoad = tab.quiet;
                tab.view.url = tab.url;
            }
            tab.quiet = false;
            revision++;
        }
        return tab.view;
    }

    function selectTab(id) {
        if (!tabs[id]) {
            return;
        }
        order.select(id);
        currentId = id;
        revision++;
        trimTabs();
        scheduleSave();
    }

    // Brings the tab in sight onto the screen: its page if it has one, else the start page.
    function showCurrent() {
        if (currentTab && currentTab.url !== "") {
            showPage();
        } else {
            showStart();
        }
    }

    function showPage() {
        if (!currentTab || currentTab.url === "") {
            showStart();
            return;
        }
        const view = viewOf(currentTab);
        screenTrail = [];
        menuShown = false;
        screen = "page";
        view.wake();
        view.forceActiveFocus();
        scheduleSave();
    }

    function showStart() {
        menuShown = false;
        screenTrail = [];
        screen = "start";
        startPage.privateMode = privateMode;
        startPage.refresh();
        startPage.focusFirst();
        scheduleSave();
    }

    // Opens `url` in the tab in sight (the start page's cards, a search, a bookmark).
    function openInCurrent(url) {
        if (url === "") {
            return;
        }
        if (!currentTab) {
            const id = addTab("", -1);
            selectTab(id);
        }
        const tab = currentTab;
        tab.url = url;
        tab.title = "";
        tab.quiet = false;
        const view = viewOf(tab);
        view.url = url;
        revision++;
        showPage();
    }

    function openInNewTab(url) {
        const id = addTab(url, currentId);
        if (id < 0) {
            notify(qsTr("Up to 10 tabs: close one to open another"));
            return;
        }
        selectTab(id);
        showPage();
    }

    function newTab() {
        const id = addTab("", currentId);
        if (id < 0) {
            notify(qsTr("Up to 10 tabs: close one to open another"));
            return;
        }
        selectTab(id);
        showStart();
    }

    function closeTabs(ids) {
        let next = -1;
        for (const id of ids) {
            const tab = tabs[id];
            if (!tab) {
                continue;
            }
            if (tab.view) {
                tab.view.destroy();
            }
            delete tabs[id];
            next = order.remove(id);
        }
        if (next < 0) {
            next = addTab("", -1);
        }
        currentId = -1;
        selectTab(next);
        showCurrent();
    }

    function askCloseTabs(id) {
        const ids = JSON.parse(order.order());
        const options = [{ text: qsTr("Close tab"), value: "this" }];
        if (ids.length > 1) {
            options.push({ text: qsTr("Close other tabs"), value: "others" });
            options.push({ text: qsTr("Close all tabs"), value: "all" });
        }
        const tab = tabs[id];
        tabMenu.ask(tab && tab.title !== "" ? tab.title : qsTr("Tab"), "", options, value => {
            if (value === "this") {
                closeTabs([id]);
            } else if (value === "others") {
                closeTabs(ids.filter(t => t !== id));
            } else {
                closeTabs(ids);
            }
        });
    }

    // Unloads the pages out of sight that the rules say go (past the few most recent, or out of sight ten minutes).
    function trimTabs() {
        const live = JSON.parse(order.order()).filter(id => id !== currentId && tabs[id] && tabs[id].view
                                                      && tabs[id].view.lifecycleState !== WebEngineView.LifecycleState.Discarded);
        for (const id of JSON.parse(order.toUnload(liveKeep, JSON.stringify(live)))) {
            const view = tabs[id].view;
            if (!view.visible) {
                view.lifecycleState = WebEngineView.LifecycleState.Discarded;
            }
        }
        const still = live.filter(id => tabs[id].view.lifecycleState !== WebEngineView.LifecycleState.Discarded);
        const wait = order.nextRest(JSON.stringify(still));
        if (wait >= 0) {
            restTimer.interval = wait + 1000;
            restTimer.restart();
        } else {
            restTimer.stop();
        }
    }

    Timer {
        id: restTimer
        onTriggered: root.trimTabs()
    }

    // --- Private browsing -----------------------------------------------------------------------------------------

    function togglePrivate() {
        if (privateMode) {
            leavePrivate();
        } else {
            enterPrivate();
        }
    }

    function enterPrivate() {
        privateProfile = host.newPrivateProfile();
        // The normal tabs are unloaded while private browsing is on; they come back with their history after.
        for (const id of JSON.parse(normalOrder.order())) {
            const view = tabs[id].view;
            if (view) {
                view.lifecycleState = WebEngineView.LifecycleState.Discarded;
            }
        }
        privateMode = true;
        // Choices made in private browsing are kept in memory only, and start afresh with each session.
        Session.forgetPrivateChoices();
        const id = addTab("", -1);
        currentId = -1;
        selectTab(id);
        showStart();
        notify(qsTr("Private browsing is on"));
    }

    function leavePrivate() {
        for (const id of JSON.parse(privateOrder.order())) {
            const tab = tabs[id];
            if (tab && tab.view) {
                tab.view.destroy();
            }
            delete tabs[id];
            privateOrder.remove(id);
        }
        // The profile goes once its pages are gone: with it go the session's cookies, storage and cache.
        endProfile.profile = privateProfile;
        endProfile.restart();
        privateProfile = null;
        privateMode = false;
        Session.forgetPrivateChoices();
        currentId = -1;
        const back = normalOrder.current();
        selectTab(back >= 0 ? back : addTab("", -1));
        showStart();
        notify(qsTr("Private browsing is off: its cookies and site data are erased"));
    }

    Timer {
        id: endProfile
        property QtObject profile: null
        interval: 500
        onTriggered: {
            root.host.endPrivateProfile(profile);
            profile = null;
        }
    }

    // --- Screens and keys -----------------------------------------------------------------------------------------

    function openScreen(name) {
        if (screen !== name) {
            screenTrail.push(screen);
        }
        menuShown = false;
        screen = name;
        scheduleSave();
    }

    function screenBack() {
        const previous = screenTrail.pop();
        if (previous === "page") {
            showPage();
        } else if (previous === undefined || previous === "start") {
            showStart();
        } else {
            screen = previous;
            focusScreen();
        }
    }

    function focusScreen() {
        if (screen === "bookmarks") bookmarksScreen.load(bookmarksScreen.folderId, -1);
        else if (screen === "history") historyScreen.start();
        else if (screen === "settings") settingsScreen.start("");
        else if (screen === "start") startPage.focusFirst();
    }

    function goHome() {
        showStart();
    }

    function openMenu() {
        if (screen !== "page") {
            return;
        }
        pointer.reset();
        menu.bookmarked = Library.isBookmarked(currentUrl);
        menu.webPage = Session.isWebUrl(currentUrl);
        menu.adBlockOn = Session.value("adblock") === "true" && !Session.adBlockOffHere(currentUrl);
        // Shown only after a block, and then the menu opens on it.
        menu.allowPending = currentView !== null && currentView.guard.pending() !== "";
        if (menu.allowPending) {
            menu.lastFocused = "allowOnce";
        }
        menuShown = true;
        menu.show();
    }

    function closeMenu() {
        menuShown = false;
        if (currentView) {
            currentView.forceActiveFocus();
        }
    }

    // Back on a page: leave a text field (the on-screen keyboard goes with it), then full screen, then scroll mode,
    // else Sofa's menu. Bigscreen's own browser goes the same way.
    function pageBack() {
        if (currentView && (host.typing || host.editing())) {
            host.typing = false;
            currentView.leaveField();
        } else if (currentView && currentView.isFullScreen) {
            currentView.fullScreenCancelled();
        } else if (pointer.scrollMode) {
            pointer.setScrollMode(false);
        } else {
            openMenu();
        }
    }

    // Back and Forward a page, for a keyboard (Alt+Left, Alt+Right) and a remote's or pad's Forward.
    function pageHistory(forward) {
        const view = screen === "page" ? currentView : null;
        if (view && (forward ? view.canGoForward : view.canGoBack)) {
            forward ? view.goForward() : view.goBack();
        }
    }

    function pageForward() {
        pageHistory(true);
    }

    // The next tab (`steps` 1) or the one before (-1) in the strip, going round at the ends.
    function stepTab(steps) {
        const id = order.neighbour(steps);
        if (id >= 0 && id !== currentId) {
            selectTab(id);
            showCurrent();
        }
    }

    // Zoom from the keyboard, as the menu's buttons do, with a notice since nothing else shows it.
    function keyZoom(action) {
        if (!currentView) {
            return;
        }
        menuAction(action);
        notify(qsTr("Zoom %1 %").arg(Math.round(currentView.zoomFactor * 100)));
    }

    // Back on the start page: the page behind it; else close an empty tab next to others (a New tab nobody used);
    // else press Back again within a moment to leave Sofa.
    function backOnStart() {
        if (currentTab && currentTab.url !== "") {
            showPage();
        } else if (order.count() > 1) {
            closeTabs([currentId]);
        } else {
            const now = Date.now();
            if (now - lastBackOnStart < 2500) {
                quit();
            } else {
                notify(qsTr("Press Back again to leave Sofa"));
            }
            lastBackOnStart = now;
        }
    }

    function menuAction(name) {
        const view = currentView;
        switch (name) {
        case "home": goHome(); break;
        case "back": if (view && view.canGoBack) { view.goBack(); closeMenu(); } break;
        case "forward": if (view && view.canGoForward) { view.goForward(); closeMenu(); } break;
        case "reload": if (view) { view.reload(); closeMenu(); } break;
        case "address":
            openScreen("search");
            searchScreen.start(currentUrl);
            break;
        case "newTab": newTab(); break;
        case "bookmark":
            if (Session.isWebUrl(currentUrl)) {
                menu.bookmarked = Library.toggleBookmark(currentTab.title, currentUrl);
                notify(menu.bookmarked ? qsTr("Bookmarked") : qsTr("Bookmark removed"));
            }
            break;
        case "pin":
            if (Session.isWebUrl(currentUrl)) {
                Library.addTile(currentTab.title, currentUrl);
                notify(qsTr("Added to the start page"));
            }
            break;
        case "bookmarks":
            openScreen("bookmarks");
            bookmarksScreen.start(0);
            break;
        case "history":
            openScreen("history");
            historyScreen.start();
            break;
        case "adblock": toggleAdBlockHere(); break;
        case "allowOnce": allowOnce(); break;
        case "pointer":
            pointerOn = !pointerOn;
            notify(pointerOn ? qsTr("Pointer on: the arrows move it") : qsTr("Pointer off: the arrows go to the page"));
            break;
        case "zoomOut": if (view) view.zoomFactor = Math.max(0.25, Math.round((view.zoomFactor - 0.1) * 10) / 10); break;
        case "zoomIn": if (view) view.zoomFactor = Math.min(5, Math.round((view.zoomFactor + 0.1) * 10) / 10); break;
        case "zoomReset": if (view) view.zoomFactor = 1; break;
        case "settings": openSettings(); break;
        }
    }

    // Ad blocking off or back on for the site in sight, then the page loads again under the new rule, past the cache:
    // a plain reload would show the ads it already has from memory. In private browsing the switch lasts until
    // private browsing ends.
    function toggleAdBlockHere() {
        if (!Session.isWebUrl(currentUrl)) {
            return;
        }
        if (Session.value("adblock") !== "true") {
            notify(qsTr("Ad blocking is turned off in Settings"));
            return;
        }
        const off = !Session.adBlockOffHere(currentUrl);
        if (!Session.setAdBlockOffHere(currentUrl, off, privateMode)) {
            return;
        }
        notify(!off ? qsTr("Ad blocking turned on for this site")
               : privateMode ? qsTr("Ad blocking turned off for this site until you leave private browsing")
               : qsTr("Ad blocking turned off for this site"));
        closeMenu();
        if (currentView) {
            currentView.reloadAndBypassCache();
        }
    }

    // Opens the pop-up or redirect blocked last in the tab in sight, once.
    function allowOnce() {
        const target = currentView ? currentView.guard.takePending() : "";
        closeMenu();
        if (target !== "") {
            currentView.guard.userNavigation();
            openInCurrent(target);
        }
    }

    function openSettings() {
        settingsScreen.openPage = !privateMode && Session.isWebUrl(currentUrl) ? currentUrl : "";
        openScreen("settings");
        settingsScreen.start("");
    }

    function notify(text) {
        toast.show(text, "");
    }

    function quit() {
        saveNow();
        Qt.quit();
    }

    // --- Saving the session ---------------------------------------------------------------------------------------

    function scheduleSave() {
        saveTimer.restart();
    }

    function saveNow() {
        saveTimer.stop();
        const ids = JSON.parse(normalOrder.order());
        const current = normalOrder.current();
        Session.saveTabs(JSON.stringify(ids.map(id => tabs[id] ? tabs[id].url : "")), Math.max(0, ids.indexOf(current)));
        const inSight = (screen === "page") && !privateMode ? currentUrl : "";
        Session.rememberLeftOff(inSight, privateMode);
    }

    Timer {
        id: saveTimer
        interval: 1000
        onTriggered: root.saveNow()
    }

    onClosing: saveNow()

    // --- Start ----------------------------------------------------------------------------------------------------

    // What to show once the first-start question is answered.
    property var afterSetup: null

    Component.onCompleted: {
        if (startFullScreen) {
            root.showFullScreen();
        } else {
            root.show();
        }
        const saved = JSON.parse(Session.savedTabs());
        let inSight = -1;
        saved.addresses.forEach((url, i) => {
            const id = addTab(url, -1);
            tabs[id].quiet = true;
            if (i === saved.current) {
                inSight = id;
            }
        });
        if (inSight < 0) {
            inSight = addTab("", -1);
        }
        selectTab(inSight);

        let show = showStart;
        if (startUrl !== "") {
            // An address from the command line (another app) opens in a tab of its own, unless the tab in sight is
            // empty or no tab fits.
            if (currentUrl !== "" && !order.isFull()) {
                selectTab(addTab("", currentId));
            }
            currentTab.url = startUrl;
            currentTab.quiet = false;
            show = showPage;
        } else {
            const address = Session.startAddress();
            const choice = Session.value("open_at_start");
            if (choice === "last" && address !== "" && currentUrl !== "") {
                show = showPage;
            } else if (choice === "page" && address !== "") {
                // A tab already on that site is reused, so restarts don't pile up tabs.
                const same = JSON.parse(order.order()).find(id => Session.sameSite(tabs[id].url, address));
                if (same !== undefined) {
                    selectTab(same);
                } else {
                    if (currentUrl !== "" && !order.isFull()) {
                        selectTab(addTab("", currentId));
                    }
                    currentTab.url = address;
                    currentTab.quiet = true;
                }
                show = showPage;
            }
        }
        if (Session.value("search_engine") === "") {
            afterSetup = show;
            screen = "setup";
            setupScreen.start();
        } else {
            show();
            // As on Android: not on the first start, which already asks a question.
            Qt.callLater(tellIfEngineOld);
        }
    }

    // Sofa can't update the web engine, only say once in a while that it is old (Session.takeEngineNotice).
    function tellIfEngineOld() {
        if (!Session.takeEngineNotice()) {
            return;
        }
        const flatpak = Session.isFlatpak();
        engineNotice.ask(qsTr("Sofa's web engine is out of date"),
            (flatpak
                ? qsTr("Sofa shows pages with Qt WebEngine, which comes with its Flatpak runtime. This one is quite old. Sofa can't update it, but updating your apps can.")
                : qsTr("Sofa shows pages with Qt WebEngine, which comes with your system. This one is quite old. Sofa can't update it, but your system's updates can."))
            + " " + qsTr("Older versions may be missing security fixes, and some sites may not work properly. You can keep browsing in the meantime."),
            [{ text: qsTr("Keep browsing"), value: "keep" }, { text: qsTr("How to update"), value: "how" }],
            value => {
                if (value === "how") {
                    Qt.callLater(() => engineNotice.tell(qsTr("How to update"), flatpak
                        ? qsTr("Open Discover and install the updates it lists, or type \"flatpak update\" in a terminal. The web engine comes with the KDE runtime and is updated with it, or with a newer Sofa.")
                        : qsTr("Install your system's updates, in Discover or with your distribution's package manager. If your distribution has no newer Qt WebEngine, Sofa's Flatpak brings a current one.")));
                }
            }, 0);
    }

    Shortcut {
        sequence: "F11"
        onActivated: root.visibility === Window.FullScreen ? root.showNormal() : root.showFullScreen()
    }
    Shortcut {
        sequences: ["HomePage", "Alt+Home"]
        onActivated: root.goHome()
    }
    // With the pointer off (or a mouse in use), the page has the keys; these still reach Sofa.
    Shortcut {
        sequences: ["Back", "Esc"]
        enabled: root.screen === "page" && !root.menuShown && !root.host.remoteKeys
        onActivated: root.pageBack()
    }
    Shortcut {
        sequence: "Menu"
        enabled: root.screen === "page" && !root.menuShown && !root.host.remoteKeys
        onActivated: root.openMenu()
    }
    Shortcut {
        sequence: "Forward"
        enabled: root.screen === "page" && !root.host.remoteKeys
        onActivated: root.pageForward()
    }
    Shortcut {
        sequences: [StandardKey.Refresh]
        enabled: root.screen === "page" && root.currentView !== null
        onActivated: root.currentView.reload()
    }

    // A keyboard's shortcuts, as in desktop browsers. On a page or the start page, never over a dialog.
    readonly property bool shortcutsOn: (screen === "page" || screen === "start") && !dialogOpen
    Shortcut {
        sequences: ["Ctrl+L", "F6"]
        enabled: root.shortcutsOn
        onActivated: root.screen === "page" ? root.menuAction("address") : startPage.search()
    }
    Shortcut {
        sequence: "Ctrl+T"
        enabled: root.shortcutsOn
        onActivated: root.newTab()
    }
    Shortcut {
        sequence: "Ctrl+W"
        enabled: root.shortcutsOn
        onActivated: root.closeTabs([root.currentId])
    }
    Shortcut {
        sequences: ["Ctrl+Tab", "Ctrl+PgDown"]
        enabled: root.shortcutsOn
        onActivated: root.stepTab(1)
    }
    Shortcut {
        // Shift+Tab arrives as Backtab, with or without Shift among the modifiers.
        sequences: ["Ctrl+Shift+Tab", "Ctrl+Shift+Backtab", "Ctrl+Backtab", "Ctrl+PgUp"]
        enabled: root.shortcutsOn
        onActivated: root.stepTab(-1)
    }
    Shortcut {
        sequence: "Alt+Left"
        enabled: root.shortcutsOn && root.screen === "page"
        onActivated: root.pageHistory(false)
    }
    Shortcut {
        sequence: "Alt+Right"
        enabled: root.shortcutsOn && root.screen === "page"
        onActivated: root.pageHistory(true)
    }
    Shortcut {
        sequences: ["Ctrl++", "Ctrl+=", "Ctrl+Shift+=", "Ctrl+Num++"]
        enabled: root.shortcutsOn && root.screen === "page"
        onActivated: root.keyZoom("zoomIn")
    }
    Shortcut {
        sequences: ["Ctrl+-", "Ctrl+Num+-"]
        enabled: root.shortcutsOn && root.screen === "page"
        onActivated: root.keyZoom("zoomOut")
    }
    Shortcut {
        sequences: ["Ctrl+0", "Ctrl+Num+0"]
        enabled: root.shortcutsOn && root.screen === "page"
        onActivated: root.keyZoom("zoomReset")
    }

    // --- What is on screen ----------------------------------------------------------------------------------------

    Item {
        id: pageLayer
        anchors.fill: parent
        visible: root.screen === "page"

        Pointer {
            id: pointer
            anchors.fill: parent
            z: 10
            target: root.currentView
            host: root.host
            visible: root.pointerOn
            onHold: (x, y) => {
                if (root.currentView) {
                    root.host.mouse(root.currentView, 3, x, y);
                }
            }
        }

        BrowseMenu {
            id: menu
            anchors.fill: parent
            z: 12
            visible: root.menuShown
            focus: root.menuShown
            address: root.currentUrl
            privateMode: root.privateMode
            canGoBack: root.currentView ? root.currentView.canGoBack : false
            canGoForward: root.currentView ? root.currentView.canGoForward : false
            pointerOn: root.pointerOn
            zoom: root.currentView ? root.currentView.zoomFactor : 1
            tabs: {
                root.revision;
                const ids = JSON.parse(root.order.order());
                const list = ids.map(id => root.tabs[id]).filter(t => t);
                const names = JSON.parse(Session.tabNames(JSON.stringify(list.map(t => t.title)),
                                                          JSON.stringify(list.map(t => t.url)), qsTr("New tab")));
                return list.map((t, i) => ({ id: t.id, name: names[i] || qsTr("New tab"), current: t.id === root.currentId }));
            }
            onAction: name => root.menuAction(name)
            onTabChosen: id => {
                root.selectTab(id);
                root.showCurrent();
            }
            onTabHeld: id => root.askCloseTabs(id)
            onCloseRequested: root.closeMenu()
        }
    }

    Component {
        id: pageComponent

        WebEngineView {
            id: view

            property int tabId: -1
            // The page process ended while out of sight: reload when shown.
            property bool stopped: false
            // History counts the visits the user made: the next load is Sofa's own (a tab brought back at start, an
            // unloaded tab shown again, a reload after a crash) and doesn't count.
            property bool quietLoad: false
            // Whether the load under way counts as a visit; a reload doesn't.
            property bool countsAsVisit: true
            readonly property var tab: (root.revision, root.tabs[tabId] || null)

            // Running again; an unloaded page loads anew, which isn't a visit.
            function wake() {
                if (lifecycleState === WebEngineView.LifecycleState.Discarded) {
                    quietLoad = true;
                }
                if (lifecycleState !== WebEngineView.LifecycleState.Active) {
                    lifecycleState = WebEngineView.LifecycleState.Active;
                }
            }

            function reloadQuietly() {
                quietLoad = true;
                reload();
            }

            // Takes the focus out of the page's text field (in whichever frame it is), so the on-screen keyboard
            // hides and the remote's keys mean the pointer again.
            function leaveField() {
                const script = "{ const e = document.activeElement; if (e && e !== document.body && (e.isContentEditable"
                             + " || /^(INPUT|TEXTAREA|SELECT)$/.test(e.tagName))) e.blur(); }";
                const frames = [mainFrame];
                while (frames.length > 0) {
                    const frame = frames.pop();
                    frame.runJavaScript(script, WebEngineScript.ApplicationWorld, () => {});
                    frames.push(...frame.children);
                }
            }

            // Stops what the page plays: every <video> and <audio> in every frame, so it stays paused when the page
            // comes back (the user starts it again). Run where the page's scripts can't interfere.
            function pauseMedia() {
                const script = "for (const m of document.querySelectorAll('video, audio')) { if (!m.paused) m.pause(); }";
                const frames = [mainFrame];
                while (frames.length > 0) {
                    const frame = frames.pop();
                    frame.runJavaScript(script, WebEngineScript.ApplicationWorld, () => {});
                    frames.push(...frame.children);
                }
            }

            anchors.fill: parent
            // In sight: Sofa shows this tab, and Sofa's window itself can be seen.
            visible: root.screen === "page" && root.currentId === tabId && root.host.windowShown
            activeFocusOnPress: true
            settings.focusOnNavigationEnabled: false

            // Out of sight (another tab or screen in front, or Sofa's window hidden): muted at once, then its media
            // paused and the page frozen, so it can't play, run or open anything. In sight: running again, its media
            // still paused. The steps wait a little so a window hidden for a blink doesn't stop a video.
            onVisibleChanged: {
                if (visible) {
                    quietSteps.stop();
                    audioMuted = false;
                    wake();
                    if (stopped) {
                        stopped = false;
                        reloadQuietly();
                    }
                } else {
                    audioMuted = true;
                    quietSteps.step = 0;
                    quietSteps.restart();
                }
            }

            Timer {
                id: quietSteps

                property int step: 0

                interval: 400
                repeat: true
                onTriggered: {
                    if (view.visible || view.lifecycleState !== WebEngineView.LifecycleState.Active) {
                        stop();
                    } else if (step++ === 0) {
                        // The pause runs before the freeze, which would hold it back.
                        view.pauseMedia();
                    } else {
                        stop();
                        view.lifecycleState = WebEngineView.LifecycleState.Frozen;
                    }
                }
            }

            onUrlChanged: {
                const t = root.tabs[tabId];
                if (t && url.toString() !== "" && url.toString() !== t.url) {
                    t.url = url.toString();
                    root.revision++;
                    root.scheduleSave();
                }
            }

            onTitleChanged: {
                const t = root.tabs[tabId];
                if (!t) {
                    return;
                }
                t.title = title;
                root.revision++;
                if (!t.private) {
                    Library.updateTitle(url.toString(), title);
                }
            }

            // The pop-up and redirect rules of this tab: what the user pressed explains where the page may go.
            property PopupGuard guard: PopupGuard {}
            // Between a load's start and its end (the engine's own "loading" turns on earlier, for any navigation).
            property bool inLoad: false

            // Opens a window's first address in this tab, unless the pop-up rules block it. Only web pages open.
            function openWindow(target, opener, link) {
                if (!visible || !Session.isWebUrl(target)) {
                    return;
                }
                if (guard.window(target, opener, link)) {
                    // The window came from a press, so its redirects are the user's too.
                    guard.userNavigation();
                    url = target;
                } else {
                    guard.blocked(target);
                    root.notify(qsTr("Blocked a pop-up. To open it, use Allow once in the menu."));
                }
            }

            // The profile's press script (Host) says here what each press landed on.
            onJavaScriptConsoleMessage: (level, message, lineNumber, sourceId) => guard.report(message)

            onNavigationRequested: request => {
                if (!request.isMainFrame) {
                    return;
                }
                const type = request.navigationType;
                // The engine calls a meta refresh and location.replace() redirects too; those come once the page has
                // loaded, while a server's redirect comes during its navigation's load. Only that one is a hop.
                const redirect = type === WebEngineNavigationRequest.RedirectNavigation && inLoad;
                if (type === WebEngineNavigationRequest.TypedNavigation
                        || type === WebEngineNavigationRequest.BackForwardNavigation
                        || type === WebEngineNavigationRequest.ReloadNavigation) {
                    // Sofa's own loads, Back and Reload: not the page's doing.
                    guard.userNavigation();
                } else {
                    // Qt 6.12 says whether a press started it; before that, a recent press counts.
                    const gesture = request.userInitiated === undefined ? -1 : request.userInitiated ? 1 : 0;
                    const target = request.url.toString();
                    if (!guard.navigation(target, url.toString(), gesture, redirect)) {
                        request.reject();
                        guard.blocked(target);
                        root.notify(qsTr("Blocked a redirect. To follow it, use Allow once in the menu."));
                        return;
                    }
                }
                // A redirect keeps what the navigation it belongs to decided.
                if (redirect) {
                    return;
                }
                countsAsVisit = !quietLoad && type !== WebEngineNavigationRequest.ReloadNavigation;
                quietLoad = false;
            }

            onLoadingChanged: loadingInfo => {
                if (loadingInfo.status === WebEngineLoadingInfo.LoadStartedStatus) {
                    guard.pageStarted();
                }
                inLoad = loadingInfo.status === WebEngineLoadingInfo.LoadStartedStatus;
                // A load the engine starts without asking (an unloaded page coming back) still meets Sofa's mark.
                if (loadingInfo.status === WebEngineLoadingInfo.LoadStartedStatus && quietLoad) {
                    countsAsVisit = false;
                    quietLoad = false;
                }
                const t = root.tabs[tabId];
                if (t && !t.private && countsAsVisit && loadingInfo.status === WebEngineLoadingInfo.LoadSucceededStatus) {
                    Library.recordVisit(title, loadingInfo.url.toString());
                }
            }

            // Private by default: pages get no camera, microphone, location, screen or notifications.
            onPermissionRequested: permission => permission.deny()

            // A window the page opens on a press (a link with target=_blank, window.open on a click) opens in this
            // tab if the pop-up rules allow it; windows a page opens on its own are dropped. Only the user opens tabs.
            // A window opened blank gets its address later: a hidden view catches the first one.
            onNewWindowRequested: request => {
                if (!guard.windowRequested(request.userInitiated)) {
                    root.notify(qsTr("Pop-up blocked"));
                    return;
                }
                // What the press was on now, before the page can move things.
                const link = guard.link();
                const opener = url.toString();
                const first = request.requestedUrl.toString();
                if (first === "" || first === "about:blank") {
                    root.catchWindow(view, request, opener, link);
                } else {
                    openWindow(first, opener, link);
                }
            }

            // A link held with OK (or right-clicked): Sofa's link menu instead of the engine's.
            onContextMenuRequested: request => {
                request.accepted = true;
                const link = request.linkUrl.toString();
                const found = link !== "" && Session.isWebUrl(link);
                pointer.answered(found);
                if (found) {
                    root.askLink(link, request.linkText);
                }
            }

            // Leaving the page's full screen gives the window back the mode it had before (F11 may have changed it).
            onFullScreenRequested: request => {
                request.accept();
                if (request.toggleOn) {
                    if (root.visibility !== Window.FullScreen) {
                        root.windowedBeforePage = true;
                    }
                    root.showFullScreen();
                } else if (root.windowedBeforePage) {
                    root.windowedBeforePage = false;
                    root.showNormal();
                }
            }

            // The engine's page process ended (out of memory, a crash): the tab reloads now if in sight, or else
            // when shown again.
            onRenderProcessTerminated: (status, code) => {
                if (status === WebEngineView.NormalTerminationStatus) {
                    return;
                }
                if (visible) {
                    root.notify(qsTr("This page stopped. Reloading it."));
                    reloadQuietly();
                } else {
                    stopped = true;
                }
            }
        }
    }

    // A window opened blank (window.open("") and an address set later): a hidden view in the opener's profile takes
    // it, hands its first address to the opener's tab for the pop-up rules, and goes. It never loads anything; a
    // window that gets no address in 10 seconds (filled by the page's script instead) is dropped.
    Component {
        id: catcherComponent

        WebEngineView {
            id: catcher

            property var done: null

            visible: false
            width: 1
            height: 1
            audioMuted: true
            onNavigationRequested: request => {
                request.reject();
                if (request.isMainFrame && done) {
                    const handOver = done;
                    done = null;
                    handOver(request.url.toString());
                    Qt.callLater(catcher.destroy);
                }
            }
            onPermissionRequested: permission => permission.deny()
            onNewWindowRequested: request => {}

            Timer {
                interval: 10000
                running: true
                onTriggered: catcher.destroy()
            }
        }
    }

    function catchWindow(view, request, opener, link) {
        const catcher = catcherComponent.createObject(pageLayer, { profile: view.profile });
        // A tab closed meanwhile is null here.
        catcher.done = target => {
            if (view) {
                view.openWindow(target, opener, link);
            }
        };
        request.openIn(catcher);
    }

    function askLink(link, text) {
        linkMenu.ask(text && text.trim() !== "" ? text.trim() : link, link, [
            { text: qsTr("Open in new tab"), value: "tab" },
            { text: qsTr("Open"), value: "open" },
            { text: Library.isBookmarked(link) ? qsTr("Remove bookmark") : qsTr("Bookmark link"), value: "bookmark" }
        ], value => {
            if (value === "tab") {
                openInNewTab(link);
            } else if (value === "open") {
                openInCurrent(link);
            } else {
                const now = Library.toggleBookmark(text && text.trim() !== "" ? text.trim() : link, link);
                notify(now ? qsTr("Bookmarked") : qsTr("Bookmark removed"));
            }
        });
    }

    StartPage {
        id: startPage
        anchors.fill: parent
        visible: root.screen === "start"
        focus: visible
        privateMode: root.privateMode
        onBackPressed: root.backOnStart()
        onTyped: text => {
            root.openScreen("search");
            searchScreen.startTyping(text);
        }
        onOpenUrl: url => root.openInCurrent(url)
        onSearch: {
            root.openScreen("search");
            searchScreen.start("");
        }
        onOpenBookmarks: folder => {
            root.openScreen("bookmarks");
            bookmarksScreen.start(folder);
        }
        onOpenHistory: {
            root.openScreen("history");
            historyScreen.start();
        }
        onTogglePrivate: root.togglePrivate()
        onOpenSettings: root.openSettings()
        onExitRequested: root.quit()
        onTileHeld: tile => tileMenu.ask(tile.title, tile.url, [
            { text: qsTr("Move"), detail: qsTr("Then Left and Right, and OK to put it down"), value: "move" },
            { text: qsTr("Edit"), value: "edit" },
            { text: qsTr("Remove"), value: "remove" }
        ], value => {
            if (value === "move") {
                startPage.startMoving(tile.id);
            } else if (value === "edit") {
                root.editTile(tile);
            } else {
                Library.removeTile(tile.id);
                startPage.refresh();
                startPage.focusFirst();
            }
        })
        onAddTile: root.editTile(null)
    }

    // Adds a Quick access tile (`tile` null) or changes one: its address, then its name.
    function editTile(tile) {
        tileDialog.ask({ title: tile ? qsTr("Site address") : qsTr("Add site"), text: tile ? tile.url : "https://", address: true,
                         message: qsTr("The page this tile opens.") }, typed => {
            const url = Session.webAddress(typed);
            if (url === "") {
                return qsTr("Type a web address, such as example.com.");
            }
            Qt.callLater(() => tileDialog.ask({ title: qsTr("Tile name"), text: tile ? tile.title : Session.label(url) }, name => {
                if (tile) {
                    Library.updateTile(tile.id, name, url);
                } else {
                    Library.addTile(name, url);
                }
                startPage.refresh();
                startPage.focusFirst();
                return "";
            }));
            return "";
        });
    }

    SearchScreen {
        id: searchScreen
        anchors.fill: parent
        visible: root.screen === "search"
        focus: visible
        onOpenText: text => root.openInCurrent(Session.resolveAddress(text))
        onOpenUrl: url => root.openInCurrent(url)
        onBack: root.screenBack()
    }

    BookmarksScreen {
        id: bookmarksScreen
        anchors.fill: parent
        visible: root.screen === "bookmarks"
        focus: visible
        onOpenUrl: url => root.openInCurrent(url)
        onBack: root.screenBack()
    }

    HistoryScreen {
        id: historyScreen
        anchors.fill: parent
        visible: root.screen === "history"
        focus: visible
        onOpenUrl: url => root.openInCurrent(url)
        onBack: root.screenBack()
    }

    SettingsScreen {
        id: settingsScreen
        anchors.fill: parent
        visible: root.screen === "settings"
        focus: visible
        onChooseEngine: {
            root.openScreen("setup");
            setupScreen.start();
        }
        onBack: root.screenBack()
    }

    SetupScreen {
        id: setupScreen
        anchors.fill: parent
        visible: root.screen === "setup"
        focus: visible
        onDone: {
            if (root.afterSetup) {
                const next = root.afterSetup;
                root.afterSetup = null;
                next();
            } else {
                root.screenBack();
            }
        }
    }

    ChoiceDialog {
        id: engineNotice
        onClosed: Qt.callLater(() => {
            if (engineNotice.visible) {
                return;
            }
            if (root.screen === "page" && root.currentView) {
                root.currentView.forceActiveFocus();
            } else if (root.screen === "start") {
                startPage.focusFirst();
            }
        })
    }
    ChoiceDialog {
        id: linkMenu
        onClosed: if (root.screen === "page" && root.currentView) root.currentView.forceActiveFocus()
    }
    ChoiceDialog {
        id: tabMenu
        onClosed: if (root.menuShown) menu.show()
    }
    ChoiceDialog {
        id: tileMenu
        onClosed: if (root.screen === "start" && !tileDialog.visible && startPage.movingTile < 0) startPage.focusFirst()
    }
    TextDialog {
        id: tileDialog
        onClosed: if (root.screen === "start" && !tileMenu.visible) startPage.focusFirst()
    }

    // Short notices ("Bookmarked", "Pop-up blocked"), over everything but the dialogs.
    BigscreenUI.Toast {
        id: toast
        z: 100
        timeout: 4000
        // Over the menu's panel when it is open, so it never covers a button.
        anchors.bottomMargin: root.menuShown ? menu.panelHeight + Kirigami.Units.gridUnit
                                             : BigscreenUI.Units.verticalScreenMarginFor(root.height)
    }
}
