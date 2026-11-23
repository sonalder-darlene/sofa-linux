// SPDX-License-Identifier: GPL-3.0-only
//
// What the QML screens need from Qt that QML can't do itself: private profiles that keep the request rules, real
// mouse and wheel events for the pointer, and the remote's keys while a page is in front. It forwards and decides
// nothing about pages; the rules live in the Rust core.

#pragma once

#include <QtCore/QElapsedTimer>
#include <QtCore/QList>
#include <QtCore/QObject>
#include <QtCore/QPointer>
#include <QtCore/QPointF>
#include <QtCore/QStringList>
#include <QtCore/QVariantMap>
#include <QtQuick/QQuickItem>

class QQuickWebEngineProfile;
class QWebEngineUrlRequestInterceptor;

namespace sofa {

class Host final : public QObject
{
    Q_OBJECT
    // While true, the remote's keys (arrows, OK, Back, Menu, Home, page and channel keys) go to remoteKey() instead
    // of the item in focus: the page in front never sees them, so the pointer can use them.
    Q_PROPERTY(bool remoteKeys READ remoteKeys WRITE setRemoteKeys NOTIFY remoteKeysChanged)
    // While true (the user is typing in a page's field), Return and Enter from a keyboard reach the page, so a
    // search form can be sent. The remote's OK (Select) always stays with the pointer.
    Q_PROPERTY(bool typing READ typing WRITE setTyping NOTIFY typingChanged)
    // False while Sofa's window can't be seen: behind Bigscreen's home or another app the compositor hides it from,
    // minimized, or on another desktop. Losing the keyboard focus to a window beside it doesn't count.
    Q_PROPERTY(bool windowShown READ windowShown NOTIFY windowShownChanged)
    // While true, the system's pointer is hidden over Sofa's windows (Sofa's own pointer is the one in use).
    Q_PROPERTY(bool hideCursor READ hideCursor WRITE setHideCursor NOTIFY hideCursorChanged)

public:
    // `pageScript` runs at the start of every frame of every page, in a world the page's own scripts can't reach
    // (the pop-up rules' press script).
    Host(QWebEngineUrlRequestInterceptor *interceptor, const QString &pageScript, QObject *parent);

    bool remoteKeys() const { return m_remoteKeys; }
    void setRemoteKeys(bool on);
    bool typing() const { return m_typing; }
    void setTyping(bool on);
    bool windowShown() const { return m_windowShown; }
    bool hideCursor() const { return m_hideCursor; }
    void setHideCursor(bool on);

    // A new profile for one private browsing session: kept in memory only, set up like the default one. Null when
    // the engine would keep it on disk after all: then there is no private browsing. endPrivateProfile() lets go of
    // it once no page uses it.
    Q_INVOKABLE QObject *newPrivateProfile();
    Q_INVOKABLE void endPrivateProfile(QObject *profile);
    // What every profile gets (the default one here, private ones when made): the request rules, the page script,
    // no third-party cookies, and the user agent of Chrome on Linux with the engine's own version.
    void setUpProfile(QQuickWebEngineProfile *profile) const;

    // A real mouse event at (x, y) in `item`: 0 move, 1 left press, 2 left release, 3 right click.
    Q_INVOKABLE void mouse(QQuickItem *item, int kind, qreal x, qreal y);
    // A real wheel event at (x, y) in `item`, scrolling by (dx, dy) pixels (positive: down and right).
    Q_INVOKABLE void wheel(QQuickItem *item, qreal x, qreal y, qreal dx, qreal dy);
    // Whether the item in focus takes typed text (a field in the page, for one).
    Q_INVOKABLE bool editing() const;

