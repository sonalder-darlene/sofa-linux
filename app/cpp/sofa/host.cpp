// SPDX-License-Identifier: GPL-3.0-only

#include "sofa/host.h"

#include <QtCore/QCoreApplication>
#include <QtDBus/QDBusConnection>
#include <QtDBus/QDBusMessage>
#include <QtGui/QGuiApplication>
#include <QtGui/QInputMethodQueryEvent>
#include <QtGui/QKeyEvent>
#include <QtGui/QMouseEvent>
#include <QtGui/QPointingDevice>
#include <QtGui/QWheelEvent>
#include <QtQuick/QQuickWindow>
#include <QtWebEngineCore/QWebEngineScript>
#include <QtWebEngineQuick/QQuickWebEngineProfile>

namespace sofa {

namespace {

bool isRemoteKey(int key)
{
    switch (key) {
    case Qt::Key_Left:
    case Qt::Key_Right:
    case Qt::Key_Up:
    case Qt::Key_Down:
    case Qt::Key_Return:
    case Qt::Key_Enter:
    case Qt::Key_Select:
    case Qt::Key_Escape:
    case Qt::Key_Back:
    case Qt::Key_Menu:
    case Qt::Key_HomePage:
    case Qt::Key_PageUp:
    case Qt::Key_PageDown:
    case Qt::Key_ChannelUp:
    case Qt::Key_ChannelDown:
        return true;
    default:
        return false;
    }
}

QString objectPath(const QString &appId)
{
    QString path = QLatin1Char('/') + appId;
    path.replace(QLatin1Char('.'), QLatin1Char('/')).replace(QLatin1Char('-'), QLatin1Char('_'));
    return path;
}

} // namespace

bool Host::claimOrHandOver(const QString &appId, const QString &url)
{
    QDBusConnection bus = QDBusConnection::sessionBus();
    if (!bus.isConnected()) {
        return false;
    }
    if (bus.registerService(appId)) {
        bus.registerObject(objectPath(appId), new ApplicationInterface(this), QDBusConnection::ExportScriptableSlots);
        return false;
    }
    // The name is taken, so another Sofa owns the profile: this one never runs beside it, even when the other one
    // doesn't answer (its window busy with a page). The wait is short, so a stuck Sofa doesn't hold this launch up.
    QDBusMessage call = QDBusMessage::createMethodCall(appId, objectPath(appId),
                                                       QStringLiteral("org.freedesktop.Application"),
                                                       url.isEmpty() ? QStringLiteral("Activate") : QStringLiteral("Open"));
    if (url.isEmpty()) {
        call << QVariantMap();
    } else {
        call << QStringList{url} << QVariantMap();
    }
    const QDBusMessage reply = bus.call(call, QDBus::Block, 3000);
    if (reply.type() == QDBusMessage::ErrorMessage) {
        qWarning("Sofa is already running but did not answer (%s); not starting a second copy.",
                 qPrintable(reply.errorName()));
    }
    return true;
}

Host::Host(QWebEngineUrlRequestInterceptor *interceptor, const QString &pageScript, QObject *parent)
    : QObject(parent)
    , m_interceptor(interceptor)
    , m_pageScript(pageScript)
{
    qApp->installEventFilter(this);
}

void Host::setRemoteKeys(bool on)
{
    if (on == m_remoteKeys) {
        return;
    }
    m_remoteKeys = on;
    Q_EMIT remoteKeysChanged();
}

void Host::setTyping(bool on)
{
    if (on == m_typing) {
        return;
    }
    m_typing = on;
    Q_EMIT typingChanged();
}

QObject *Host::newPrivateProfile()
{
    // A profile without a storage name keeps nothing on disk: cookies, storage and cache live in memory and go
    // with the profile. (Made through the named constructor: since Qt 6.9 the other one is on its way out.)
    auto *profile = new QQuickWebEngineProfile(QString(), this);
    profile->setUrlRequestInterceptor(m_interceptor);
    addPageScript(profile);
    return profile;
}

void Host::addPageScript(QQuickWebEngineProfile *profile) const
{
    QWebEngineScript script;
    script.setName(QStringLiteral("sofa-press"));
    script.setSourceCode(m_pageScript);
    script.setInjectionPoint(QWebEngineScript::DocumentCreation);
    script.setWorldId(QWebEngineScript::ApplicationWorld);
    script.setRunsOnSubFrames(true);
    // The profile's script list has no public C++ header, only its QML API, so insert() is called through that. Once
    // per profile rather than per view in QML: every view and window of the profile then has it, catchers included.
    auto *scripts = profile->property("userScripts").value<QObject *>();
    if (!scripts || !QMetaObject::invokeMethod(scripts, "insert", Q_ARG(QWebEngineScript, script))) {
        qWarning("The page script could not be added: the pop-up rules won't know which link was pressed.");
    }
}

void Host::endPrivateProfile(QObject *profile)
{
    if (auto *p = qobject_cast<QQuickWebEngineProfile *>(profile); p && p->parent() == this) {
        p->deleteLater();
    }
}

void Host::mouse(QQuickItem *item, int kind, qreal x, qreal y)
{
    if (!item || !item->window()) {
        return;
    }
    QQuickWindow *window = item->window();
    const QPointF scene = item->mapToScene(QPointF(x, y));
    const QPointF global = window->mapToGlobal(scene);
    auto send = [&](QEvent::Type type, Qt::MouseButton button, Qt::MouseButtons buttons) {
        QMouseEvent event(type, scene, scene, global, button, buttons, Qt::NoModifier,
                          QPointingDevice::primaryPointingDevice());
        QCoreApplication::sendEvent(window, &event);
    };
    switch (kind) {
    case 0:
        send(QEvent::MouseMove, Qt::NoButton, m_buttons);
        break;
    case 1:
        m_buttons = Qt::LeftButton;
        send(QEvent::MouseButtonPress, Qt::LeftButton, m_buttons);
        break;
    case 2:
        m_buttons = Qt::NoButton;
        send(QEvent::MouseButtonRelease, Qt::LeftButton, m_buttons);
        break;
    case 3:
        send(QEvent::MouseButtonPress, Qt::RightButton, Qt::RightButton);
        send(QEvent::MouseButtonRelease, Qt::RightButton, Qt::NoButton);
        break;
    default:
        break;
    }
}

void Host::wheel(QQuickItem *item, qreal x, qreal y, qreal dx, qreal dy)
{
    if (!item || !item->window()) {
        return;
    }
    QQuickWindow *window = item->window();
    const QPointF scene = item->mapToScene(QPointF(x, y));
    // Wheel deltas point the other way: a positive angle scrolls up. 120 eighths of a degree is one notch, which
    // Chromium turns into about 53 pixels; both deltas are given so either reading scrolls the same distance.
    const QPoint pixels(qRound(-dx), qRound(-dy));
    const QPoint angle(qRound(-dx * 120.0 / 53.0), qRound(-dy * 120.0 / 53.0));
    QWheelEvent event(scene, window->mapToGlobal(scene), pixels, angle, Qt::NoButton, Qt::NoModifier,
                      Qt::NoScrollPhase, false, Qt::MouseEventNotSynthesized,
                      QPointingDevice::primaryPointingDevice());
    QCoreApplication::sendEvent(window, &event);
}

bool Host::editing() const
{
    QObject *focus = QGuiApplication::focusObject();
    if (!focus) {
        return false;
    }
    QInputMethodQueryEvent query(Qt::ImEnabled);
    QCoreApplication::sendEvent(focus, &query);
    return query.value(Qt::ImEnabled).toBool();
}

bool Host::isPress(QEvent *event) const
{
    switch (event->type()) {
    case QEvent::MouseButtonPress:
    case QEvent::TouchBegin:
        return true;
    case QEvent::KeyPress: {
        auto *key = static_cast<QKeyEvent *>(event);
        if (key->isAutoRepeat()) {
            return false;
        }
        switch (key->key()) {
        case Qt::Key_Space:
            return true;
        case Qt::Key_Return:
        case Qt::Key_Enter:
        case Qt::Key_Select:
            // Taken for the pointer (which then clicks, a press of its own) unless the page has them.
            return !m_remoteKeys || (m_typing && key->key() != Qt::Key_Select);
        default:
            return false;
        }
    }
    default:
        return false;
    }
}

bool Host::eventFilter(QObject *watched, QEvent *event)
{
    const QEvent::Type type = event->type();
    // The compositor says when the window stops being seen (Wayland's "suspended" state, an unmapped X11 window).
    if (type == QEvent::Expose && watched->isWindowType()) {
        auto *window = static_cast<QWindow *>(watched);
        if (!window->transientParent() && window->isExposed() != m_windowShown) {
            m_windowShown = window->isExposed();
            Q_EMIT windowShownChanged();
        }
    }
    // Sofa's screens are made of KDE's TV controls, which take OK as Return: a remote's Select is sent on as Return
    // (and then counted as a press, once).
    if (!m_remoteKeys && watched->isWindowType() && (type == QEvent::KeyPress || type == QEvent::KeyRelease)) {
        auto *key = static_cast<QKeyEvent *>(event);
        if (key->key() == Qt::Key_Select) {
            QKeyEvent ok(type, Qt::Key_Return, key->modifiers(), QStringLiteral("\r"), key->isAutoRepeat(),
                         static_cast<quint16>(key->count()));
            QCoreApplication::sendEvent(watched, &ok);
            return true;
        }
    }
    // Each event reaches the window first, so a press is counted once, there.
    if (watched->isWindowType() && isPress(event)) {
        Q_EMIT pressed();
    }
    if (!m_remoteKeys || (type != QEvent::KeyPress && type != QEvent::KeyRelease && type != QEvent::ShortcutOverride)) {
        return QObject::eventFilter(watched, event);
    }
    // A key press passes through here once per object it is sent to (the window, then the item in focus); only the
    // window's copy is taken, so each press is seen once. The shortcut check goes to the item in focus first.
    if (type != QEvent::ShortcutOverride && !watched->isWindowType()) {
        return QObject::eventFilter(watched, event);
    }
    auto *key = static_cast<QKeyEvent *>(event);
    if (!isRemoteKey(key->key())) {
        return QObject::eventFilter(watched, event);
    }
    if (m_typing && (key->key() == Qt::Key_Return || key->key() == Qt::Key_Enter)) {
        return QObject::eventFilter(watched, event);
    }
    if (type == QEvent::ShortcutOverride) {
        // Taken here, so neither the page nor a window shortcut claims the key.
        event->accept();
        return true;
    }
    if (!key->isAutoRepeat()) {
        Q_EMIT remoteKey(key->key(), type == QEvent::KeyPress);
    }
    return true;
}

} // namespace sofa
