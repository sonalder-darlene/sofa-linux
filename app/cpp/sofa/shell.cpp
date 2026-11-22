// SPDX-License-Identifier: GPL-3.0-only

#include "sofa/shell.h"
#include "sofa/host.h"

#include <QtCore/QDir>
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

rust::Slice<const std::uint8_t> bytesOf(const QByteArray &data)
{
    return {reinterpret_cast<const std::uint8_t *>(data.constData()), static_cast<std::size_t>(data.size())};
}

// What a request is for, in the Rust core's terms.
ResourceKind kindOf(QWebEngineUrlRequestInfo::ResourceType type)
{
    using Info = QWebEngineUrlRequestInfo;
    switch (type) {
    case Info::ResourceTypeMainFrame:
    case Info::ResourceTypeNavigationPreloadMainFrame:
        return ResourceKind::Page;
    case Info::ResourceTypeSubFrame:
    case Info::ResourceTypeNavigationPreloadSubFrame:
        return ResourceKind::Frame;
    case Info::ResourceTypeStylesheet:
        return ResourceKind::Stylesheet;
    case Info::ResourceTypeScript:
    case Info::ResourceTypeWorker:
    case Info::ResourceTypeSharedWorker:
    case Info::ResourceTypeServiceWorker:
        return ResourceKind::Script;
    case Info::ResourceTypeImage:
    case Info::ResourceTypeFavicon:
        return ResourceKind::Image;
    case Info::ResourceTypeFontResource:
        return ResourceKind::Font;
    case Info::ResourceTypeMedia:
        return ResourceKind::Media;
    case Info::ResourceTypeObject:
    case Info::ResourceTypePluginResource:
        return ResourceKind::Object;
    case Info::ResourceTypeXhr:
        return ResourceKind::Xhr;
    case Info::ResourceTypePing:
        return ResourceKind::Ping;
    case Info::ResourceTypeWebSocket:
        return ResourceKind::WebSocket;
    case Info::ResourceTypeCspReport:
        return ResourceKind::CspReport;
    default: // prefetches, "other" subresources, and kinds newer Qt versions add
        return ResourceKind::Other;
    }
}

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
        const QByteArray firstParty = info.firstPartyUrl().toEncoded();
        if (m_policy->shouldBlock(bytesOf(url), bytesOf(firstParty), kindOf(info.resourceType()))) {
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
                   const QString &startUrl, bool fullScreen, const QString &pageScript)
{
    // One interceptor for every profile (the default one and each private session's), owned by the engine so it
    // outlives them all.
    auto *interceptor = new RequestInterceptor(std::move(policy), &engine);
    auto *host = new Host(interceptor, pageScript, &engine);
    // A Sofa already running takes the address, before this one touches the profile.
    if (host->claimOrHandOver(QGuiApplication::desktopFileName(), startUrl)) {
        return false;
    }

    // Outside Plasma there may be no icon theme set; Breeze (KDE's) fills in where it is installed, in its dark
    // version: outside Plasma Sofa takes Bigscreen's dark colour scheme (KDE's TV components do it).
    if (QIcon::fallbackThemeName().isEmpty() || QIcon::fallbackThemeName() == QStringLiteral("hicolor")) {
        QIcon::setFallbackThemeName(QStringLiteral("breeze-dark"));
    }

    // KDE's TV components, when Sofa was built with its own copy of them (CMake option SOFA_BIGSCREEN_UI): they sit
    // in a folder of Sofa's next to its binary's, which only Sofa looks in.
    const QDir ownModules(QCoreApplication::applicationDirPath() + QStringLiteral("/../lib/sofa-tv-browser/qml"));
    if (ownModules.exists()) {
        engine.addImportPath(ownModules.canonicalPath());
    }

    // Named at construction (Qt 6.9 and newer): a profile renamed or switched to disk afterwards is on its way out.
    auto *profile = new QQuickWebEngineProfile(storageName, &engine);
    profile->setUrlRequestInterceptor(interceptor);
    host->addPageScript(profile);

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

QString chromiumVersion()
{
    return QString::fromLatin1(qWebEngineChromiumVersion());
}

QString securityPatchVersion()
{
    return QString::fromLatin1(qWebEngineChromiumSecurityPatchVersion());
}

QString qtVersion()
{
    return QString::fromLatin1(qVersion());
}

QString platformName()
{
    return QGuiApplication::platformName();
}

} // namespace sofa
