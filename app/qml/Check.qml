// SPDX-License-Identifier: GPL-3.0-only
//
// `sofa-tv-browser --check`: asks the engine what it can play, prints it through Core.report and quits.
// The page is a blank one in a throwaway profile; nothing is fetched from the network.

import QtQuick
import QtWebEngine
import Sofa

Window {
    id: root

    width: 320
    height: 180
    visible: true
    title: "Sofa check"

    readonly property string probe: `
        (async () => {
            const types = [
                ["H.264", 'video/mp4; codecs="avc1.42E01E"'],
                ["HEVC", 'video/mp4; codecs="hvc1.1.6.L93.B0"'],
                ["VP9", 'video/webm; codecs="vp09.00.10.08"'],
                ["AV1", 'video/mp4; codecs="av01.0.05M.08"'],
                ["AAC", 'audio/mp4; codecs="mp4a.40.2"'],
                ["Opus", 'audio/webm; codecs="opus"'],
            ];
            const video = document.createElement("video");
            const lines = [];
            for (const [name, type] of types) {
                const plays = video.canPlayType(type) !== "";
                const streams = !!window.MediaSource && MediaSource.isTypeSupported(type);
                lines.push(name + ": " + (plays ? "plays" : "no") + (streams ? ", streams" : ""));
            }
            let drm;
            try {
                await navigator.requestMediaKeySystemAccess("com.widevine.alpha", [{
                    initDataTypes: ["cenc"],
                    videoCapabilities: [{ contentType: 'video/mp4; codecs="avc1.42E01E"', robustness: "" }],
                }]);
                drm = "available";
            } catch (e) {
                drm = "not available";
            }
            lines.push("Widevine DRM: " + drm);
            window.sofaCheck = lines.join("\\n");
        })();
    `

    // No storage name: an off-the-record profile, nothing kept on disk.
    WebEngineProfilePrototype {
        id: throwaway
    }

    WebEngineView {
        id: view
        anchors.fill: parent
        profile: throwaway.instance()
        // EME needs a secure context; the page is local and empty, the origin only names it.
        Component.onCompleted: loadHtml("<!doctype html><title>check</title>", "https://check.invalid/")
        onLoadingChanged: info => {
            if (info.status === WebEngineLoadingInfo.LoadSucceededStatus) {
                runJavaScript(root.probe);
                poll.start();
            } else if (info.status === WebEngineLoadingInfo.LoadFailedStatus) {
                Core.report("Media check: the test page did not load (" + info.errorString + ")");
                Qt.exit(1);
            }
        }
    }

    Timer {
        id: poll
        interval: 100
        repeat: true
        property int tries: 0
        onTriggered: {
            if (++tries > 150) {
                Core.report("Media check: no answer from the engine");
                Qt.exit(1);
                return;
            }
            view.runJavaScript("window.sofaCheck", result => {
                if (result !== undefined && running) {
                    stop();
                    Core.report(result);
                    Qt.exit(0);
                }
            });
        }
    }
}
