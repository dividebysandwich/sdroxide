// sdroxide's C face on Data2G's native core; see data2g_shim.h.

#include "data2g_shim.h"

#include <cstdio>
#include <cstring>
#include <exception>
#include <memory>
#include <optional>
#include <span>
#include <string>

#include "arq/engine.hpp"
#include "arq/link.hpp"
#include "config/config.hpp"
#include "host/host.hpp"
#include "kisslink/kisslink.hpp"

using namespace data2g;

struct sdrx_d2g {
    // The KISS link is never served on a port here; the host's MODES and BCAST
    // need one to answer at all.
    std::unique_ptr<kisslink::KissLink> kiss;
    std::unique_ptr<arq::Engine> engine;
    std::unique_ptr<host::Host> host;
    std::string state;
};

namespace {

void put_err(char* err, size_t errlen, const char* what) {
    if (err && errlen) std::snprintf(err, errlen, "%s", what);
}

void log_failure(const char* where, const char* what) {
    arq::log_write("sdroxide.data2g", 40, std::string(where) + ": " + what);
}

}  // namespace

extern "C" {

void sdrx_d2g_set_log(sdrx_d2g_log_fn f, int min_level) {
    try {
        arq::LogSink sink;
        sink.enabled = [min_level](const char*, int level) { return level >= min_level; };
        sink.write = [f](const char* logger, int level, const std::string& msg) {
            if (f) f(logger, level, msg.c_str());
        };
        arq::set_log_sink(std::move(sink));
    } catch (...) {
    }
}

sdrx_d2g* sdrx_d2g_new(const char* mycall, int64_t seed, double ptt_delay_s, char* err, size_t errlen) {
    try {
        auto d = std::make_unique<sdrx_d2g>();
        d->kiss = std::make_unique<kisslink::KissLink>(2);
        arq::EngineConfig cfg;
        cfg.ptt_delay_s = ptt_delay_s;
        cfg.kiss = d->kiss.get();
        cfg.worker = false;
        if (seed >= 0) cfg.seed = static_cast<std::uint64_t>(seed);
        d->engine = std::make_unique<arq::Engine>(mycall && *mycall ? mycall : "N0CALL", cfg);
        d->host = std::make_unique<host::Host>(*d->engine);
        // No ACCEPT: the data of a session goes to whoever holds the data port.
        d->host->dial = nullptr;
        return d.release();
    } catch (const std::exception& e) {
        put_err(err, errlen, e.what());
    } catch (...) {
        put_err(err, errlen, "unknown error");
    }
    return nullptr;
}

void sdrx_d2g_free(sdrx_d2g* d) {
    if (!d) return;
    try {
        d->host.reset();
        if (d->engine) d->engine->stop();
        d->engine.reset();
        d->kiss.reset();
    } catch (...) {
    }
    delete d;
}

int sdrx_d2g_step(sdrx_d2g* d, const double* in, double* out, size_t n, int* ptt) {
    if (ptt) *ptt = 0;
    try {
        auto o = d->engine->step(std::span<const double>(in, n));
        const size_t k = o.audio.size() < n ? o.audio.size() : n;
        std::memcpy(out, o.audio.data(), k * sizeof(double));
        for (size_t i = k; i < n; i++) out[i] = 0.0;
        if (ptt) *ptt = o.ptt ? 1 : 0;
        // In sync mode the engine's after_block has run; this is the host's.
        d->host->after_step(o.ptt);
        return 0;
    } catch (const std::exception& e) {
        log_failure("step", e.what());
    } catch (...) {
        log_failure("step", "unknown error");
    }
    for (size_t i = 0; i < n; i++) out[i] = 0.0;
    return -1;
}

int sdrx_d2g_command(sdrx_d2g* d, const char* line) {
    try {
        d->host->command(line ? line : "");
        return 0;
    } catch (const std::exception& e) {
        log_failure("command", e.what());
    } catch (...) {
        log_failure("command", "unknown error");
    }
    d->host->out_cmd.push_back("WRONG");
    return -1;
}

int sdrx_d2g_data_in(sdrx_d2g* d, const uint8_t* data, size_t len) {
    try {
        d->host->data_in(arq::ByteView(data, len));
        return 0;
    } catch (const std::exception& e) {
        log_failure("data", e.what());
    } catch (...) {
        log_failure("data", "unknown error");
    }
    return -1;
}

void sdrx_d2g_client_gone(sdrx_d2g* d) {
    try {
        d->host->client_gone();
    } catch (...) {
        log_failure("client_gone", "threw");
    }
}

int sdrx_d2g_abort(sdrx_d2g* d) {
    try {
        return d->host->abort_session() ? 1 : 0;
    } catch (...) {
        log_failure("abort", "threw");
    }
    return 0;
}

void sdrx_d2g_take_output(sdrx_d2g* d, sdrx_d2g_line_fn line, sdrx_d2g_data_fn data, void* user) {
    try {
        auto& h = *d->host;
        for (const auto& l : h.out_cmd)
            if (line) line(user, l.data(), l.size());
        h.out_cmd.clear();
        if (!h.out_data.empty() && data) data(user, h.out_data.data(), h.out_data.size());
        h.out_data.clear();
    } catch (...) {
        log_failure("take_output", "threw");
    }
}

int sdrx_d2g_busy(const sdrx_d2g* d) {
    return d->engine->busy() || d->engine->channel_busy() ? 1 : 0;
}

const char* sdrx_d2g_state(const sdrx_d2g* d) {
    try {
        auto* m = const_cast<sdrx_d2g*>(d);
        m->state = arq::state_name(m->engine->session().state);
        return m->state.c_str();
    } catch (...) {
    }
    return "";
}

const char* sdrx_d2g_version(void) { return DATA2G_VERSION; }

int sdrx_d2g_protocol(void) { return config::PROTOCOL_VERSION; }

}  // extern "C"
