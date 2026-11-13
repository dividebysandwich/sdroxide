/* sdroxide's C face on Data2G's native core (vendor/data2g/native/core).
 *
 * One station: an arq::Engine in sync mode (step() returns this block's
 * output, deterministically) behind a host::Host, which answers the TNC
 * command language. No sockets, no sound card, no threads of the shim's own:
 * the caller clocks it with 8 kHz audio and carries the host's lines.
 *
 * Every entry point catches what the C++ throws: nothing unwinds into Rust.
 * Calls on one station must not overlap. */
#ifndef SDROXIDE_DATA2G_SHIM_H
#define SDROXIDE_DATA2G_SHIM_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct sdrx_d2g sdrx_d2g;

/* Log lines from the core: logger name, level (Python's: 20 info, 30 warning,
 * 40 error), message. Set once, before the first station. */
typedef void (*sdrx_d2g_log_fn)(const char *logger, int level, const char *msg);
void sdrx_d2g_set_log(sdrx_d2g_log_fn f, int min_level);

/* A station for `mycall`. seed < 0: random. NULL on failure, with the reason
 * in err. */
sdrx_d2g *sdrx_d2g_new(const char *mycall, int64_t seed, double ptt_delay_s, char *err, size_t errlen);
void sdrx_d2g_free(sdrx_d2g *d);

/* n samples in, n out (peak 1.0). *ptt: the engine wants the transmitter. 0 on
 * success, -1 when the step threw (the station plays silence). */
int sdrx_d2g_step(sdrx_d2g *d, const double *in, double *out, size_t n, int *ptt);

/* The host's side: one command line without the CR, and payload. */
int sdrx_d2g_command(sdrx_d2g *d, const char *line);
int sdrx_d2g_data_in(sdrx_d2g *d, const uint8_t *data, size_t len);
void sdrx_d2g_client_gone(sdrx_d2g *d);
/* The operator's abort: 1 when a session was dropped. */
int sdrx_d2g_abort(sdrx_d2g *d);

/* What goes back to the host since the last call: each line (no CR), then
 * the payload in one piece. */
typedef void (*sdrx_d2g_line_fn)(void *user, const char *line, size_t len);
typedef void (*sdrx_d2g_data_fn)(void *user, const uint8_t *data, size_t len);
void sdrx_d2g_take_output(sdrx_d2g *d, sdrx_d2g_line_fn line, sdrx_d2g_data_fn data, void *user);

/* A burst on the channel (the receiver's busy), and the host's BUSY. */
int sdrx_d2g_busy(const sdrx_d2g *d);
/* The session's state, as session.py names it ("IDLE", "CONNECTED", ...). */
const char *sdrx_d2g_state(const sdrx_d2g *d);
/* "0.4.3" */
const char *sdrx_d2g_version(void);
/* config::PROTOCOL_VERSION */
int sdrx_d2g_protocol(void);

#ifdef __cplusplus
}
#endif

#endif
