// SPDX-License-Identifier: GPL-3.0-only

#include "sofa/shell.h"

#include <QtCore/QVariantMap>
#include <QtGui/QGuiApplication>
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

void prepareEngine(QQmlApplicationEngine &engine, const QString &storageName, rust::Box<RequestPolicy> policy,
                   const QString &startUrl, bool fullScreen)
{
    auto *profile = new QQuickWebEngineProfile(&engine);
    profile->setStorageName(storageName);
    profile->setOffTheRecord(false);
    profile->setUrlRequestInterceptor(new RequestInterceptor(std::move(policy), profile));

    engine.setInitialProperties(QVariantMap{
        {QStringLiteral("profile"), QVariant::fromValue(profile)},
        {QStringLiteral("startUrl"), startUrl},
        {QStringLiteral("startFullScreen"), fullScreen},
    });
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
