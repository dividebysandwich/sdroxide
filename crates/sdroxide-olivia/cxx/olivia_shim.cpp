// C face of Pawel Jalocha's MFSK receiver. See pj_mfsk.h for the engine and
// README.md for its provenance and licence.

#include "olivia_shim.h"

#include <cstddef>

#include "pj_mfsk.h"

extern "C" void *olivia_new(int tones, int bandwidth, double rate, double centre,
                            int reverse) {
    MFSK_Receiver<float> *rx = new MFSK_Receiver<float>();
    rx->Tones = (size_t)tones;
    rx->Bandwidth = (size_t)bandwidth;
    // fldigi's and SDR-J's defaults: an eight-spacing search either side of the
    // carrier, integrated over four FEC blocks.
    rx->SyncMargin = 8;
    rx->SyncIntegLen = 4;
    // No squelch: every character the engine decodes is handed over, and the
    // caller decides what to do with a weak one.
    rx->SyncThreshold = 0.0f;
    rx->SampleRate = 8000.0f;
    rx->InputSampleRate = (float)rate;
    rx->Reverse = reverse != 0;

    // The bank is centred on `centre`, so the first carrier sits half the span
    // below it — the same arithmetic as `olivia::restart()` in fldigi.
    double fc_offset = bandwidth * (1.0 - 0.5 / tones) / 2.0;
    rx->FirstCarrierMultiplier =
        (float)((reverse ? centre + fc_offset : centre - fc_offset) / 500.0);

    if (rx->Preset() < 0) {
        delete rx;
        return NULL;
    }
    return rx;
}

extern "C" int olivia_process(void *handle, const float *audio, int n, char *out, int outlen,
                              double *snr) {
    if (handle == NULL) return 0;
    MFSK_Receiver<float> *rx = (MFSK_Receiver<float> *)handle;

    int written = 0;
    int offset = 0;
    // The reference drivers feed 512 samples at a time; the engine accepts any
    // length, but this keeps the timing identical to the one that works.
    while (offset < n) {
        int chunk = n - offset;
        if (chunk > 512) chunk = 512;
        rx->Process(const_cast<float *>(audio) + offset, (size_t)chunk);
        offset += chunk;

        unsigned char c;
        while (rx->GetChar(c) > 0) {
            if (written < outlen - 1) out[written++] = (char)c;
        }
    }

    if (snr != NULL) *snr = (double)rx->SignalToNoiseRatio();
    return written;
}

extern "C" void olivia_free(void *handle) {
    if (handle == NULL) return;
    delete (MFSK_Receiver<float> *)handle;
}
