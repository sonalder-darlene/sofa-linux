// SPDX-License-Identifier: GPL-3.0-only
//
// What the QML screens need from Qt that QML can't do itself: private profiles that keep the request rules, real
// mouse and wheel events for the pointer, and the remote's keys while a page is in front. It forwards and decides
// nothing about pages; the rules live in the Rust core.

#pragma once

#include <QtCore/QObject>
#include <QtCore/QPointF>
#include <QtCore/QStringList>
#include <QtCore/QVariantMap>
#include <QtQuick/QQuickItem>

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

public:
    Host(QWebEngineUrlRequestInterceptor *interceptor, QObject *parent);

    bool remoteKeys() const { return m_remoteKeys; }
    void setRemoteKeys(bool on);
    bool typing() const { return m_typing; }
    void setTyping(bool on);

    // A new profile for one private browsing session: kept in memory only, with the same request rules as the
    // default one. endPrivateProfile() lets go of it once no page uses it.
    Q_INVOKABLE QObject *newPrivateProfile();
    Q_INVOKABLE void endPrivateProfile(QObject *profile);

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

protected:
    bool eventFilter(QObject *watched, QEvent *event) override;

private:
    QWebEngineUrlRequestInterceptor *m_interceptor;
    bool m_remoteKeys = false;
    bool m_typing = false;
    Qt::MouseButtons m_buttons = Qt::NoButton;
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
    Q_SCRIPTABLE void Activate(const QVariantMap &)
    {
        Q_EMIT m_host->openRequested(QString());
    }

    Q_SCRIPTABLE void Open(const QStringList &uris, const QVariantMap &)
    {
        for (const QString &uri : uris) {
            Q_EMIT m_host->openRequested(uri);
        }
    }

    Q_SCRIPTABLE void ActivateAction(const QString &, const QVariantList &, const QVariantMap &)
    {
        Q_EMIT m_host->openRequested(QString());
    }

private:
    Host *m_host;
};

} // namespace sofa
