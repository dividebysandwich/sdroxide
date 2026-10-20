//! C interface to Pawel Jalocha's MFSK receiver.
//!
//! `pj_mfsk.h` is a C++ template library, so the wrapper is C++ and this is its
//! plain-C face. Nothing here does any signal processing of its own: it sets the
//! receiver's parameters the way fldigi and SDR-J do, feeds it audio in the same
//! 512-sample batches, and drains the character queue.

#ifndef SDROXIDE_OLIVIA_SHIM_H
#define SDROXIDE_OLIVIA_SHIM_H

#ifdef __cplusplus
extern "C" {
#endif

/// `tones` is the tone count (2, 4, ... 256), `bandwidth` the occupied band in
/// Hz (125, 250, 500, 1000, 2000), `rate` the audio sample rate and `centre`
/// the audio frequency the bank is centred on. Returns NULL if the engine
/// cannot allocate.
void *olivia_new(int tones, int bandwidth, double rate, double centre, int reverse);

/// Decode `n` samples of audio. Returns the number of characters written to
/// `out` (never more than `outlen`), and the engine's signal-to-noise estimate
/// in `snr` when it is not NULL.
int olivia_process(void *handle, const float *audio, int n, char *out, int outlen, double *snr);

void olivia_free(void *handle);

#ifdef __cplusplus
}
#endif

#endif
