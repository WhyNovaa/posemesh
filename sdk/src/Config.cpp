#include <iostream>
#include <Posemesh/Config.hpp>

#if defined(__APPLE__)
#    include "../platform/Apple/src/Util.hpp"
#endif

namespace psm {

Config::Config() {
    #if !defined(__EMSCRIPTEN__)
        m_serveAsBootstrap = false;
        m_serveAsRelay = false;
    #endif
}

Config::Config(const Config& config) = default;

Config::Config(Config&& config) noexcept = default;

Config::~Config() = default;

Config& Config::operator=(const Config& config) = default;

Config& Config::operator=(Config&& config) noexcept = default;

bool Config::operator==(const Config& config) const noexcept {
    if (this == &config)
        return true;
    #if !defined(__EMSCRIPTEN__)
        if(m_serveAsBootstrap != config.m_serveAsBootstrap)
            return false;
        if(m_serveAsRelay != config.m_serveAsRelay)
            return false;
    #endif
    return m_bootstraps == config.m_bootstraps;
}

bool Config::operator!=(const Config& config) const noexcept {
    return !(*this == config);
}

#if !defined(__EMSCRIPTEN__)
    bool Config::getServeAsBootstrap() const noexcept {
        return m_serveAsBootstrap;
    }

    void Config::setServeAsBootstrap(bool serveAsBootstrap) noexcept {
        m_serveAsBootstrap = serveAsBootstrap;
    }

    bool Config::getServeAsRelay() const noexcept {
        return m_serveAsRelay;
    }

    void Config::setServeAsRelay(bool serveAsRelay) noexcept {
        m_serveAsRelay = serveAsRelay;
    }
#endif

std::vector<std::string> Config::getBootstraps() const {
    return m_bootstraps;
}

bool Config::setBootstraps(std::vector<std::string> bootstraps) noexcept {
    const auto bootstrapsCount = bootstraps.size();
    for (std::size_t i = 0; i < bootstrapsCount; ++i) {
        if (bootstraps[i].find(';') != std::string::npos) {
            std::cerr << "Config::setBootstraps(): bootstrap at index " << i << " contains an illegal ';' character" << std::endl;
            return false;
        }
    }
    for (std::size_t i = 0; i < bootstrapsCount - 1; ++i) {
        for (std::size_t j = i + 1; j < bootstrapsCount; ++j) {
            if (bootstraps[i] == bootstraps[j]) {
                std::cerr << "Config::setBootstraps(): bootstrap at index " << j << " is the same as bootstrap at index " << i << std::endl;
                return false;
            }
        }
    }
    m_bootstraps = std::move(bootstraps);
    return true;
}

std::vector<std::string> Config::getRelays() const {
    return m_relays;
}

bool Config::setRelays(std::vector<std::string> relays) noexcept {
    const auto relaysCount = relays.size();
    for (std::size_t i = 0; i < relaysCount; ++i) {
        if (relays[i].find(';') != std::string::npos) {
            std::cerr << "Config::setRelays(): relay at index " << i << " contains an illegal ';' character" << std::endl;
            return false;
        }
    }
    for (std::size_t i = 0; i < relaysCount - 1; ++i) {
        for (std::size_t j = i + 1; j < relaysCount; ++j) {
            if (relays[i] == relays[j]) {
                std::cerr << "Config::setRelays(): relay at index " << j << " is the same as relay at index " << i << std::endl;
                return false;
            }
        }
    }
    m_relays = std::move(relays);
    return true;
}

std::vector<std::uint8_t> Config::getPrivateKey() const {
    return m_privateKey;
}

void Config::setPrivateKey(std::vector<std::uint8_t> privateKey) noexcept {
    m_privateKey = std::move(privateKey);
}

#if !defined(__EMSCRIPTEN__)
    std::string Config::getPrivateKeyPath() const {
        return m_privateKeyPath;
    }

    void Config::setPrivateKeyPath(std::string privateKeyPath) noexcept {
        m_privateKeyPath = std::move(privateKeyPath);
    }
#endif

Config Config::createDefault() {
    Config config;
    // TODO: set config.m_bootstraps to well-known bootstraps
    // TODO: set config.m_relays to well-known relays
    #if defined(__APPLE__)
        config.m_privateKeyPath = util::getAppSupportDirectoryPath();
        if (!config.m_privateKeyPath.empty()) {
            if (config.m_privateKeyPath.back() != '/') {
                config.m_privateKeyPath += "/";
            }
            config.m_privateKeyPath += "posemesh_private_key.dat";
        }
    #endif
    return config;
}

}
