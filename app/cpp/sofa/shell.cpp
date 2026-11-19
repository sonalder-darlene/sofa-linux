// SPDX-License-Identifier: GPL-3.0-only

#include "sofa/shell.h"
#include "sofa/host.h"

#include <QtCore/QVariantMap>
#include <QtGui/QGuiApplication>
#include <QtGui/QIcon>
#include <QtWebEngineCore/QWebEngineUrlRequestInfo>
#include <QtWebEngineCore/QWebEngineUrlRequestInterceptor>
#include <QtWebEngineCore/qtwebenginecoreglobal.h>
#include <QtWebEngineQuick/QQuickWebEngineProfile>
#include <QtWebEngineQuick/QtWebEngineQuick>

#include "sofa-tv-browser/src/shell.cxxqt.h"

namespace sofa {

namespace {

// Asks the Rust policy about every request the engine is about to send.
class RequestInterceptor final : public QWebEngineUrlRequestInterceptor
{
public:
    RequestInterceptor(rust::Box<RequestPolicy> policy, QObject *parent)
        : QWebEngineUrlRequestInterceptor(parent)
        , m_policy(std::move(policy))
    {
    }

    void interceptRequest(QWebEngineUrlRequestInfo &info) override
    {
        const QByteArray url = info.requestUrl().toEncoded();
        const rust::Slice<const std::uint8_t> bytes(reinterpret_cast<const std::uint8_t *>(url.constData()),
                                                    static_cast<std::size_t>(url.size()));
        if (m_policy->shouldBlock(bytes)) {
            info.block(true);
        }
    }

private:
    rust::Box<RequestPolicy> m_policy;
};

} // namespace

void initWebEngine()
{
    QtWebEngineQuick::initialize();
}

bool prepareEngine(QQmlApplicationEngine &engine, const QString &storageName, rust::Box<RequestPolicy> policy,
                   const QString &startUrl, bool fullScreen)
{
    // One interceptor for every profile (the default one and each private session's), owned by the engine so it
    // outlives them all.
    auto *interceptor = new RequestInterceptor(std::move(policy), &engine);
    auto *host = new Host(interceptor, &engine);
    // A Sofa already running takes the address, before this one touches the profile.
    if (host->claimOrHandOver(QGuiApplication::desktopFileName(), startUrl)) {
        return false;
    }

    // Outside Plasma there may be no icon theme set; Breeze (KDE's) fills in where it is installed.
    if (QIcon::fallbackThemeName().isEmpty() || QIcon::fallbackThemeName() == QStringLiteral("hicolor")) {
        QIcon::setFallbackThemeName(QStringLiteral("breeze"));
    }

    auto *profile = new QQuickWebEngineProfile(&engine);
    profile->setStorageName(storageName);
    profile->setOffTheRecord(false);
    profile->setUrlRequestInterceptor(interceptor);

    engine.setInitialProperties(QVariantMap{
        {QStringLiteral("profile"), QVariant::fromValue(profile)},
        {QStringLiteral("host"), QVariant::fromValue(host)},
        {QStringLiteral("startUrl"), startUrl},
        {QStringLiteral("startFullScreen"), fullScreen},
    });
    return true;
}

bool hasWindow(const QQmlApplicationEngine &engine)
{
    return !engine.rootObjects().isEmpty();
}

QString engineVersions()
{
    return QStringLiteral("Qt %1, QtWebEngine %2, Chromium %3 (security patches up to %4)")
        .arg(QString::fromLatin1(qVersion()), QString::fromLatin1(qWebEngineVersion()),
             QString::fromLatin1(qWebEngineChromiumVersion()),
             QString::fromLatin1(qWebEngineChromiumSecurityPatchVersion()));
}

QString platformName()
{
    return QGuiApplication::platformName();
}

} // namespace sofa