    // Makes this Sofa the one that answers on the session bus as `appId` (org.freedesktop.Application). When another
    // Sofa already does, hands `url` to it ("" just brings it to the front, waiting at most 3 s) and returns true,
    // whether or not the other one answered: this one then quits, so two copies never share one profile. Without a
    // session bus, it returns false and Sofa runs on its own.
    bool claimOrHandOver(const QString &appId, const QString &url);

Q_SIGNALS:
    void remoteKeysChanged();
    void typingChanged();
    // Another launch asked this Sofa to open `url` ("" to come to the front).
    void openRequested(const QString &url);
    // A remote key while remoteKeys is on; `key` is the Qt::Key, `pressed` false on release. Repeats are left out:
    // the pointer runs on its own clock between press and release.
    void remoteKey(int key, bool pressed);
    // A real pointer was used over the window (a mouse, a touchpad, a pad's stick that the system turns into one, a
    // touch): it moved, pressed or scrolled at (x, y) in the window. The events themselves still go to what is under
    // it; this only tells Sofa's own pointer to step aside. Sofa's own events (mouse(), wheel()) never count.
    void realPointer(qreal x, qreal y);
    // Something was pressed that may reach the page in front: a click (the pointer's too), a touch, or a key that
    // activates what has focus. The pop-up rules count presses; what the press landed on the page says itself.
    void pressed();
    void windowShownChanged();
    void hideCursorChanged();

protected:
    bool eventFilter(QObject *watched, QEvent *event) override;

private:
    bool isPress(QEvent *event) const;
    // The remote's keys go to the pointer: asked for, and no pop-up of the engine is up.
    bool remoteActive() const { return m_remoteKeys && m_popups.isEmpty(); }
    void noticeRealPointer(QEvent *event);
    void trackPopup(QWindow *window, bool shown);

    QWebEngineUrlRequestInterceptor *m_interceptor;
    QString m_pageScript;
    bool m_remoteKeys = false;
    bool m_typing = false;
    bool m_windowShown = true;
    bool m_hideCursor = false;
    Qt::MouseButtons m_buttons = Qt::NoButton;
    // Sofa's own events carry the time they were made, in milliseconds since Sofa started: the engine counts a
    // second press as a double click when it comes within the system's interval of the one before, and an event
    // without a time looks like it came at once.
    QElapsedTimer m_clock;
    // While Sofa sends its own mouse or wheel event.
    bool m_sending = false;
    // The engine's pop-up windows on show (a list of choices opened by a press): while one is up the keys belong to
    // it, not to Sofa's pointer.
    QList<QPointer<QWindow>> m_popups;
    // Where the real pointer was last seen; none until the first event.
    QPointF m_realAt;
    bool m_realSeen = false;
};

// The standard interface a desktop app answers on the session bus (what DBusActivatable launchers call): another
// launch of Sofa uses it to hand over its address.
class ApplicationInterface final : public QObject
{
    Q_OBJECT
    Q_CLASSINFO("D-Bus Interface", "org.freedesktop.Application")

public:
    explicit ApplicationInterface(Host *host)
        : QObject(host)
        , m_host(host)
    {
    }

public Q_SLOTS:
    Q_SCRIPTABLE void Activate(const QVariantMap &platformData)
    {
        takeToken(platformData);
        Q_EMIT m_host->openRequested(QString());
    }

    Q_SCRIPTABLE void Open(const QStringList &uris, const QVariantMap &platformData)
    {
        takeToken(platformData);
        for (const QString &uri : uris) {
            Q_EMIT m_host->openRequested(uri);
        }
    }

    Q_SCRIPTABLE void ActivateAction(const QString &, const QVariantList &, const QVariantMap &platformData)
    {
        takeToken(platformData);
        Q_EMIT m_host->openRequested(QString());
    }

private:
    // The launch that called carries the compositor's permission to bring a window to the front (an XDG activation
    // token, given to whatever the user started). Qt's Wayland support uses it on the next requestActivate(), which
    // openRequested() leads to; without it KWin keeps Sofa behind (focus-stealing prevention).
    static void takeToken(const QVariantMap &platformData)
    {
        const QByteArray token = platformData.value(QStringLiteral("activation-token")).toString().toUtf8();
        if (!token.isEmpty()) {
            qputenv("XDG_ACTIVATION_TOKEN", token);
        }
    }

    Host *m_host;
};

} // namespace sofa
